//! One canonical form and one hash format (T4, target-system-design §3.6).
//!
//! - [`parse`] reads I-JSON (RFC 7493): duplicate member names, lone surrogates, integers
//!   outside ±(2^53 − 1) and non-finite numbers are rejected (§3.6.1 rule 1).
//! - [`jcs`] writes RFC 8785 canonical bytes. Keys are sorted explicitly by UTF-16 code
//!   units and numbers use the ECMAScript algorithm; neither map iteration order nor
//!   serde_json's float formatter is relied on (§3.6.1, "Rust implementation constraint").
//! - [`Sha256Ref`] is the only hash format: `sha256:` and 64 lowercase hex (rule 8).

mod canon;
mod hash;
mod parse;

pub use canon::jcs;
pub use hash::{HashFormatError, Sha256Ref, sha256_jcs, sha256_raw};
pub use parse::parse;

/// Maximum nesting of arrays and objects accepted by [`parse`] and [`jcs`].
pub const MAX_DEPTH: usize = 128;

/// Largest integer magnitude representable exactly in binary64: 2^53 − 1.
pub const MAX_SAFE_INTEGER: u64 = (1 << 53) - 1;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum JcsError {
    #[error("invalid JSON at byte {offset}: {reason}")]
    Syntax { offset: usize, reason: &'static str },
    #[error("duplicate member name {name:?} at byte {offset}")]
    DuplicateMember { name: String, offset: usize },
    #[error("lone surrogate \\u{code:04x} at byte {offset}")]
    LoneSurrogate { code: u16, offset: usize },
    #[error("integer {literal} is outside ±(2^53 − 1)")]
    IntegerOutOfRange { literal: String },
    #[error("number {literal} is not a finite binary64 value")]
    NonFinite { literal: String },
    #[error("nesting deeper than {MAX_DEPTH}")]
    TooDeep,
}
