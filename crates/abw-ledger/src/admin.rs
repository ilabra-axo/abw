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
use crate::rows::{self, EventView};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::postgres::PgConnectOptions;
use sqlx::types::Json;
use sqlx::{Connection, PgConnection, Row};
use std::collections::BTreeMap;
use std::str::FromStr;

/// The projections `rebuild_diff` accepts (ADR-0004 item 5, ADR-0008 §3).
const PROJECTIONS: &[&str] = &[
    "proj.probes",
    "proj.allowances",
    "proj.allowance_holds",
    "proj.episodes",
    "proj.agent_versions",
    "proj.agent_heads",
];

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
    /// The fold refused the stored events, or a live row didn't decode.
    #[error("replay failed: {0}")]
    Replay(String),
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
    if !PROJECTIONS.contains(&projection) {
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
    let diff = match projection {
        "proj.probes" => probes_diff(&mut tx).await,
        other => m2_diff(&mut tx, other).await,
    };
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

/// One stored event with the stream's tenant, the input of every M2 fold.
struct StoredRow {
    stream_id: String,
    global_pos: i64,
    event_type: String,
    payload: Value,
    tenant: String,
    actor: String,
}

impl StoredRow {
    fn view(&self) -> EventView<'_> {
        EventView {
            stream_id: &self.stream_id,
            global_pos: self.global_pos,
            event_type: &self.event_type,
            payload: &self.payload,
            tenant: &self.tenant,
            actor: &self.actor,
        }
    }
}

fn replay(e: String) -> AdminError {
    AdminError::Replay(e)
}

