//! The episode and hold reaper (target-system-design §4.5 ⟨6⟩, §4.7; economy.md §5 step 4;
//! ADR-0008 §4 `reaper`).
//!
//! One pass closes every episode still running `EPISODE_MAX_RUNTIME` after it opened, with
//! `episode::Abandon { AbandonedTimeout }` at its current head: the episode closes `failure`
//! and its whole hold is released in one transaction. It is a callable function, not a
//! background task (ADR-0008 §1 item 9). When the run's own close, another reaper or the
//! closer wins the race, the abandon gets `Conflict` or `EpisodeClosed` and writes nothing
//! (§4.7 "mutually safe").

use crate::boot::Ledger;
use crate::command::Expected;
use crate::ctx::{Actor, CommandCtx};
use crate::episode::{Abandon, ReasonClass};
use crate::error::{DispatchError, ReadError, Rejection};
use crate::read_query;
use chrono::{DateTime, Utc};
use sqlx::Row;
use std::time::Duration;
use uuid::Uuid;

/// How long an episode may run before the reaper closes it (§4.5 ⟨6⟩, default 30 min).
pub const EPISODE_MAX_RUNTIME: Duration = Duration::from_secs(30 * 60);

/// The actor id the reaper's abandonments are recorded under.
const REAPER_ACTOR: &str = "episode-reaper";

/// What one pass did: episodes it closed, and episodes another recoverer closed first.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReaperReport {
    pub abandoned: Vec<Uuid>,
    pub raced: Vec<Uuid>,
}

#[derive(Debug, thiserror::Error)]
pub enum ReaperError {
    #[error("reading running episodes: {0}")]
    Read(#[from] ReadError),
    #[error("abandoning episode {episode_id}: {source}")]
    Dispatch {
        episode_id: Uuid,
        #[source]
        source: DispatchError,
    },
    #[error("value out of range: {0}")]
    Range(String),
}

/// One reaper pass at `now`.
pub async fn run_once(ledger: &Ledger, now: DateTime<Utc>) -> Result<ReaperReport, ReaperError> {
    let max = chrono::Duration::from_std(EPISODE_MAX_RUNTIME)
        .map_err(|e| ReaperError::Range(e.to_string()))?;
    let cutoff = now - max;
    let rows = ledger
        .reader()
        .fetch_all(
            read_query!(
                "SELECT e.episode_id, e.tenant_id, s.head
                   FROM proj.episodes e JOIN ledger.streams s ON s.stream_id = e.stream_id
                  WHERE e.status = 'running' AND e.opened_at < $1
                  ORDER BY e.opened_pos"
            )
            .bind(cutoff),
        )
        .await?;
    let mut report = ReaperReport::default();
    for r in rows {
        let episode_id: Uuid = r.try_get("episode_id").map_err(ReadError::Db)?;
        let tenant: String = r.try_get("tenant_id").map_err(ReadError::Db)?;
        let head: i64 = r.try_get("head").map_err(ReadError::Db)?;
        let head = u64::try_from(head).map_err(|_| ReaperError::Range(format!("head {head}")))?;
        let ctx = CommandCtx::new(tenant, Actor::platform(REAPER_ACTOR));
        let cmd = Abandon::new(
            episode_id,
            Expected::Exact(head),
            ReasonClass::AbandonedTimeout,
        );
        match ledger.dispatch(cmd, &ctx).await {
            Ok(d) if !d.replayed => report.abandoned.push(episode_id),
            Ok(_)
            | Err(DispatchError::Conflict { .. })
            | Err(DispatchError::Rejected(Rejection::EpisodeClosed { .. })) => {
                report.raced.push(episode_id);
            }
            Err(source) => return Err(ReaperError::Dispatch { episode_id, source }),
        }
    }
    Ok(report)
}
