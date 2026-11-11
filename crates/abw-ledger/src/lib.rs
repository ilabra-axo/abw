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
pub mod agent;
pub mod allowance;
mod artifact;
mod boot;
mod command;
pub mod crud;
mod ctx;
mod dispatch;
pub mod episode;
mod error;
mod fault;
mod fence;
mod fold;
pub mod migrate;
mod outbox;
pub mod policy;
pub mod probe;
mod projection;
mod read;
pub mod reaper;
mod registry;
mod rows;
mod sql_check;

pub use abw_canon::{JcsError, Sha256Ref};
pub use abw_ledger_macros::read_query;
pub use artifact::{ARTIFACT_INLINE_MAX, Canon, PendingArtifact, artifact_bytes, artifact_json};
pub use boot::{Ledger, LedgerConfig};
pub use command::{Command, Dispatched, Expected};
pub use crud::Crud;
pub use ctx::{Actor, ActorKind, CommandCtx, PLATFORM_TENANT};
pub use error::{BootError, DispatchError, ReadError, Rejection};
#[cfg(feature = "fault-injection")]
pub use fault::FaultPoint;
pub use fence::{FenceError, SingletonFence};
pub use outbox::OutboxJob;
pub use read::{ReadDb, ReadQuery};
/// Row access for [`ReadDb`] results. Handlers see sqlx types only through these re-exports.
pub use sqlx::Row;
pub use sqlx::postgres::PgRow;
