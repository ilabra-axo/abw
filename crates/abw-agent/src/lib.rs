//! Agents at run time (target-system-design §4, §12.5; ADR-0008 §5-§8).
//!
//! - [`card`] resolves a legacy card into the JSON whose JCS hash is its version id, and
//!   [`import`] seeds the ported cards under `agents/` as agent versions (PROT-09).
//! - [`llm`] is the provider interface with a deterministic [`llm::ScriptedProvider`];
//!   [`tools`] is the tool registry; [`budget`] sizes the hold.
//! - [`executor`] runs one episode: TX-A, the bounded loop with one TX-B per tool call,
//!   TX-C (§4.5). No ledger transaction is open while it awaits a provider or a tool: it only
//!   reaches the ledger through `Ledger::dispatch` (SUB-06).
//! - [`egress`] is the outbound SSRF guard (§12.5 `net_guard`), the only code that builds
//!   an outbound HTTP client (T18).

pub mod budget;
pub mod card;
pub mod egress;
pub mod executor;
pub mod import;
pub mod llm;
pub mod tools;

/// A boxed `Send` future: the return type of the object-safe async traits here.
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;
