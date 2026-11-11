//! The episode pipeline (target-system-design §4.4-§4.7; economy.md §5 "Run an episode";
//! ADR-0008 §7, §10).
//!
//! One run is three kinds of transaction, each a single `Ledger::dispatch`:
//! - TX-A `episode::Open` places the hold before any provider call (ECO-5);
//! - one TX-B `episode::RecordTool` per tool call, as it happens (§4.5 ⟨12⟩);
//! - TX-C `episode::Close` settles at most the hold (economy.md §5 step 3).
//!
//! No transaction is open while the executor awaits the provider or a tool (SUB-06): it
//! holds no connection, only the ledger's dispatch. Recovery follows ADR-0008 §7: a failed
//! TX-B is abandoned at once (§4.5 ⟨6⟩); a failed TX-C is left to the reaper (economy.md §7).

use crate::budget::{MAX_ITERATIONS, estimate_hold};
use crate::llm::{LlmProvider, LlmRequest, Message, ToolCall};
use crate::tools::ToolRegistry;
use abw_canon::Sha256Ref;
#[cfg(feature = "fault-injection")]
use abw_ledger::FaultPoint;
use abw_ledger::episode::{
    self, Abandon, CloseOutcome, EpisodeStatus, OpenEpisode, ReasonClass, RecordTool, ToolOutcome,
    ToolRecord, Usage,
};
use abw_ledger::policy::{self, PolicyKind, PricePolicy};
use abw_ledger::{CommandCtx, DispatchError, Expected, Ledger, ReadError, Rejection};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// One run to execute.
#[derive(Debug, Clone, PartialEq)]
pub struct RunRequest {
    pub episode_id: Uuid,
    pub owner_id: String,
    pub agent_id: String,
    pub task: String,
    pub max_tokens: u32,
    pub at: DateTime<Utc>,
}

/// A run whose TX-C committed, whatever its status.
#[derive(Debug, Clone, PartialEq)]
pub struct RunOutcome {
    pub episode_id: Uuid,
    pub status: EpisodeStatus,
    pub reason_class: Option<ReasonClass>,
    pub llm_calls: u32,
    pub tool_calls: u32,
    pub held_cu: u64,
    pub consumed_cu: u64,
    pub output: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    /// The run was refused before any work (TX-A or policy resolution).
    #[error("refused: {0}")]
    Rejected(Rejection),
    /// A dispatch failed after the episode opened, or TX-A failed for a non-refusal reason.
    #[error("ledger: {0}")]
    Ledger(DispatchError),
    #[error("read: {0}")]
    Read(ReadError),
    /// A simulated process death (tests only): nothing was closed or abandoned.
    #[cfg(feature = "fault-injection")]
    #[error("crashed at {0:?}")]
    Crashed(PipelineFault),
}

/// Where a test makes the pipeline fail (ADR-0008 §7 item 8, §10). `call` counts the run's
/// tool calls from 0.
#[cfg(feature = "fault-injection")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineFault {
    TxA(FaultPoint),
    TxB { call: u32, point: FaultPoint },
    TxC(FaultPoint),
    CrashAfterTxA,
    CrashAfterTxB { call: u32 },
    CrashBeforeTxC,
}

/// Runs episodes for one provider and tool registry.
pub struct Executor {
    ledger: Arc<Ledger>,
    provider: Arc<dyn LlmProvider>,
    tools: ToolRegistry,
    #[cfg(feature = "fault-injection")]
    fault: Option<PipelineFault>,
}

/// Which dispatch of the run is about to happen, for fault injection.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(not(feature = "fault-injection"), allow(dead_code))]
enum Stage {
    TxA,
    TxB(u32),
    TxC,
}

/// A point at which a test may simulate the process dying.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(not(feature = "fault-injection"), allow(dead_code))]
enum Crash {
    AfterTxA,
    AfterTxB(u32),
    BeforeTxC,
}

/// How the loop ended, before TX-C.
enum End {
    Success(String),
    Partial(ReasonClass),
    ProviderError,
}

/// The error class of a call allowed by the card but not registered in this process.
const TOOL_UNAVAILABLE: &str = "tool_unavailable";

impl Executor {
    pub fn new(
        ledger: Arc<Ledger>,
        provider: Arc<dyn LlmProvider>,
        tools: ToolRegistry,
    ) -> Executor {
        Executor {
            ledger,
            provider,
            tools,
            #[cfg(feature = "fault-injection")]
            fault: None,
        }
    }

