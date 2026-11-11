//! The closed event-type registry (target-system-design §3.7, §12.6, OD-08).
//!
//! Each event type is declared once here, in Rust, and registered by a migration in
//! `ledger.event_types`. Boot compares the two as sets and refuses to start on any
//! difference (`boot.rs`). Payloads are checked against the declared schema before the
//! insert (§3.2.1 step 8, "the JSON-schema check in Rust before the insert").

use abw_canon::{Sha256Ref, sha256_jcs};
use serde_json::Value;

/// `ledger.event_types.epistemic` (§3.1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Epistemic {
    Evidentiary,
    #[allow(dead_code)] // declared by the M3+ catalogue
    Interpretive,
    Structural,
    Decision,
    #[allow(dead_code)]
    Observation,
}

impl Epistemic {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Epistemic::Evidentiary => "evidentiary",
            Epistemic::Interpretive => "interpretive",
            Epistemic::Structural => "structural",
            Epistemic::Decision => "decision",
            Epistemic::Observation => "observation",
        }
    }
}

#[derive(Debug)]
pub(crate) struct EventTypeDef {
    pub(crate) event_type: &'static str,
    pub(crate) schema_version: i32,
    pub(crate) stream_kind: &'static str,
    pub(crate) epistemic: Epistemic,
    /// The JSON Schema, as JSON text. Its JCS hash must equal the table's json_schema_sha.
    pub(crate) json_schema: &'static str,
}

pub(crate) const PROBE_RECORDED: EventTypeDef = EventTypeDef {
    event_type: "probe.recorded",
    schema_version: 1,
    stream_kind: "probe",
    epistemic: Epistemic::Structural,
    json_schema: r#"{"type":"object"}"#,
};

// ADR-0008 §2: the M2 event types. Every schema names the payload fields the conformance
// tests read as `required`; extra fields are allowed.

pub(crate) const POLICY_ADJUSTED: EventTypeDef = EventTypeDef {
    event_type: "policy.adjusted",
    schema_version: 1,
    stream_kind: "policy",
    epistemic: Epistemic::Decision,
    json_schema: r#"{"type":"object","required":["policy_id","version","policy_kind","doc_sha","accepted_by","rationale","active_from"]}"#,
};

pub(crate) const AGENT_VERSION_CREATED: EventTypeDef = EventTypeDef {
    event_type: "agent.version_created",
    schema_version: 1,
    stream_kind: "agent",
    epistemic: Epistemic::Structural,
    json_schema: r#"{"type":"object","required":["agent_id","version_number","card_sha","prompt_sha","reason"]}"#,
};

pub(crate) const AGENT_RETIRED: EventTypeDef = EventTypeDef {
    event_type: "agent.retired",
    schema_version: 1,
    stream_kind: "agent",
    epistemic: Epistemic::Structural,
    json_schema: r#"{"type":"object","required":["agent_id","reason"]}"#,
};

pub(crate) const EPISODE_OPENED: EventTypeDef = EventTypeDef {
    event_type: "episode.opened",
    schema_version: 1,
    stream_kind: "episode",
    epistemic: Epistemic::Evidentiary,
    json_schema: r#"{"type":"object","required":["episode_id","tenant_id","owner_id","agent_id","version_number","card_sha","input_sha","hold_id","hold_cu","price_version","tools_allowed","opened_at"]}"#,
};

pub(crate) const TOOL_INVOKED: EventTypeDef = EventTypeDef {
    event_type: "tool.invoked",
    schema_version: 1,
    stream_kind: "episode",
    epistemic: Epistemic::Evidentiary,
    json_schema: r#"{"type":"object","required":["iteration","call_index","tool_name","tool_version","allowed","input_sha","output_sha","duration_ms","error_class"]}"#,
};

pub(crate) const GATE_DECIDED: EventTypeDef = EventTypeDef {
    event_type: "gate.decided",
    schema_version: 1,
    stream_kind: "episode",
    epistemic: Epistemic::Decision,
    json_schema: r#"{"type":"object","required":["gate","decision","reason_class","subject","episode_id"]}"#,
};

