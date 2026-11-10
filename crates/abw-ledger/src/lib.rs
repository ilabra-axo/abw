//! The event store and the write boundary (target-system-design §3, T1–T6).
//!
//! `abw-ledger` is the only crate with a write-capable database handle. Trust state changes
//! only by appending events through [`Ledger::dispatch`]; handlers get [`ReadDb`], which
//! runs only queries built by [`read_query!`]. The surface the conformance tests use is
//! fixed by ADR-0004.

// The `read_query!` expansion names `::abw_ledger::ReadQuery`; this makes the path resolve
// inside the crate too.
extern crate self as abw_ledger;

pub mod admin;
mod artifact;
mod boot;
mod command;
mod ctx;
mod dispatch;
mod error;
mod fault;
mod fence;
pub mod migrate;
mod outbox;
pub mod probe;
mod projection;
mod read;
mod registry;
mod sql_check;

pub use abw_canon::{JcsError, Sha256Ref};
pub use abw_ledger_macros::read_query;
pub use artifact::{ARTIFACT_INLINE_MAX, Canon, PendingArtifact, artifact_bytes, artifact_json};
pub use boot::{Ledger, LedgerConfig};
pub use command::{Command, Dispatched, Expected};
pub use ctx::{Actor, ActorKind, CommandCtx};
pub use error::{BootError, DispatchError, ReadError, Rejection};
#[cfg(feature = "fault-injection")]
pub use fault::FaultPoint;
pub use fence::{FenceError, SingletonFence};
pub use outbox::OutboxJob;
pub use read::{ReadDb, ReadQuery};
/// Row access for [`ReadDb`] results. Handlers see sqlx types only through these re-exports.
pub use sqlx::Row;
pub use sqlx::postgres::PgRow;
