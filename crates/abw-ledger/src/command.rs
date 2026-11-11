//! Commands and event envelopes (target-system-design §3.7, §3.8).
//!
//! A command is validated purely (§3.2.1 step 0), declares its write and read sets (step
//! 3), folds the state it needs from the locked streams (step 4) and decides its appends
//! synchronously from that state (step 5). The ledger does everything else. [`Command`]
//! stays sealed in M2 (ADR-0008 §1 item 5): every aggregate lives in this crate, so no
//! decision and no transaction is reachable from outside it (SUB-06).

use crate::artifact::PendingArtifact;
use crate::ctx::CommandCtx;
use crate::error::{DispatchError, Rejection};
use crate::outbox::OutboxJob;
use crate::registry::EventTypeDef;
use abw_canon::Sha256Ref;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::BTreeMap;

/// Payloads must stay under 64 KiB of JCS (OD-05); anything larger is an artifact.
pub(crate) const PAYLOAD_MAX: usize = 64 * 1024;

/// The expected version of a stream for compare-and-swap (§3.8). There is no `Any`: the
/// only commutative stream kind (`telemetry`) gets its own append path when it lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expected {
    /// The stream must not exist yet.
    NoStream,
    /// The stream head must be exactly this. `Exact(0)` means the same as `NoStream`: a
    /// stream with no events has version 0.
    Exact(u64),
}

impl Expected {
    pub(crate) fn head(self) -> u64 {
        match self {
            Expected::NoStream => 0,
            Expected::Exact(n) => n,
        }
    }
}

/// The result of a dispatch. `replayed` is true when an idempotent retry returned the
/// stored result of the first attempt instead of running again (§3.2.1 step 2).
#[derive(Debug, Clone, PartialEq)]
pub struct Dispatched<O> {
    pub output: O,
    pub replayed: bool,
}

/// A validated event to append: payload checked against its registered schema, JCS
/// hash computed once from the Rust value (§3.1.4 physical notes).
#[derive(Debug, Clone)]
pub struct NewEvent {
    pub(crate) def: &'static EventTypeDef,
    pub(crate) payload: Value,
    pub(crate) payload_sha: Sha256Ref,
    pub(crate) artifacts: Vec<PendingArtifact>,
    pub(crate) outbox: Vec<OutboxJob>,
}

impl NewEvent {
    pub(crate) fn new(def: &'static EventTypeDef, payload: Value) -> Result<NewEvent, Rejection> {
        let payload_sha = check_payload(def, &payload)?;
        Ok(NewEvent {
            def,
            payload,
            payload_sha,
            artifacts: Vec::new(),
            outbox: Vec::new(),
        })
    }
}

/// The pure envelope checks for a payload of type `def`, returning its sha.
pub(crate) fn check_payload(def: &EventTypeDef, payload: &Value) -> Result<Sha256Ref, Rejection> {
    def.check_payload(payload).map_err(Rejection::Payload)?;
    if contains_nul(payload) {
        // jsonb cannot store U+0000 (Appendix D case 29); refuse it before any write.
        return Err(Rejection::Payload(
            "strings may not contain U+0000 (jsonb cannot store it)".to_string(),
        ));
    }
    let bytes = abw_canon::jcs(payload).map_err(|e| Rejection::Payload(e.to_string()))?;
    if bytes.len() > PAYLOAD_MAX {
        return Err(Rejection::PayloadTooLarge {
            len: bytes.len(),
            max: PAYLOAD_MAX,
        });
    }
    Ok(abw_canon::sha256_raw(&bytes))
}

fn contains_nul(v: &Value) -> bool {
    match v {
        Value::String(s) => s.contains('\0'),
        Value::Array(items) => items.iter().any(contains_nul),
        Value::Object(map) => map.iter().any(|(k, v)| k.contains('\0') || contains_nul(v)),
        _ => false,
    }
}

/// The events one command appends to one stream at one expected version.
#[derive(Debug, Clone)]
pub struct StreamAppend {
    pub(crate) stream_id: String,
    pub(crate) kind: &'static str,
    pub(crate) expected: Expected,
    pub(crate) events: Vec<NewEvent>,
}

/// Heads of the streams `lock_streams` returned (§3.2.1 step 3); absent streams have no
/// entry.
#[derive(Debug, Default)]
pub struct Heads(pub(crate) BTreeMap<String, u64>);

impl Heads {
    pub(crate) fn head(&self, stream_id: &str) -> u64 {
        self.0.get(stream_id).copied().unwrap_or(0)
    }
}