    /// Makes runs fail at `fault` (tests only).
    #[cfg(feature = "fault-injection")]
    pub fn with_fault(mut self, fault: PipelineFault) -> Executor {
        self.fault = Some(fault);
        self
    }

    /// The ledger for `stage`: a fault-injecting one if the test asked for it there.
    #[cfg(feature = "fault-injection")]
    fn injected(&self, stage: Stage) -> Option<Ledger> {
        let point = match (self.fault?, stage) {
            (PipelineFault::TxA(p), Stage::TxA) | (PipelineFault::TxC(p), Stage::TxC) => p,
            (PipelineFault::TxB { call, point }, Stage::TxB(n)) if call == n => point,
            _ => return None,
        };
        Some(self.ledger.inject_fault(point))
    }

    #[cfg(not(feature = "fault-injection"))]
    fn injected(&self, _stage: Stage) -> Option<Ledger> {
        None
    }

    #[cfg(feature = "fault-injection")]
    fn crash(&self, at: Crash) -> Result<(), RunError> {
        let hit = match (self.fault, at) {
            (Some(PipelineFault::CrashAfterTxA), Crash::AfterTxA)
            | (Some(PipelineFault::CrashBeforeTxC), Crash::BeforeTxC) => true,
            (Some(PipelineFault::CrashAfterTxB { call }), Crash::AfterTxB(n)) => call == n,
            _ => false,
        };
        match self.fault {
            Some(f) if hit => Err(RunError::Crashed(f)),
            _ => Ok(()),
        }
    }

    #[cfg(not(feature = "fault-injection"))]
    fn crash(&self, _at: Crash) -> Result<(), RunError> {
        Ok(())
    }

    /// Runs one episode (ADR-0008 §7 "Run semantics").
    pub async fn run(&self, req: RunRequest, ctx: &CommandCtx) -> Result<RunOutcome, RunError> {
        // 1. Policies and the hold, all before TX-A.
        let read = self.ledger.reader();
        let grant = policy::active(&read, PolicyKind::Grant, &ctx.tenant, req.at)
            .await
            .map_err(RunError::Read)?
            .ok_or_else(|| no_policy(PolicyKind::Grant, &ctx.tenant))?;
        let price_ref = policy::active(&read, PolicyKind::Price, &ctx.tenant, req.at)
            .await
            .map_err(RunError::Read)?
            .ok_or_else(|| no_policy(PolicyKind::Price, &ctx.tenant))?;
        let price = policy::load_price(&read, &price_ref)
            .await
            .map_err(RunError::Read)?
            .ok_or_else(|| {
                RunError::Rejected(Rejection::PolicyNotFound {
                    policy: price_ref.to_string(),
                })
            })?;
        let hold_cu = estimate_hold(&price, req.max_tokens);

        // 2. TX-A. No provider call happens before it commits.
        let open = episode::Open::new(OpenEpisode {
            episode_id: req.episode_id,
            owner_id: req.owner_id.clone(),
            agent_id: req.agent_id.clone(),
            input: json!({"task": req.task}),
            hold_cu,
            price_policy: price_ref,
            grant_policy: grant,
            at: req.at,
        });
        let faulty = self.injected(Stage::TxA);
        let opened = faulty
            .as_ref()
            .unwrap_or(&self.ledger)
            .dispatch(open, ctx)
            .await
            .map_err(|e| match e {
                DispatchError::Rejected(r) => RunError::Rejected(r),
                other => RunError::Ledger(other),
            })?
            .output;
        self.crash(Crash::AfterTxA)?;

        let mut run = Run {
            exec: self,
            ctx,
            episode_id: req.episode_id,
            head: opened.seq,
            price,
            hold_cu,
            usage: Usage::default(),
            llm_calls: 0,
            tool_calls: 0,
        };
        let card = match load_card(&read, &opened.card_sha).await {
            Ok(c) => c,
            Err(e) => return Err(run.abandon(e).await),
        };
        let end = run.loop_turns(&card, &req).await?;
        self.crash(Crash::BeforeTxC)?;
        run.close(end).await
    }
}

fn no_policy(kind: PolicyKind, tenant: &str) -> RunError {
    RunError::Rejected(Rejection::PolicyNotFound {
        policy: format!("no active {} policy for tenant {tenant}", kind.as_str()),
    })
}

