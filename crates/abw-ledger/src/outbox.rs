//! Transactional outbox jobs (target-system-design §3.1.6, §3.2.1 step 11, SUB-03).
//!
//! A job is inserted in the transaction that appends the event causing it and references
//! that event (`source_pos`), so a side effect without a recorded decision cannot exist.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct OutboxJob {
    pub topic: String,
    /// Unique across the outbox; a second job with the same key is dropped
    /// (`ON CONFLICT (dedupe_key) DO NOTHING`), and consumers are idempotent on it.
    pub dedupe_key: String,
    pub payload: Value,
}

impl OutboxJob {
    pub fn new(topic: impl Into<String>, dedupe_key: impl Into<String>, payload: Value) -> Self {
        OutboxJob {
            topic: topic.into(),
            dedupe_key: dedupe_key.into(),
            payload,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        let topic_ok = !self.topic.is_empty()
            && self
                .topic
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_'));
        if !topic_ok {
            return Err(format!("invalid topic {:?}", self.topic));
        }
        if self.dedupe_key.trim().is_empty() {
            return Err("empty dedupe key".to_string());
        }
        abw_canon::jcs(&self.payload)
            .map(|_| ())
            .map_err(|e| format!("payload is not I-JSON: {e}"))
    }
}