/// An event as appended in this transaction (§3.7 `RecordedEvent`, the fields used so far).
#[derive(Debug, Clone)]
pub struct Recorded {
    pub(crate) stream_id: String,
    pub(crate) seq: u64,
    pub(crate) global_pos: i64,
    pub(crate) event_type: &'static str,
    pub(crate) payload: Value,
    pub(crate) payload_sha: Sha256Ref,
    /// `ledger.streams.tenant_id` of the stream, which projections carry (§13.1).
    pub(crate) tenant: String,
    pub(crate) actor: String,
}

/// A boxed `Send` future, the shape of the ledger-internal async steps of a command.
pub(crate) type BoxFuture<'a, T> =
    std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

pub(crate) mod sealed {
    use super::*;
    use sqlx::PgConnection;

    /// The command contract. Public only as the sealed supertrait of [`super::Command`].
    ///
    /// The async steps (`route`, `fold`) read the ledger's own tables on the dispatch
    /// connection; nothing else is awaited inside a dispatch. `decide` is a plain
    /// synchronous fn (SUB-06, `cargo xtask check` rule `sync-decide`).
    pub trait CommandSpec: Send + Sync + 'static {
        /// `ledger.command_dedup.command_type`.
        const TYPE: &'static str;
        type Output: Serialize + DeserializeOwned + Send;
        /// Immutable facts read before the lock that the write set depends on (for example
        /// the owner of an episode, fixed by `episode.opened`).
        type Route: Send + Sync;
        /// The state `decide` needs, folded from the locked streams (§3.2.1 step 4).
        type Facts: Send + Sync;

        /// Step 0: pure checks of the command alone. No connection is acquired before it.
        fn validate(&self, ctx: &CommandCtx) -> Result<(), Rejection>;
        fn idempotency_key(&self) -> Option<&str>;
        /// The canonical request, hashed into `command_dedup.request_sha`.
        fn request(&self) -> Value;
        /// Before step 3: reads the immutable facts the stream sets depend on.
        fn route<'a>(
            &'a self,
            conn: &'a mut PgConnection,
        ) -> BoxFuture<'a, Result<Self::Route, DispatchError>>;
        /// Streams appended to (W) and only read (R) (§3.2.1 step 3).
        fn write_set(&self, route: &Self::Route) -> Vec<String>;
        fn read_set(&self, route: &Self::Route) -> Vec<String>;
        /// Step 4: folds the locked streams into the state the decision reads.
        fn fold<'a>(
            &'a self,
            conn: &'a mut PgConnection,
            route: &'a Self::Route,
            ctx: &'a CommandCtx,
        ) -> BoxFuture<'a, Result<Self::Facts, DispatchError>>;
        /// Step 5: pure decision from the locked heads and the folded state.
        fn decide(
            &self,
            heads: &Heads,
            facts: &Self::Facts,
            ctx: &CommandCtx,
        ) -> Result<Vec<StreamAppend>, DispatchError>;
        /// The result, deterministic from the appended events and the folded state
        /// (§3.2.1 step 12).
        fn output(
            &self,
            recorded: &[Recorded],
            facts: &Self::Facts,
        ) -> Result<Self::Output, DispatchError>;
    }
}

/// A command [`crate::Ledger::dispatch`] accepts. Sealed; see the module docs.
pub trait Command: sealed::CommandSpec {}

impl<T: sealed::CommandSpec> Command for T {}

/// `<kind>:<id>` as `ledger.streams` requires (§3.1.2): the kind is `[a-z][a-z_]{1,31}`
/// and the id is 1 to 128 of `[A-Za-z0-9._@-]`.
pub(crate) fn stream_id(kind: &str, id: &str) -> Result<String, Rejection> {
    let kind_ok = kind.len() >= 2
        && kind.len() <= 32
        && kind.starts_with(|c: char| c.is_ascii_lowercase())
        && kind.chars().all(|c| c.is_ascii_lowercase() || c == '_');
    let id_ok = !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '@' | '-'));
    if kind_ok && id_ok {
        Ok(format!("{kind}:{id}"))
    } else {
        Err(Rejection::InvalidStreamId(format!("{kind}:{id}")))
    }
}

#[cfg(test)]
mod tests {
    use super::stream_id;

    #[test]
    fn stream_ids_follow_the_ddl() {
        assert_eq!(
            stream_id("probe", "a.b@c-d_9").ok().as_deref(),
            Some("probe:a.b@c-d_9")
        );
        for bad in ["", "a b", "x:y", &"a".repeat(129)] {
            assert!(stream_id("probe", bad).is_err(), "{bad:?}");
        }
        assert!(stream_id("p", "x").is_err());
        assert!(stream_id("Probe", "x").is_err());
    }
}