/// The parts of the cited card version the loop needs.
struct Card {
    system: String,
    model: String,
    tools_allowed: Vec<String>,
}

/// Reads the card artifact the episode cites (§4.5 ⟨2⟩: from the version, not from
/// mutable columns).
async fn load_card(read: &abw_ledger::ReadDb, sha: &Sha256Ref) -> Result<Card, RunError> {
    let missing = |why: String| {
        RunError::Read(ReadError::Artifact {
            sha: sha.to_string(),
            reason: why,
        })
    };
    let bytes = read
        .resolve(sha)
        .await
        .map_err(RunError::Read)?
        .ok_or_else(|| missing("the card artifact is not stored".into()))?;
    let text = String::from_utf8(bytes).map_err(|e| missing(e.to_string()))?;
    let card = abw_canon::parse(&text).map_err(|e| missing(e.to_string()))?;
    let caps = card.get("capabilities");
    Ok(Card {
        system: card
            .get("system_prompt")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        model: caps
            .and_then(|c| c.get("model"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        tools_allowed: caps
            .and_then(|c| c.get("tools_allowed"))
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// One run in progress, after TX-A.
struct Run<'e> {
    exec: &'e Executor,
    ctx: &'e CommandCtx,
    episode_id: Uuid,
    /// The episode stream head, the expected version of the next append.
    head: u64,
    price: PricePolicy,
    hold_cu: u64,
    usage: Usage,
    llm_calls: u32,
    tool_calls: u32,
}

impl Run<'_> {
    /// Metering (ADR-0008 §7 item 4): the run stops once the priced usage reaches the hold.
    fn at_ceiling(&self) -> bool {
        self.price.price(&self.usage) >= self.hold_cu
    }

    /// The bounded loop: `MAX_ITERATIONS` calls offering tools, then one flush call
    /// offering none (§4.1, §4.4).
    async fn loop_turns(&mut self, card: &Card, req: &RunRequest) -> Result<End, RunError> {
        let offered = self.exec.tools.offered(&card.tools_allowed);
        let mut messages = vec![Message::User(req.task.clone())];
        for iteration in 0..=MAX_ITERATIONS {
            let flush = iteration == MAX_ITERATIONS;
            let request = LlmRequest {
                model: card.model.clone(),
                system: card.system.clone(),
                messages: messages.clone(),
                tools: if flush { Vec::new() } else { offered.clone() },
                max_tokens: req.max_tokens,
            };
            self.llm_calls += 1;
            let turn = match self.exec.provider.complete(&request).await {
                Ok(t) => t,
                // Item 5: a provider error closes Failure, charging what was metered.
                Err(_) => return Ok(End::ProviderError),
            };
            self.usage.input_tokens = self
                .usage
                .input_tokens
                .saturating_add(turn.usage.input_tokens);
            self.usage.output_tokens = self
                .usage
                .output_tokens
                .saturating_add(turn.usage.output_tokens);
            if self.at_ceiling() {
                return Ok(End::Partial(ReasonClass::BudgetExhausted));
            }
            if turn.tool_calls.is_empty() {
                return Ok(End::Success(turn.text));
            }
            if flush {
                // ADR-0008 ambiguity 5: flush-turn tool calls are neither run nor recorded.
                return Ok(End::Partial(ReasonClass::IterationsExhausted));
            }
            messages.push(Message::Assistant {
                text: turn.text.clone(),
                tool_calls: turn.tool_calls.clone(),
            });
            for (call_index, call) in turn.tool_calls.iter().enumerate() {
                let call_index = u32::try_from(call_index).unwrap_or(u32::MAX);
                let result = self
                    .tool_call(iteration, call_index, call, &card.tools_allowed)
                    .await?;
                messages.push(result);
                if self.at_ceiling() {
                    return Ok(End::Partial(ReasonClass::BudgetExhausted));
                }
            }
        }
        Ok(End::Partial(ReasonClass::IterationsExhausted))
    }

    /// Runs (or refuses) one tool call and records it in its own TX-B (§4.3, §4.5 ⟨12⟩).
    async fn tool_call(
        &mut self,
        iteration: u32,
        call_index: u32,
        call: &ToolCall,
        allowed: &[String],
    ) -> Result<Message, RunError> {
        let on_list = allowed.contains(&call.name);
        let started = Instant::now();
        let (record_allowed, version, outcome) = if !on_list {
            // Not offered, so not run: a refusal with its gate decision.
            (
                false,
                String::new(),
                ToolOutcome::Err {
                    class: episode::ALLOWLIST_ERROR_CLASS.into(),
                    message: format!("{} is not on this agent's allowlist", call.name),
                },
            )
        } else {
            match self.exec.tools.get(&call.name) {
                Some(tool) => {
                    self.usage.tool_calls = self.usage.tool_calls.saturating_add(1);
                    let out = match tool.call(&call.input).await {
                        Ok(v) => ToolOutcome::Ok(v),
                        Err(e) => ToolOutcome::Err {
                            class: e.class,
                            message: e.message,
                        },
                    };
                    (true, tool.version().to_string(), out)
                }
                // Allowed by the card but not registered here [I]: recorded as allowed,
                // with an error, since the allowlist didn't refuse it.
                None => (
                    true,
                    String::new(),
                    ToolOutcome::Err {
                        class: TOOL_UNAVAILABLE.into(),
                        message: format!("{} is not available in this process", call.name),
                    },
                ),
            }
        };
        let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let message = match &outcome {
            ToolOutcome::Ok(v) => Message::ToolResult {
                call_id: call.id.clone(),
                content: v.to_string(),
                is_error: false,
            },
            ToolOutcome::Err { class, message } => Message::ToolResult {
                call_id: call.id.clone(),
                content: format!("{class}: {message}"),
                is_error: true,
            },
        };
        let record = ToolRecord {
            iteration,
            call_index,
            tool_name: call.name.clone(),
            tool_version: version,
            allowed: record_allowed,
            input: call.input.clone(),
            output: outcome,
            duration_ms,
        };
        let n = self.tool_calls;
        let faulty = self.exec.injected(Stage::TxB(n));
        let cmd = RecordTool::new(self.episode_id, Expected::Exact(self.head), record);
        match faulty
            .as_ref()
            .unwrap_or(&self.exec.ledger)
            .dispatch(cmd, self.ctx)
            .await
        {
            Ok(d) => self.head = d.output.seq,
            // Item 7: without its evidence the run can't go on.
            Err(e) => return Err(self.abandon(RunError::Ledger(e)).await),
        }
        self.tool_calls += 1;
        self.exec.crash(Crash::AfterTxB(n))?;
        Ok(message)
    }

    /// Abandons the episode with `LedgerError` and returns `cause` (§4.5 ⟨6⟩). If the
    /// abandon itself fails, the episode stays running for the reaper.
    async fn abandon(&self, cause: RunError) -> RunError {
        let cmd = Abandon::new(
            self.episode_id,
            Expected::Exact(self.head),
            ReasonClass::LedgerError,
        );
        match self.exec.ledger.dispatch(cmd, self.ctx).await {
            Ok(_) => cause,
            // The caller needs the original cause either way: an episode the abandon
            // couldn't close is closed by the reaper (§4.7).
            Err(_still_running) => cause,
        }
    }

    /// TX-C (item 6). A failure here is returned as is: the hold stays open for the reaper.
    async fn close(self, end: End) -> Result<RunOutcome, RunError> {
        let (status, reason_class, output) = match end {
            End::Success(text) => (EpisodeStatus::Success, None, Some(text)),
            End::Partial(r) => (EpisodeStatus::Partial, Some(r), None),
            End::ProviderError => (
                EpisodeStatus::Failure,
                Some(ReasonClass::ProviderError),
                None,
            ),
        };
        let cmd = episode::Close::new(
            self.episode_id,
            Expected::Exact(self.head),
            CloseOutcome {
                status,
                reason_class,
                raw_output: output.clone(),
                usage: self.usage,
            },
        );
        let faulty = self.exec.injected(Stage::TxC);
        let closed = faulty
            .as_ref()
            .unwrap_or(&self.exec.ledger)
            .dispatch(cmd, self.ctx)
            .await
            .map_err(RunError::Ledger)?
            .output;
        Ok(RunOutcome {
            episode_id: self.episode_id,
            status: closed.status,
            reason_class,
            llm_calls: self.llm_calls,
            tool_calls: self.tool_calls,
            held_cu: self.hold_cu,
            consumed_cu: closed.consumed_cu,
            output,
        })
    }
}
