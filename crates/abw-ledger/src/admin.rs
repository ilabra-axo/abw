//! Operator tools (target-system-design §3.4.3 "Rebuild", ADR-0004 item 5).
//!
//! `rebuild_diff` replays a projection from `ledger.events` with the projector's own fold
//! and compares the result with the live table, row by row. It runs as `abw_owner` in one
//! REPEATABLE READ, READ ONLY transaction, so events and projection are read from the same
//! snapshot and nothing is written. The design describes the replay target as a shadow
//! table; here the shadow is in memory and bodies are compared as JCS bytes, so the
//! comparison is byte-exact and needs no write (an implementation choice, not a design
//! change: the diff is the same).

use crate::probe;
use crate::registry::PROBE_RECORDED;
use serde_json::Value;
use sqlx::postgres::PgConnectOptions;
use sqlx::types::Json;
use sqlx::{Connection, PgConnection, Row};
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    #[error("invalid database URL: {0}")]
    Url(#[source] sqlx::Error),
    #[error("{what}: {source}")]
    Db {
        what: &'static str,
        #[source]
        source: sqlx::Error,
    },
    #[error("unknown projection {0:?}")]
    UnknownProjection(String),
    #[error("could not canonicalise a stored value: {0}")]
    Canon(#[from] abw_canon::JcsError),
    #[error("stored value out of range: {0}")]
    Range(String),
}

fn db(what: &'static str) -> impl FnOnce(sqlx::Error) -> AdminError {
    move |source| AdminError::Db { what, source }
}

/// One row that differs between the live projection and its rebuild.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowDiff {
    pub key: String,
    /// The live row, canonicalised; `None` if the rebuild has it and the table does not.
    pub live: Option<String>,
    /// The rebuilt row, canonicalised; `None` if the table has it and the rebuild does not.
    pub rebuilt: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectionDiff {
    pub projection: String,
    pub rows: Vec<RowDiff>,
}

impl ProjectionDiff {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// Rebuilds `projection` (e.g. `"proj.probes"`) from events and diffs it with the table.
pub async fn rebuild_diff(owner_url: &str, projection: &str) -> Result<ProjectionDiff, AdminError> {
    if projection != "proj.probes" {
        return Err(AdminError::UnknownProjection(projection.to_string()));
    }
    let opts = PgConnectOptions::from_str(owner_url).map_err(AdminError::Url)?;
    let mut conn = PgConnection::connect_with(&opts)
        .await
        .map_err(db("connect as owner"))?;
    let mut tx = conn.begin().await.map_err(db("begin"))?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(db("set the snapshot"))?;
    let diff = probes_diff(&mut tx).await;
    tx.rollback().await.map_err(db("end the snapshot"))?;
    conn.close().await.map_err(db("close"))?;
    diff
}

async fn probes_diff(conn: &mut PgConnection) -> Result<ProjectionDiff, AdminError> {
    // Replay: the projector's fold over its event types, per stream in seq order.
    let events = sqlx::query(
        "SELECT stream_id, seq, global_pos, payload FROM ledger.events
          WHERE event_type = $1 AND schema_version = $2
          ORDER BY stream_id, seq",
    )
    .bind(PROBE_RECORDED.event_type)
    .bind(PROBE_RECORDED.schema_version)
    .fetch_all(&mut *conn)
    .await
    .map_err(db("read events"))?;
    let mut rebuilt: BTreeMap<String, probe::ProbeRow> = BTreeMap::new();
    for e in events {
        let stream_id: String = e.try_get("stream_id").map_err(db("read events"))?;
        let seq: i64 = e.try_get("seq").map_err(db("read events"))?;
        let seq = u64::try_from(seq).map_err(|_| AdminError::Range(format!("seq {seq}")))?;
        let pos: i64 = e.try_get("global_pos").map_err(db("read events"))?;
        let payload: Json<Value> = e.try_get("payload").map_err(db("read events"))?;
        let row = probe::apply(rebuilt.get(&stream_id), seq, &payload.0, pos);
        rebuilt.insert(stream_id, row);
    }

    let live_rows = sqlx::query("SELECT stream_id, seq, body, applied_pos FROM proj.probes")
        .fetch_all(&mut *conn)
        .await
        .map_err(db("read proj.probes"))?;
    let mut live: BTreeMap<String, probe::ProbeRow> = BTreeMap::new();
    for r in live_rows {
        let seq: i64 = r.try_get("seq").map_err(db("read proj.probes"))?;
        live.insert(
            r.try_get("stream_id").map_err(db("read proj.probes"))?,
            probe::ProbeRow {
                seq: u64::try_from(seq).map_err(|_| AdminError::Range(format!("seq {seq}")))?,
                body: r
                    .try_get::<Json<Value>, _>("body")
                    .map_err(db("read proj.probes"))?
                    .0,
                applied_pos: r.try_get("applied_pos").map_err(db("read proj.probes"))?,
            },
        );
    }

    let mut keys: Vec<&String> = rebuilt.keys().chain(live.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut rows = Vec::new();
    for key in keys {
        let l = live.get(key).map(canonical_row).transpose()?;
        let b = rebuilt.get(key).map(canonical_row).transpose()?;
        if l != b {
            rows.push(RowDiff {
                key: key.clone(),
                live: l,
                rebuilt: b,
            });
        }
    }
    Ok(ProjectionDiff {
        projection: "proj.probes".to_string(),
        rows,
    })
}

fn canonical_row(r: &probe::ProbeRow) -> Result<String, AdminError> {
    let body = abw_canon::jcs(&r.body)?;
    Ok(format!(
        "seq={} applied_pos={} body={}",
        r.seq,
        r.applied_pos,
        String::from_utf8_lossy(&body)
    ))
}
