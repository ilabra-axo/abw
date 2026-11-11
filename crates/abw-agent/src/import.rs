//! Importing the ported cards as agent versions (PROT-09; target-system-design §4.2
//! "Seed on insert only"; ADR-0008 §6).
//!
//! Cards are data under `agents/<slug>/agent_card.json`. Each is resolved by
//! [`crate::card::resolve`] and dispatched as `agent::PutVersion`: `Seed` for an agent the
//! tenant doesn't have, `AdoptFile` otherwise, so an unchanged card is a no-op. Field-origin
//! merging (§4.2 seed table) arrives with owner edits over HTTP [I].

use crate::card::{self, CardError};
use abw_canon::{JcsError, Sha256Ref};
use abw_ledger::agent::{self, PutVersion, VersionReason};
use abw_ledger::{CommandCtx, DispatchError, Ledger, ReadError};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// The card file inside each agent directory.
pub const CARD_FILE: &str = "agent_card.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedAgent {
    pub agent_id: String,
    pub version_number: u64,
    pub card_sha: Sha256Ref,
    pub created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportReport {
    pub agents: Vec<ImportedAgent>,
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path}: directory name is not UTF-8")]
    BadName { path: PathBuf },
    #[error("{path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: JcsError,
    },
    #[error("{slug}/{CARD_FILE} names agent_id {agent_id:?}, not its directory")]
    IdMismatch {
        slug: String,
        agent_id: Option<String>,
    },
    #[error("{slug}: {source}")]
    Card {
        slug: String,
        #[source]
        source: CardError,
    },
    #[error("reading the head of {agent_id}: {source}")]
    Read {
        agent_id: String,
        #[source]
        source: ReadError,
    },
    #[error("importing {agent_id}: {source}")]
    Dispatch {
        agent_id: String,
        #[source]
        source: DispatchError,
    },
}

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> ImportError + '_ {
    move |source| ImportError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// `(slug, raw card)` for every `<dir>/<slug>/agent_card.json`, sorted by slug. A card
/// whose `agent_id` differs from its directory name is refused.
pub fn read_dir(dir: &Path) -> Result<Vec<(String, Value)>, ImportError> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(io(dir))? {
        let entry = entry.map_err(io(dir))?;
        let path = entry.path();
        if !entry.file_type().map_err(io(&path))?.is_dir() {
            continue;
        }
        let slug = entry
            .file_name()
            .into_string()
            .map_err(|_| ImportError::BadName { path: path.clone() })?;
        let file = path.join(CARD_FILE);
        let text = std::fs::read_to_string(&file).map_err(io(&file))?;
        let raw = abw_canon::parse(&text).map_err(|source| ImportError::Parse {
            path: file.clone(),
            source,
        })?;
        let id = raw.get("agent_id").and_then(Value::as_str);
        if id != Some(slug.as_str()) {
            return Err(ImportError::IdMismatch {
                agent_id: id.map(str::to_string),
                slug,
            });
        }
        out.push((slug, raw));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// Resolves and dispatches every card in `dir` under `ctx`, in slug order.
pub async fn import_dir(
    ledger: &Ledger,
    dir: &Path,
    ctx: &CommandCtx,
) -> Result<ImportReport, ImportError> {
    let read = ledger.reader();
    let mut report = ImportReport::default();
    for (slug, raw) in read_dir(dir)? {
        let resolved = card::resolve(&raw).map_err(|source| ImportError::Card {
            slug: slug.clone(),
            source,
        })?;
        let agent_id = resolved.agent_id().to_string();
        let existing = agent::head(&read, &ctx.tenant, &agent_id)
            .await
            .map_err(|source| ImportError::Read {
                agent_id: agent_id.clone(),
                source,
            })?;
        let reason = match existing {
            None => VersionReason::Seed,
            Some(_) => VersionReason::AdoptFile,
        };
        let cmd = PutVersion::new(agent_id.clone(), resolved.json().clone(), reason);
        let v = ledger
            .dispatch(cmd, ctx)
            .await
            .map_err(|source| ImportError::Dispatch {
                agent_id: agent_id.clone(),
                source,
            })?
            .output;
        report.agents.push(ImportedAgent {
            agent_id: v.agent_id,
            version_number: v.version_number,
            card_sha: v.card_sha,
            created: v.created,
        });
    }
    Ok(report)
}
