//! The hash format: `sha256:` followed by 64 lowercase hex digits (§3.6.1 rule 8).

use crate::{JcsError, jcs};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt;

const PREFIX: &str = "sha256:";

/// A validated `sha256:<64 lowercase hex>` reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Sha256Ref(String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("not a sha256:<64 lowercase hex> reference: {0:?}")]
pub struct HashFormatError(pub String);

impl Sha256Ref {
    pub fn parse(s: &str) -> Result<Sha256Ref, HashFormatError> {
        let ok = s.strip_prefix(PREFIX).is_some_and(|hex| {
            hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        });
        if ok {
            Ok(Sha256Ref(s.to_string()))
        } else {
            Err(HashFormatError(s.to_string()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn of_digest(digest: &[u8]) -> Sha256Ref {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut s = String::with_capacity(PREFIX.len() + 64);
        s.push_str(PREFIX);
        for b in digest {
            s.push(char::from(HEX[usize::from(b >> 4)]));
            s.push(char::from(HEX[usize::from(b & 0xf)]));
        }
        Sha256Ref(s)
    }
}

impl fmt::Display for Sha256Ref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for Sha256Ref {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// The hash of the bytes as given (raw-utf8 and raw-bytes artifacts).
pub fn sha256_raw(bytes: &[u8]) -> Sha256Ref {
    Sha256Ref::of_digest(&Sha256::digest(bytes))
}

/// The hash of the JCS bytes of `v`.
pub fn sha256_jcs(v: &Value) -> Result<Sha256Ref, JcsError> {
    Ok(sha256_raw(&jcs(v)?))
}
