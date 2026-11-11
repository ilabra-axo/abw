//! Resolving a card into its version identity (target-system-design §4.2, §4.3; ADR-0008 §6).
//!
//! The rules, applied in this order and nothing else:
//! 1. the raw card is a JSON object with a string `agent_id` in the stream-id grammar;
//! 2. every [`EXCLUDED_FIELDS`] key is removed at the top level;
//! 3. `"$excluded"` is set to [`EXCLUDED_FIELDS`], so changing the list changes every
//!    `card_sha` visibly (§4.2);
//! 4. an absent or null `system_prompt` becomes `""`;
//! 5. an absent `capabilities.tools_allowed` is backfilled from the names of
//!    `capabilities.mcp_tools`, in order, without duplicates (§4.3 one-time backfill); an
//!    absent `capabilities` becomes `{"tools_allowed": []}`.
//!
//! `card_sha = sha256_jcs(json)`, the same value `abw_ledger::agent::PutVersion` computes.

use abw_canon::{JcsError, Sha256Ref, sha256_jcs, sha256_raw};
use serde_json::{Map, Value, json};

/// Fields that change at run time without a configuration change (§4.2).
pub const EXCLUDED_FIELDS: &[&str] = &[
    "performance",
    "usage",
    "ontology_stats",
    "wallet",
    "declared_prompt_sha256",
];

/// A resolved card and its identity.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCard {
    agent_id: String,
    json: Value,
    card_sha: Sha256Ref,
    prompt_sha: Sha256Ref,
}

impl ResolvedCard {
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    pub fn json(&self) -> &Value {
        &self.json
    }

    pub fn card_sha(&self) -> Sha256Ref {
        self.card_sha.clone()
    }

    pub fn prompt_sha(&self) -> Sha256Ref {
        self.prompt_sha.clone()
    }

    /// `capabilities.tools_allowed` (§4.3).
    pub fn tools_allowed(&self) -> Vec<String> {
        self.json
            .get("capabilities")
            .and_then(|c| c.get("tools_allowed"))
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CardError {
    #[error("a card is a JSON object")]
    NotAnObject,
    #[error("the card has no string agent_id")]
    MissingAgentId,
    #[error("agent_id {0:?} is not 1 to 128 of [A-Za-z0-9._@-]")]
    InvalidAgentId(String),
    /// After rule 4, `system_prompt` must be a string to have a `prompt_sha`.
    #[error("system_prompt is not a string")]
    PromptNotAString,
    #[error("the card is not I-JSON: {0}")]
    Canonical(#[from] JcsError),
}

/// The stream-id grammar of `ledger.streams` (§3.1.2).
pub(crate) fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '@' | '-'))
}

/// Resolves `raw` by the rules in the module docs.
pub fn resolve(raw: &Value) -> Result<ResolvedCard, CardError> {
    let mut obj: Map<String, Value> = raw.as_object().cloned().ok_or(CardError::NotAnObject)?;
    let agent_id = obj
        .get("agent_id")
        .and_then(Value::as_str)
        .ok_or(CardError::MissingAgentId)?
        .to_string();
    if !valid_id(&agent_id) {
        return Err(CardError::InvalidAgentId(agent_id));
    }
    for f in EXCLUDED_FIELDS {
        obj.remove(*f);
    }
    obj.insert("$excluded".into(), json!(EXCLUDED_FIELDS));
    if obj.get("system_prompt").is_none_or(Value::is_null) {
        obj.insert("system_prompt".into(), json!(""));
    }
    let caps = obj.entry("capabilities").or_insert_with(|| json!({}));
    if let Some(caps) = caps.as_object_mut()
        && !caps.contains_key("tools_allowed")
    {
        let mut names: Vec<Value> = Vec::new();
        let declared = caps.get("mcp_tools").and_then(Value::as_array);
        for t in declared.into_iter().flatten() {
            if let Some(n) = t.get("name")
                && !names.contains(n)
            {
                names.push(n.clone());
            }
        }
        caps.insert("tools_allowed".into(), Value::Array(names));
    }
    let prompt = obj
        .get("system_prompt")
        .and_then(Value::as_str)
        .ok_or(CardError::PromptNotAString)?
        .to_string();
    let json = Value::Object(obj);
    Ok(ResolvedCard {
        card_sha: sha256_jcs(&json)?,
        prompt_sha: sha256_raw(prompt.as_bytes()),
        agent_id,
        json,
    })
}
