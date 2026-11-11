//! Inline projections (target-system-design §3.4, step 9 of §3.2.1).
//!
//! An inline projector runs inside the append transaction, after the events are inserted,
//! and writes its rows citing a `global_pos` appended by this transaction. The database
//! checks that citation at COMMIT (`proj.row_justified_by_event`, T3); [`ProjTx`] checks it
//! first in Rust, so a projector bug surfaces as a typed error instead of a failed commit.
//! The M2 folds are pure functions in `rows.rs`, shared with rebuild.

use crate::command::Recorded;
use crate::error::DispatchError;
use crate::policy::{Adjusted, stored_tenant};
use crate::probe;
use crate::registry::{POLICY_ADJUSTED, PROBE_RECORDED};
use crate::rows::{self, EventView};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{Encode, PgConnection, Postgres, Type};

/// The append transaction as seen by a projector (§3.8 `ProjTx`): obtainable only inside
/// dispatch after step 8, and exposing only writer functions keyed by a position this
/// transaction appended.
pub(crate) struct ProjTx<'t> {
    conn: &'t mut PgConnection,
    appended: &'t [Recorded],
}

const ALLOWANCE_LOAD: &str = "SELECT to_jsonb(t) FROM proj.allowances t WHERE owner_id = $1";
const ALLOWANCE_UPSERT: &str = "INSERT INTO proj.allowances
       SELECT * FROM jsonb_populate_record(NULL::proj.allowances, $1)
     ON CONFLICT (owner_id) DO UPDATE SET
       (free_tier, granted, purchased, held, available, total_granted_free_tier,
        total_granted, total_purchased, total_consumed, applied_pos)
     = (EXCLUDED.free_tier, EXCLUDED.granted, EXCLUDED.purchased, EXCLUDED.held,
        EXCLUDED.available, EXCLUDED.total_granted_free_tier, EXCLUDED.total_granted,
        EXCLUDED.total_purchased, EXCLUDED.total_consumed, EXCLUDED.applied_pos)";

const HOLD_LOAD: &str = "SELECT to_jsonb(t) FROM proj.allowance_holds t WHERE hold_id = $1";
const HOLD_UPSERT: &str = "INSERT INTO proj.allowance_holds
       SELECT * FROM jsonb_populate_record(NULL::proj.allowance_holds, $1)
     ON CONFLICT (hold_id) DO UPDATE SET (status, applied_pos)
     = (EXCLUDED.status, EXCLUDED.applied_pos)";

const EPISODE_LOAD: &str = "SELECT to_jsonb(t) FROM proj.episodes t WHERE episode_id = $1";
const EPISODE_UPSERT: &str = "INSERT INTO proj.episodes
       SELECT * FROM jsonb_populate_record(NULL::proj.episodes, $1)
     ON CONFLICT (episode_id) DO UPDATE SET
       (consumed_cu, status, reason_class, raw_output_sha, tool_call_shas, closed_pos,
        applied_pos)
     = (EXCLUDED.consumed_cu, EXCLUDED.status, EXCLUDED.reason_class, EXCLUDED.raw_output_sha,
        EXCLUDED.tool_call_shas, EXCLUDED.closed_pos, EXCLUDED.applied_pos)";

const HEAD_LOAD: &str = "SELECT to_jsonb(t) FROM proj.agent_heads t WHERE agent_id = $1";
const HEAD_UPSERT: &str = "INSERT INTO proj.agent_heads
       SELECT * FROM jsonb_populate_record(NULL::proj.agent_heads, $1)
     ON CONFLICT (agent_id) DO UPDATE SET (version_number, card_sha, retired, applied_pos)
     = (EXCLUDED.version_number, EXCLUDED.card_sha, EXCLUDED.retired, EXCLUDED.applied_pos)";

const VERSION_INSERT: &str = "INSERT INTO proj.agent_versions
       SELECT * FROM jsonb_populate_record(NULL::proj.agent_versions, $1)";