pub(crate) const EPISODE_CLOSED: EventTypeDef = EventTypeDef {
    event_type: "episode.closed",
    schema_version: 1,
    stream_kind: "episode",
    epistemic: Epistemic::Evidentiary,
    json_schema: r#"{"type":"object","required":["status","reason_class","raw_output_sha","tool_call_shas","tokens_in","tokens_out","tool_calls","consumed_cu"]}"#,
};

pub(crate) const ALLOWANCE_OPENED: EventTypeDef = EventTypeDef {
    event_type: "allowance.opened",
    schema_version: 1,
    stream_kind: "allowance",
    epistemic: Epistemic::Evidentiary,
    json_schema: r#"{"type":"object","required":["owner_id","idempotency_key"]}"#,
};

pub(crate) const ALLOWANCE_GRANTED: EventTypeDef = EventTypeDef {
    event_type: "allowance.granted",
    schema_version: 1,
    stream_kind: "allowance",
    epistemic: Epistemic::Evidentiary,
    json_schema: r#"{"type":"object","required":["owner_id","kind","amount_cu","grant_policy_version","idempotency_key"]}"#,
};

pub(crate) const ALLOWANCE_HOLD_PLACED: EventTypeDef = EventTypeDef {
    event_type: "allowance.hold_placed",
    schema_version: 1,
    stream_kind: "allowance",
    epistemic: Epistemic::Evidentiary,
    json_schema: r#"{"type":"object","required":["owner_id","episode_id","hold_id","amount_cu","workspace_id","idempotency_key"]}"#,
};

pub(crate) const ALLOWANCE_CONSUMED: EventTypeDef = EventTypeDef {
    event_type: "allowance.consumed",
    schema_version: 1,
    stream_kind: "allowance",
    epistemic: Epistemic::Evidentiary,
    json_schema: r#"{"type":"object","required":["owner_id","episode_id","hold_id","held_cu","priced_cu","consumed_cu","released_cu","split","usage","price_version","idempotency_key"]}"#,
};

pub(crate) const ALLOWANCE_HOLD_RELEASED: EventTypeDef = EventTypeDef {
    event_type: "allowance.hold_released",
    schema_version: 1,
    stream_kind: "allowance",
    epistemic: Epistemic::Evidentiary,
    json_schema: r#"{"type":"object","required":["owner_id","episode_id","hold_id","amount_cu","reason_class","idempotency_key"]}"#,
};

/// Every event type this build can append or fold.
pub(crate) const REGISTRY: &[&EventTypeDef] = &[
    &PROBE_RECORDED,
    &POLICY_ADJUSTED,
    &AGENT_VERSION_CREATED,
    &AGENT_RETIRED,
    &EPISODE_OPENED,
    &TOOL_INVOKED,
    &GATE_DECIDED,
    &EPISODE_CLOSED,
    &ALLOWANCE_OPENED,
    &ALLOWANCE_GRANTED,
    &ALLOWANCE_HOLD_PLACED,
    &ALLOWANCE_CONSUMED,
    &ALLOWANCE_HOLD_RELEASED,
];

/// The JSON Schema keywords the payload check enforces. A schema using any other keyword
/// fails boot: a declared constraint that is not enforced would be a check that records
/// without refusing (AGENTS.md rule 4).
const SUPPORTED_KEYWORDS: &[&str] = &["type", "required"];

impl EventTypeDef {
    pub(crate) fn schema(&self) -> Result<Value, String> {
        let v = abw_canon::parse(self.json_schema)
            .map_err(|e| format!("{}: schema is not I-JSON: {e}", self.event_type))?;
        let Value::Object(map) = &v else {
            return Err(format!("{}: schema is not an object", self.event_type));
        };
        if let Some(k) = map
            .keys()
            .find(|k| !SUPPORTED_KEYWORDS.contains(&k.as_str()))
        {
            return Err(format!(
                "{}: schema keyword {k:?} is not enforced by this build",
                self.event_type
            ));
        }
        Ok(v)
    }

