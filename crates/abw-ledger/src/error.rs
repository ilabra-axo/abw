//! Typed errors (target-system-design §3.8 `DispatchError`; §13.2 boot checks).

use crate::command::Expected;

/// A typed refusal by the command itself (§3.2.1 step 0 or step 5), before anything is
/// written. Maps to 4xx.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Rejection {
    #[error("invalid stream id {0:?}: must match <kind>:[A-Za-z0-9._@-]{{1,128}}")]
    InvalidStreamId(String),
    #[error("invalid tenant id {0:?}")]
    InvalidTenant(String),
    #[error("invalid actor id {0:?}")]
    InvalidActor(String),
    #[error("invalid idempotency key {0:?}")]
    InvalidIdempotencyKey(String),
    #[error("payload rejected: {0}")]
    Payload(String),
    #[error("payload is {len} bytes of JCS; the limit is {max} (OD-05)")]
    PayloadTooLarge { len: usize, max: usize },
    #[error("artifact rejected: {0}")]
    Artifact(String),
    #[error("outbox job rejected: {0}")]
    Outbox(String),
    #[error("stream {stream} belongs to another tenant")]
    TenantMismatch { stream: String },
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    #[error("rejected: {0}")]
    Rejected(Rejection),
    /// The expected version did not match the stream head (§3.2.1 steps 6a/6b). 409.
    #[error("conflict on {stream}: expected {expected:?}, actual head {actual}")]
    Conflict {
        stream: String,
        expected: Expected,
        actual: u64,
    },
    /// The idempotency key was used before for a different request (§3.2.1 step 2). 409.
    #[error("idempotency key {key:?} was used for a different request")]
    IdempotencyMismatch { key: String },

    /// A database invariant (ABW-T* trigger, CHECK, deferred FK) refused the append. A bug.
    #[error("integrity check refused the append: {0}")]
    Integrity(String),
    /// A stored dedup result could not be decoded.
    #[error("stored command result for key {key:?} is unreadable: {reason}")]
    CorruptResult { key: String, reason: String },
    #[error("database error: {0}")]
    Db(#[source] sqlx::Error),
    /// The dispatch failed with `cause` and the ROLLBACK then failed too. The server rolls
    /// the transaction back when the connection drops, so nothing was committed.
    #[error("{cause}; rollback also failed: {rollback}")]
    RollbackFailed {
        cause: Box<DispatchError>,
        #[source]
        rollback: sqlx::Error,
    },
    /// ADR-0004 item 4: an injected fault aborted the dispatch at this point.
    #[cfg(feature = "fault-injection")]
    #[error("injected fault at {0:?}")]
    Injected(crate::fault::FaultPoint),
}

impl From<sqlx::Error> for DispatchError {
    fn from(e: sqlx::Error) -> DispatchError {
        // A refusal by one of the ledger's own invariants is an integrity failure, not an
        // ordinary database error: it means the Rust side tried something the schema forbids.
        let integrity = e.as_database_error().and_then(|d| {
            let msg = d.message();
            let code = d.code().map(|c| c.to_string()).unwrap_or_default();
            let ours = msg.starts_with("ABW-T");
            let constraint = matches!(code.as_str(), "23503" | "23505" | "23514" | "23502");
            (ours || constraint).then(|| format!("[{code}] {msg}"))
        });
        match integrity {
            Some(m) => DispatchError::Integrity(m),
            None => DispatchError::Db(e),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BootError {
    #[error("{what}: {source}")]
    Db {
        what: &'static str,
        #[source]
        source: sqlx::Error,
    },
    /// §2.6 step 2: the write pool opens only while this process holds the boot fence.
    #[error("the singleton fence is not held for this database: {0}")]
    FenceNotHeld(String),
    /// §13.2: ownership bypasses REVOKE, so the app role must not own (or be able to become
    /// the owner of) any table.
    #[error("the app role {role} owns or can assume the owner of {tables:?}")]
    AppRoleOwnsTables { role: String, tables: Vec<String> },
    #[error("the app role {0} is a superuser or bypasses row security")]
    AppRolePrivileged(String),
    /// §13.2: abw_read must not be able to write.
    #[error("the read handle can write: {0}")]
    ReadRoleCanWrite(String),
    #[error("the app and read handles point at different databases ({app} vs {read})")]
    DatabaseMismatch { app: String, read: String },
    /// SUB-02: an unmigrated database does not boot.
    #[error("the database is not migrated (ledger.schema_migrations is missing)")]
    NotMigrated,
    /// SUB-02: applied migrations must equal the embedded set, name and checksum.
    #[error("applied migrations differ from this build: {0}")]
    MigrationMismatch(String),
    /// §3.7, OD-08: the Rust event registry and ledger.event_types must be equal sets.
    #[error(
        "event registry differs from ledger.event_types: missing in the table {missing_in_table:?}, unknown to this build {unknown_to_build:?}"
    )]
    RegistryMismatch {
        missing_in_table: Vec<String>,
        unknown_to_build: Vec<String>,
    },
    #[error("event registry is invalid: {0}")]
    InvalidRegistry(String),
}

impl BootError {
    pub(crate) fn db(what: &'static str) -> impl FnOnce(sqlx::Error) -> BootError {
        move |source| BootError::Db { what, source }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    /// Defence in depth behind `read_query!`: the query is re-checked before it is sent.
    #[error("not a read-only query: {0}")]
    NotReadOnly(String),
    #[error("could not bind a query parameter: {0}")]
    Bind(String),
    /// The stored artifact does not hash to its key, or is not stored inline.
    #[error("artifact {sha}: {reason}")]
    Artifact { sha: String, reason: String },
}
