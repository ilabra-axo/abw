//! Platform tools and their registry (target-system-design §4.3, §12.1; ADR-0008 §7).
//!
//! The registry holds what can run; an episode's allowlist (`capabilities.tools_allowed`,
//! stamped on `episode.opened`) says what may. The executor offers the model only their
//! intersection and refuses, records and gates any other call (§4.3 "Enforcement").

use crate::BoxFuture;
use crate::llm::ToolSpec;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;

/// A tool's error, recorded as the call's output (`error_class` on `tool.invoked`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolError {
    pub class: String,
    pub message: String,
}

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    /// Recorded on every `tool.invoked` (§12.6 "Tools").
    fn version(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> Value;
    fn call<'a>(&'a self, input: &'a Value) -> BoxFuture<'a, Result<Value, ToolError>>;
}

/// The tools this process can run, by name.
#[derive(Default, Clone)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.tools.keys()).finish()
    }
}

impl ToolRegistry {
    pub fn new() -> ToolRegistry {
        ToolRegistry::default()
    }

    /// Registers `tool` under its name, replacing any tool of that name.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    /// Specs of the registered tools on `allowed`, sorted by name: what the model is offered.
    pub fn offered(&self, allowed: &[String]) -> Vec<ToolSpec> {
        self.tools
            .values()
            .filter(|t| allowed.iter().any(|a| a == t.name()))
            .map(|t| ToolSpec {
                name: t.name().to_string(),
                description: t.description().to_string(),
                input_schema: t.input_schema(),
            })
            .collect()
    }
}
