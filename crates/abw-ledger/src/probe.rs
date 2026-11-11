//! The conformance probe aggregate (ADR-0004 item 1).
//!
//! Stream kind `probe`, event type `probe.recorded` v1 (structural, payload any JSON
//! object), inline projection `proj.probes`. It exists so the substrate is proven through
//! the real dispatch path before the first domain aggregate arrives. Domain code never
//! uses it (`cargo xtask check`, probe isolation).

use crate::artifact::PendingArtifact;
use crate::command::{
    BoxFuture, Expected, Heads, NewEvent, Recorded, StreamAppend, check_payload,
    sealed::CommandSpec, stream_id,
};
use crate::ctx::CommandCtx;
use crate::error::{DispatchError, Rejection};
use crate::outbox::OutboxJob;
use crate::registry::PROBE_RECORDED;
use abw_canon::Sha256Ref;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub(crate) const KIND: &str = "probe";

/// Longest accepted idempotency key.
const KEY_MAX: usize = 256;

/// Append one `probe.recorded` event with `body` as its payload to `probe:<id>`.
#[derive(Debug, Clone)]
pub struct Append {
    id: String,
    expected: Expected,
    body: Value,
    key: Option<String>,
    artifacts: Vec<PendingArtifact>,
    outbox: Vec<OutboxJob>,
}

impl Append {
    pub fn new(id: impl Into<String>, expected: Expected, body: Value) -> Append {
        Append {
            id: id.into(),
            expected,
            body,
            key: None,
            artifacts: Vec::new(),
            outbox: Vec::new(),
        }
    }

    /// Makes the command idempotent on `key` (§3.2.1 step 2).
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Append {
        self.key = Some(key.into());
        self
    }

    /// Stores `artifact` in the same transaction; `created_pos` is this event (step 7).
    pub fn artifact(mut self, artifact: PendingArtifact) -> Append {
        self.artifacts.push(artifact);
        self
    }

    /// Emits `job` in the same transaction; `source_pos` is this event (step 11).
    pub fn outbox(mut self, job: OutboxJob) -> Append {
        self.outbox.push(job);
        self
    }

    fn stream_id(&self) -> Result<String, Rejection> {
        stream_id(KIND, &self.id)
    }
}

/// The result of an [`Append`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Appended {
    pub stream_id: String,
    pub seq: u64,
    pub global_pos: i64,
    #[serde(with = "sha_serde")]
    pub payload_sha: Sha256Ref,
}

mod sha_serde {
    use abw_canon::Sha256Ref;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(sha: &Sha256Ref, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(sha.as_str())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Sha256Ref, D::Error> {
        let text = String::deserialize(d)?;
        Sha256Ref::parse(&text).map_err(serde::de::Error::custom)
    }
}

impl CommandSpec for Append {
    const TYPE: &'static str = "probe.append";
    type Output = Appended;
    type Route = ();
    type Facts = ();

    fn validate(&self, _ctx: &CommandCtx) -> Result<(), Rejection> {
        self.stream_id()?;
        if !self.body.is_object() {
            return Err(Rejection::Payload(
                "a probe body must be a JSON object".to_string(),
            ));
        }
        check_payload(&PROBE_RECORDED, &self.body)?;
        if let Some(k) = &self.key
            && (k.trim().is_empty() || k.len() > KEY_MAX || k.chars().any(char::is_control))
        {
            return Err(Rejection::InvalidIdempotencyKey(k.clone()));
        }
        for a in &self.artifacts {
            a.validate().map_err(Rejection::Artifact)?;
        }
        for j in &self.outbox {
            j.validate().map_err(Rejection::Outbox)?;
        }
        Ok(())
    }

    fn idempotency_key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    fn request(&self) -> Value {
        json!({
            "stream_id": self.stream_id().ok(),
            "expected": self.expected.head(),
            "body": self.body,
            "artifacts": self.artifacts.iter().map(|a| json!({
                "sha": a.sha.as_str(),
                "canonicalization": a.canon.as_str(),
                "media_type": a.media_type,
            })).collect::<Vec<_>>(),
            "outbox": self.outbox.iter().map(|j| json!({
                "topic": j.topic,
                "dedupe_key": j.dedupe_key,
                "payload": j.payload,
            })).collect::<Vec<_>>(),
        })
    }

    fn route<'a>(
        &'a self,
        _conn: &'a mut sqlx::PgConnection,
    ) -> BoxFuture<'a, Result<(), DispatchError>> {
        crate::fold::ready(())
    }

    fn write_set(&self, _route: &()) -> Vec<String> {
        self.stream_id().map(|s| vec![s]).unwrap_or_default()
    }

    fn read_set(&self, _route: &()) -> Vec<String> {
        Vec::new()
    }

    /// The probe decides from heads alone.
    fn fold<'a>(
        &'a self,
        _conn: &'a mut sqlx::PgConnection,
        _route: &'a (),
        _ctx: &'a CommandCtx,
    ) -> BoxFuture<'a, Result<(), DispatchError>> {
        crate::fold::ready(())
    }

    fn decide(
        &self,
        _heads: &Heads,
        _facts: &(),
        _ctx: &CommandCtx,
    ) -> Result<Vec<StreamAppend>, DispatchError> {
        let stream_id = self.stream_id().map_err(DispatchError::Rejected)?;
        let mut event =
            NewEvent::new(&PROBE_RECORDED, self.body.clone()).map_err(DispatchError::Rejected)?;
        event.artifacts = self.artifacts.clone();
        event.outbox = self.outbox.clone();
        Ok(vec![StreamAppend {
            stream_id,
            kind: KIND,
            expected: self.expected,
            events: vec![event],
        }])
    }

    fn output(&self, recorded: &[Recorded], _facts: &()) -> Result<Appended, DispatchError> {
        match recorded {
            [r] => Ok(Appended {
                stream_id: r.stream_id.clone(),
                seq: r.seq,
                global_pos: r.global_pos,
                payload_sha: r.payload_sha.clone(),
            }),
            _ => Err(DispatchError::Integrity(format!(
                "probe.append expected one recorded event, got {}",
                recorded.len()
            ))),
        }
    }
}

/// A `proj.probes` row.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProbeRow {
    pub(crate) seq: u64,
    pub(crate) body: Value,
    pub(crate) applied_pos: i64,
}

/// The projector's pure fold step, shared by the inline write and by rebuild (§3.4.3).
pub(crate) fn apply(
    _prev: Option<&ProbeRow>,
    seq: u64,
    payload: &Value,
    global_pos: i64,
) -> ProbeRow {
    ProbeRow {
        seq,
        body: payload.clone(),
        applied_pos: global_pos,
    }
}