async fn m2_diff(conn: &mut PgConnection, projection: &str) -> Result<ProjectionDiff, AdminError> {
    let rows = sqlx::query(
        "SELECT e.stream_id, e.global_pos, e.event_type, e.payload, e.actor, s.tenant_id
           FROM ledger.events e JOIN ledger.streams s ON s.stream_id = e.stream_id
          WHERE s.kind IN ('allowance', 'episode', 'agent')
          ORDER BY e.global_pos",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(db("read events"))?;
    let mut events = Vec::with_capacity(rows.len());
    for r in rows {
        events.push(StoredRow {
            stream_id: r.try_get("stream_id").map_err(db("read events"))?,
            global_pos: r.try_get("global_pos").map_err(db("read events"))?,
            event_type: r.try_get("event_type").map_err(db("read events"))?,
            payload: r
                .try_get::<Json<Value>, _>("payload")
                .map_err(db("read events"))?
                .0,
            tenant: r.try_get("tenant_id").map_err(db("read events"))?,
            actor: r.try_get("actor").map_err(db("read events"))?,
        });
    }

    let (rebuilt, live) = match projection {
        "proj.allowances" => {
            let mut m: BTreeMap<String, rows::AllowanceRow> = BTreeMap::new();
            for e in &events {
                let v = e.view();
                if let Some(k) = rows::allowance_key(&v)
                    && let Some(row) = rows::apply_allowance(m.get(&k), &v).map_err(replay)?
                {
                    m.insert(k, row);
                }
            }
            let live = live_rows(
                conn,
                "SELECT to_jsonb(t) FROM proj.allowances t",
                |r: &rows::AllowanceRow| r.owner_id.clone(),
            )
            .await?;
            (canonical_map(&m)?, live)
        }
        "proj.allowance_holds" => {
            let mut m: BTreeMap<String, rows::HoldRow> = BTreeMap::new();
            for e in &events {
                let v = e.view();
                if let Some(k) = rows::hold_key(&v).map_err(replay)? {
                    let k = k.to_string();
                    if let Some(row) = rows::apply_hold(m.get(&k), &v).map_err(replay)? {
                        m.insert(k, row);
                    }
                }
            }
            let live = live_rows(
                conn,
                "SELECT to_jsonb(t) FROM proj.allowance_holds t",
                |r: &rows::HoldRow| r.hold_id.to_string(),
            )
            .await?;
            (canonical_map(&m)?, live)
        }
        "proj.episodes" => {
            let mut m: BTreeMap<String, rows::EpisodeRow> = BTreeMap::new();
            for e in &events {
                let v = e.view();
                if let Some(k) = rows::episode_key(&v) {
                    let k = k.to_string();
                    if let Some(row) = rows::apply_episode(m.get(&k), &v).map_err(replay)? {
                        m.insert(k, row);
                    }
                }
            }
            let live = live_rows(
                conn,
                "SELECT to_jsonb(t) FROM proj.episodes t",
                |r: &rows::EpisodeRow| r.episode_id.to_string(),
            )
            .await?;
            (canonical_map(&m)?, live)
        }
        "proj.agent_versions" => {
            let mut m: BTreeMap<String, rows::AgentVersionRow> = BTreeMap::new();
            for e in &events {
                if let Some(row) = rows::agent_version(&e.view()).map_err(replay)? {
                    m.insert(format!("{}/{}", row.agent_id, row.version_number), row);
                }
            }
            let live = live_rows(
                conn,
                "SELECT to_jsonb(t) FROM proj.agent_versions t",
                |r: &rows::AgentVersionRow| format!("{}/{}", r.agent_id, r.version_number),
            )
            .await?;
            (canonical_map(&m)?, live)
        }
        "proj.agent_heads" => {
            let mut m: BTreeMap<String, rows::AgentHeadRow> = BTreeMap::new();
            for e in &events {
                let v = e.view();
                if let Some(k) = rows::agent_key(&v)
                    && let Some(row) = rows::apply_agent_head(m.get(&k), &v).map_err(replay)?
                {
                    m.insert(k, row);
                }
            }
            let live = live_rows(
                conn,
                "SELECT to_jsonb(t) FROM proj.agent_heads t",
                |r: &rows::AgentHeadRow| r.agent_id.clone(),
            )
            .await?;
            (canonical_map(&m)?, live)
        }
        other => return Err(AdminError::UnknownProjection(other.to_string())),
    };
    Ok(ProjectionDiff {
        projection: projection.to_string(),
        rows: diff_maps(&live, &rebuilt),
    })
}

/// The live table, each row decoded into its fold's row type and canonicalised the same
/// way as the rebuild, so representation differences (timestamp offsets, key order) don't
/// count and value differences do.
async fn live_rows<R: DeserializeOwned + Serialize>(
    conn: &mut PgConnection,
    sql: &'static str,
    key: impl Fn(&R) -> String,
) -> Result<BTreeMap<String, String>, AdminError> {
    let docs: Vec<Json<Value>> = sqlx::query_scalar(sql)
        .fetch_all(&mut *conn)
        .await
        .map_err(db("read the projection"))?;
    let mut out = BTreeMap::new();
    for d in docs {
        let row: R = serde_json::from_value(d.0).map_err(|e| AdminError::Replay(e.to_string()))?;
        out.insert(key(&row), canonical(&row)?);
    }
    Ok(out)
}

fn canonical<R: Serialize>(row: &R) -> Result<String, AdminError> {
    let v = serde_json::to_value(row).map_err(|e| AdminError::Replay(e.to_string()))?;
    Ok(String::from_utf8_lossy(&abw_canon::jcs(&v)?).into_owned())
}

fn canonical_map<R: Serialize>(
    m: &BTreeMap<String, R>,
) -> Result<BTreeMap<String, String>, AdminError> {
    m.iter()
        .map(|(k, r)| Ok((k.clone(), canonical(r)?)))
        .collect()
}

fn diff_maps(live: &BTreeMap<String, String>, rebuilt: &BTreeMap<String, String>) -> Vec<RowDiff> {
    let mut keys: Vec<&String> = live.keys().chain(rebuilt.keys()).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .filter(|k| live.get(*k) != rebuilt.get(*k))
        .map(|k| RowDiff {
            key: k.clone(),
            live: live.get(k).cloned(),
            rebuilt: rebuilt.get(k).cloned(),
        })
        .collect()
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