impl<'t> ProjTx<'t> {
    pub(crate) fn new(conn: &'t mut PgConnection, appended: &'t [Recorded]) -> ProjTx<'t> {
        ProjTx { conn, appended }
    }

    fn cite(&self, pos: i64) -> Result<(), DispatchError> {
        if self.appended.iter().any(|r| r.global_pos == pos) {
            Ok(())
        } else {
            Err(DispatchError::Integrity(format!(
                "projector cited event {pos}, which this transaction did not append"
            )))
        }
    }

    /// Upserts the `proj.probes` row of `stream_id`.
    async fn write_probe(
        &mut self,
        stream_id: &str,
        row: &probe::ProbeRow,
    ) -> Result<(), DispatchError> {
        self.cite(row.applied_pos)?;
        let seq = i64::try_from(row.seq)
            .map_err(|_| DispatchError::Integrity(format!("seq {} out of range", row.seq)))?;
        sqlx::query(
            "INSERT INTO proj.probes (stream_id, seq, body, applied_pos) VALUES ($1, $2, $3, $4)
             ON CONFLICT (stream_id) DO UPDATE
               SET seq = EXCLUDED.seq, body = EXCLUDED.body, applied_pos = EXCLUDED.applied_pos",
        )
        .bind(stream_id)
        .bind(seq)
        .bind(Json(&row.body))
        .bind(row.applied_pos)
        .execute(&mut *self.conn)
        .await?;
        Ok(())
    }

    /// The current row of a projection, by key.
    async fn load<K, R>(&mut self, sql: &'static str, key: K) -> Result<Option<R>, DispatchError>
    where
        K: for<'q> Encode<'q, Postgres> + Type<Postgres> + Send,
        R: DeserializeOwned,
    {
        let row: Option<Json<Value>> = sqlx::query_scalar(sql)
            .bind(key)
            .fetch_optional(&mut *self.conn)
            .await?;
        row.map(|r| {
            serde_json::from_value(r.0)
                .map_err(|e| DispatchError::Integrity(format!("projection row: {e}")))
        })
        .transpose()
    }

    /// Writes `row`, which cites the event at `pos`.
    async fn write<R: Serialize>(
        &mut self,
        sql: &'static str,
        row: &R,
        pos: i64,
    ) -> Result<(), DispatchError> {
        self.cite(pos)?;
        let doc = serde_json::to_value(row)
            .map_err(|e| DispatchError::Integrity(format!("projection row: {e}")))?;
        sqlx::query(sql)
            .bind(Json(doc))
            .execute(&mut *self.conn)
            .await?;
        Ok(())
    }

    /// §3.1.7: the `ledger.policy_versions` row of a `policy.adjusted`, in its transaction.
    async fn write_policy_version(&mut self, rec: &Recorded) -> Result<(), DispatchError> {
        self.cite(rec.global_pos)?;
        let a: Adjusted = serde_json::from_value(rec.payload.clone())
            .map_err(|e| DispatchError::Integrity(format!("policy.adjusted: {e}")))?;
        let version = i32::try_from(a.version)
            .map_err(|_| DispatchError::Integrity(format!("policy version {}", a.version)))?;
        sqlx::query(
            "INSERT INTO ledger.policy_versions
               (policy_id, version, policy_kind, tenant_id, doc, doc_sha, supersedes,
                accepted_by, rationale, active_from, event_pos)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(a.policy_id)
        .bind(version)
        .bind(a.policy_kind.as_str())
        .bind(stored_tenant(&rec.tenant))
        .bind(Json(&a.doc))
        .bind(a.doc_sha.as_str())
        .bind((version > 1).then_some(version - 1))
        .bind(&a.accepted_by)
        .bind(&a.rationale)
        .bind(a.active_from)
        .bind(rec.global_pos)
        .execute(&mut *self.conn)
        .await?;
        Ok(())
    }
}

fn integrity(e: String) -> DispatchError {
    DispatchError::Integrity(e)
}

/// Applies every inline projector that handles `rec.event_type`.
pub(crate) async fn apply_inline(tx: &mut ProjTx<'_>, rec: &Recorded) -> Result<(), DispatchError> {
    if rec.event_type == PROBE_RECORDED.event_type {
        // probe.recorded replaces the whole row, so the fold needs no previous state.
        let row = probe::apply(None, rec.seq, &rec.payload, rec.global_pos);
        tx.write_probe(&rec.stream_id, &row).await?;
        return Ok(());
    }
    if rec.event_type == POLICY_ADJUSTED.event_type {
        return tx.write_policy_version(rec).await;
    }
    let ev = EventView {
        stream_id: &rec.stream_id,
        global_pos: rec.global_pos,
        event_type: rec.event_type,
        payload: &rec.payload,
        tenant: &rec.tenant,
        actor: &rec.actor,
    };
    let pos = rec.global_pos;

    if let Some(owner) = rows::allowance_key(&ev) {
        let prev: Option<rows::AllowanceRow> = tx.load(ALLOWANCE_LOAD, owner).await?;
        if let Some(row) = rows::apply_allowance(prev.as_ref(), &ev).map_err(integrity)? {
            tx.write(ALLOWANCE_UPSERT, &row, pos).await?;
        }
    }
    if let Some(hold_id) = rows::hold_key(&ev).map_err(integrity)? {
        let prev: Option<rows::HoldRow> = tx.load(HOLD_LOAD, hold_id).await?;
        if let Some(row) = rows::apply_hold(prev.as_ref(), &ev).map_err(integrity)? {
            tx.write(HOLD_UPSERT, &row, pos).await?;
        }
    }
    if let Some(episode_id) = rows::episode_key(&ev) {
        let prev: Option<rows::EpisodeRow> = tx.load(EPISODE_LOAD, episode_id).await?;
        if let Some(row) = rows::apply_episode(prev.as_ref(), &ev).map_err(integrity)? {
            tx.write(EPISODE_UPSERT, &row, pos).await?;
        }
    }
    if let Some(row) = rows::agent_version(&ev).map_err(integrity)? {
        tx.write(VERSION_INSERT, &row, pos).await?;
    }
    if let Some(agent_id) = rows::agent_key(&ev) {
        let prev: Option<rows::AgentHeadRow> = tx.load(HEAD_LOAD, agent_id).await?;
        if let Some(row) = rows::apply_agent_head(prev.as_ref(), &ev).map_err(integrity)? {
            tx.write(HEAD_UPSERT, &row, pos).await?;
        }
    }
    Ok(())
}