    pub(crate) fn schema_sha(&self) -> Result<Sha256Ref, String> {
        let v = self.schema()?;
        sha256_jcs(&v).map_err(|e| format!("{}: {e}", self.event_type))
    }

    /// Checks `payload` against the declared schema.
    pub(crate) fn check_payload(&self, payload: &Value) -> Result<(), String> {
        let schema = self.schema()?;
        if let Some(t) = schema.get("type") {
            let want = t.as_str().ok_or("schema `type` must be a string")?;
            let ok = match want {
                "object" => payload.is_object(),
                "array" => payload.is_array(),
                "string" => payload.is_string(),
                "number" => payload.is_number(),
                "integer" => payload.is_i64() || payload.is_u64(),
                "boolean" => payload.is_boolean(),
                "null" => payload.is_null(),
                other => return Err(format!("schema type {other:?} is not supported")),
            };
            if !ok {
                return Err(format!("payload is not of type {want}"));
            }
        }
        if let Some(req) = schema.get("required") {
            let names = req.as_array().ok_or("schema `required` must be an array")?;
            for n in names {
                let name = n.as_str().ok_or("schema `required` must list strings")?;
                if payload.get(name).is_none() {
                    return Err(format!("payload lacks required field {name:?}"));
                }
            }
        }
        Ok(())
    }
}

/// `(event_type, schema_version, stream_kind, epistemic, json_schema_sha)` for every entry,
/// the shape the boot check compares with `ledger.event_types`.
pub(crate) fn registry_rows() -> Result<Vec<String>, String> {
    let mut rows = Vec::with_capacity(REGISTRY.len());
    for d in REGISTRY {
        rows.push(row_key(
            d.event_type,
            d.schema_version,
            d.stream_kind,
            d.epistemic.as_str(),
            d.schema_sha()?.as_str(),
        ));
    }
    rows.sort();
    Ok(rows)
}

pub(crate) fn row_key(
    event_type: &str,
    schema_version: i32,
    stream_kind: &str,
    epistemic: &str,
    sha: &str,
) -> String {
    format!("{event_type} v{schema_version} on {stream_kind} ({epistemic}) {sha}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_schema_sha_matches_the_migration() {
        let sha = PROBE_RECORDED.schema_sha();
        assert_eq!(
            sha.as_ref().map(Sha256Ref::as_str),
            Ok("sha256:a2c799262a3ce3c19ef5cdd983bf3d12b43ab3c426227091b909dcb7054738c0")
        );
        let migration = include_str!("../migrations/0002_probe.sql");
        assert!(
            migration.contains(
                "sha256:a2c799262a3ce3c19ef5cdd983bf3d12b43ab3c426227091b909dcb7054738c0"
            )
        );
    }

    #[test]
    fn every_schema_sha_is_registered_by_a_migration() {
        let migrations: String = crate::migrate::MIGRATIONS.iter().map(|m| m.sql).collect();
        for d in REGISTRY {
            let sha = d.schema_sha();
            assert!(sha.is_ok(), "{}: {sha:?}", d.event_type);
            if let Ok(sha) = sha {
                assert!(
                    migrations.contains(sha.as_str()),
                    "{} {sha} is not in any migration",
                    d.event_type
                );
            }
        }
    }

    #[test]
    fn required_fields_are_enforced() {
        assert!(
            ALLOWANCE_OPENED
                .check_payload(&serde_json::json!({"owner_id": "o", "idempotency_key": "open:o"}))
                .is_ok()
        );
        assert!(
            ALLOWANCE_OPENED
                .check_payload(&serde_json::json!({"owner_id": "o"}))
                .is_err()
        );
    }

    #[test]
    fn payload_type_is_enforced() {
        assert!(
            PROBE_RECORDED
                .check_payload(&serde_json::json!({"a": 1}))
                .is_ok()
        );
        assert!(
            PROBE_RECORDED
                .check_payload(&serde_json::json!([1]))
                .is_err()
        );
    }
}
