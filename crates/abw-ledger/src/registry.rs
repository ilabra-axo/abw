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
    #[allow(dead_code)] // declared by the M2+ catalogue
    Evidentiary,
    #[allow(dead_code)]
    Interpretive,
    Structural,
    #[allow(dead_code)]
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

/// Every event type this build can append or fold.
pub(crate) const REGISTRY: &[&EventTypeDef] = &[&PROBE_RECORDED];

/// The JSON Schema keywords the payload check enforces. A schema using any other keyword
/// fails boot: a declared constraint that is not enforced would be a check that records
/// without refusing (AGENTS.md rule 4).
const SUPPORTED_KEYWORDS: &[&str] = &["type"];

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
