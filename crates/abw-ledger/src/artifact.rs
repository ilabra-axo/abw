//! Content-addressed artifacts (target-system-design §3.1.5, §3.8 "artifacts").

use abw_canon::{JcsError, Sha256Ref, jcs, sha256_raw};
use serde_json::Value;

/// Artifacts up to this size are stored inline in `ledger.artifacts.bytes` (§2.5).
pub const ARTIFACT_INLINE_MAX: usize = 256 * 1024;

/// `ledger.artifacts.canonicalization` (§3.1.5 canonicalisation rules).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Canon {
    /// JSON documents: the bytes are the JCS form.
    JcsRfc8785,
    /// Model text: UTF-8 as given, no normalisation.
    RawUtf8,
    /// Images and binary attachments.
    RawBytes,
}

impl Canon {
    pub fn as_str(self) -> &'static str {
        match self {
            Canon::JcsRfc8785 => "jcs-rfc8785",
            Canon::RawUtf8 => "raw-utf8",
            Canon::RawBytes => "raw-bytes",
        }
    }
}

/// An artifact to be stored in the same transaction as the event that cites it (§3.2.1
/// step 7). Its sha is computed once, at construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingArtifact {
    pub(crate) sha: Sha256Ref,
    pub(crate) bytes: Vec<u8>,
    pub(crate) canon: Canon,
    pub(crate) media_type: String,
}

impl PendingArtifact {
    pub fn sha(&self) -> &Sha256Ref {
        &self.sha
    }

    pub fn canon(&self) -> Canon {
        self.canon
    }

    pub fn byte_len(&self) -> usize {
        self.bytes.len()
    }

    /// The checks the append makes before any write (§3.2.1 step 0). A `RawUtf8` artifact
    /// must be UTF-8 and a `JcsRfc8785` artifact must already be in canonical form, so the
    /// stored tag always tells a verifier the rule that reproduces the sha.
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.bytes.len() > ARTIFACT_INLINE_MAX {
            // Object storage arrives when an artifact first exceeds the inline limit
            // (core-delta §2.5); until then such an artifact is refused, not truncated.
            return Err(format!(
                "{}: {} bytes exceeds the inline limit of {ARTIFACT_INLINE_MAX} and no object store is configured",
                self.sha,
                self.bytes.len()
            ));
        }
        if self.media_type.trim().is_empty() {
            return Err(format!("{}: empty media type", self.sha));
        }
        match self.canon {
            Canon::RawBytes => {}
            Canon::RawUtf8 => {
                if std::str::from_utf8(&self.bytes).is_err() {
                    return Err(format!("{}: raw-utf8 bytes are not UTF-8", self.sha));
                }
            }
            Canon::JcsRfc8785 => {
                let text = std::str::from_utf8(&self.bytes)
                    .map_err(|_| format!("{}: jcs bytes are not UTF-8", self.sha))?;
                let canonical = abw_canon::parse(text)
                    .and_then(|v| jcs(&v))
                    .map_err(|e| format!("{}: not canonical JSON: {e}", self.sha))?;
                if canonical != self.bytes {
                    return Err(format!("{}: bytes are not in JCS form", self.sha));
                }
            }
        }
        Ok(())
    }
}

/// A JSON artifact, stored as its JCS bytes and addressed by `sha256_jcs`.
pub fn artifact_json(v: &Value, media_type: &str) -> Result<PendingArtifact, JcsError> {
    let bytes = jcs(v)?;
    Ok(PendingArtifact {
        sha: sha256_raw(&bytes),
        bytes,
        canon: Canon::JcsRfc8785,
        media_type: media_type.to_string(),
    })
}

/// An artifact stored as the given bytes and addressed by `sha256_raw`.
pub fn artifact_bytes(bytes: Vec<u8>, canon: Canon, media_type: &str) -> PendingArtifact {
    PendingArtifact {
        sha: sha256_raw(&bytes),
        bytes,
        canon,
        media_type: media_type.to_string(),
    }
}
