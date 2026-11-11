//! The LLM provider interface (target-system-design §4.4; ADR-0008 §7).
//!
//! Providers are reached only between ledger dispatches. Tests never call a real model:
//! [`ScriptedProvider`] replays a fixed sequence of turns and errors (ADR-0008 §1 item 8).

use crate::BoxFuture;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard};

/// A tool offered to the model.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// A tool call the model asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: Value,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// One model response.
#[derive(Debug, Clone, PartialEq)]
pub struct Turn {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    User(String),
    Assistant {
        text: String,
        tool_calls: Vec<ToolCall>,
    },
    ToolResult {
        call_id: String,
        content: String,
        is_error: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmRequest {
    pub model: String,
    pub system: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSpec>,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LlmError {
    #[error("transport: {0}")]
    Transport(String),
    #[error("provider returned {status}: {message}")]
    Provider { status: u16, message: String },
    #[error("rate limited")]
    RateLimited,
    #[error("the script has no more turns")]
    ScriptExhausted,
}

/// A model provider. Implementations hold no ledger handle.
pub trait LlmProvider: Send + Sync {
    fn complete<'a>(&'a self, req: &'a LlmRequest) -> BoxFuture<'a, Result<Turn, LlmError>>;
}

/// One step of a script.
#[derive(Debug, Clone, PartialEq)]
pub enum Scripted {
    Turn(Turn),
    Error(LlmError),
}

#[derive(Debug, Default)]
struct Script {
    remaining: VecDeque<Scripted>,
    requests: Vec<LlmRequest>,
}

/// A deterministic provider: each call returns the next scripted item and records the
/// request; past the end it returns [`LlmError::ScriptExhausted`].
#[derive(Debug, Default)]
pub struct ScriptedProvider {
    script: Mutex<Script>,
}

impl ScriptedProvider {
    pub fn new(script: Vec<Scripted>) -> ScriptedProvider {
        ScriptedProvider {
            script: Mutex::new(Script {
                remaining: script.into(),
                requests: Vec::new(),
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Script> {
        // The guarded state is always consistent between statements, so a poisoned lock
        // (a panic elsewhere while holding it) is safe to keep using.
        match self.script.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }

    /// How many times `complete` was called.
    pub fn calls(&self) -> usize {
        self.lock().requests.len()
    }

    /// Scripted items not yet returned.
    pub fn remaining(&self) -> usize {
        self.lock().remaining.len()
    }

    /// Every request received, in order.
    pub fn requests(&self) -> Vec<LlmRequest> {
        self.lock().requests.clone()
    }
}

impl LlmProvider for ScriptedProvider {
    fn complete<'a>(&'a self, req: &'a LlmRequest) -> BoxFuture<'a, Result<Turn, LlmError>> {
        let next = {
            let mut s = self.lock();
            s.requests.push(req.clone());
            s.remaining.pop_front()
        };
        Box::pin(async move {
            match next {
                Some(Scripted::Turn(t)) => Ok(t),
                Some(Scripted::Error(e)) => Err(e),
                None => Err(LlmError::ScriptExhausted),
            }
        })
    }
}
