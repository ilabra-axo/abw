//! `Ledger::dispatch`: the append algorithm of target-system-design §3.2.1.
//!
//! One READ COMMITTED transaction per attempt. Steps, as numbered in §3.2.1:
//!
//! | Step | Here |
//! | --- | --- |
//! | 0 | `validate`: pure; no connection is acquired before it |
//! | 1 | `BEGIN ISOLATION LEVEL READ COMMITTED` |
//! | 2 | idempotency lookup in `ledger.command_dedup` (replay or `IdempotencyMismatch`) |
//! | 3 | `ledger.lock_streams(W, R)` |
//! | 4 | fold: nothing in M1 (the probe decides from heads alone) |
//! | 5 | `decide`, then the expected-version check against the locked heads |
//! | 6a/6b | new stream rows / head compare-and-swap |
//! | 7 | artifacts, `ON CONFLICT (sha) DO NOTHING`, citing pre-allocated event positions |
//! | 8 | events |
//! | 9 | inline projections |
//! | 10 | provenance: none in M1 |
//! | 11 | outbox, `ON CONFLICT (dedupe_key) DO NOTHING` |
//! | 12 | the dedup row with the final result, `ON CONFLICT DO NOTHING` |
//! | 14 | `pg_notify('abw_events', '<stream_id>:<head>')` |
//! | 13 | `COMMIT`: deferred gap-free, justification and FK checks run here |
//!
//! Step 2 is a lookup and the dedup row is written at step 12, the option §3.2.1 step 12
//! allows ("step 2 is deferred to here as INSERT … ON CONFLICT DO NOTHING with the final
//! result"), because the result (`global_pos`) is only known after step 8 and the dedup row
//! is immutable. Two concurrent attempts with one key are then serialised by the stream
//! lock or by the dedup key's unique-index entry; the loser rolls back and replays the
//! winner's stored result.

use crate::boot::Ledger;
use crate::command::{Command, Dispatched, Heads, Recorded, StreamAppend};
use crate::ctx::CommandCtx;
use crate::error::{DispatchError, Rejection};
use crate::fault::FaultPoint;
use crate::projection::{ProjTx, apply_inline};
use abw_canon::{Sha256Ref, sha256_jcs};
use serde_json::{Value, json};
use sqlx::types::Json;
use sqlx::{PgConnection, Row};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

/// What one attempt produced.
enum Attempt<O> {
    Committed(O),
    /// The dedup row already exists: replay it, or refuse on a different request.
    Replay(StoredResult),
}

struct StoredResult {
    command_type: String,
    request_sha: String,
    result: Value,
}

struct Keyed<'a> {
    key: &'a str,
    request_sha: Sha256Ref,
}

impl Ledger {
    /// Validates, decides and appends `cmd` in one transaction (§3.2.1).
    pub async fn dispatch<C: Command>(
        &self,
        cmd: C,
        ctx: &CommandCtx,
    ) -> Result<Dispatched<C::Output>, DispatchError> {
        // Step 0: pure checks.
        validate_ctx(ctx).map_err(DispatchError::Rejected)?;
        cmd.validate(ctx).map_err(DispatchError::Rejected)?;
        let keyed = match cmd.idempotency_key() {
            Some(key) => Some(Keyed {
                key,
                request_sha: request_sha::<C>(&cmd, ctx)?,
            }),
            None => None,
        };

        match self.attempt(&cmd, ctx, keyed.as_ref()).await {
            Ok(Attempt::Committed(output)) => Ok(Dispatched {
                output,
                replayed: false,
            }),
            Ok(Attempt::Replay(stored)) => match &keyed {
                Some(k) => replay::<C>(k, stored),
                None => Err(DispatchError::Integrity(
                    "a dedup row was found for a command without a key".to_string(),
                )),
            },
            // A concurrent attempt with the same key may have won the stream race and
            // committed its dedup row: the retry then returns that result (§3.10).
            Err(conflict @ DispatchError::Conflict { .. }) => match &keyed {
                Some(k) => match self.lookup(k.key).await? {
                    Some(stored) => replay::<C>(k, stored),
                    None => Err(conflict),
                },
                None => Err(conflict),
            },
            Err(e) => Err(e),
        }
    }

    async fn attempt<C: Command>(
        &self,
        cmd: &C,
        ctx: &CommandCtx,
        keyed: Option<&Keyed<'_>>,
    ) -> Result<Attempt<C::Output>, DispatchError> {
        // Step 1.
        let mut tx = self.pool.begin().await?;
        let result = self.steps(&mut tx, cmd, ctx, keyed).await;
        match result {
            Ok(Attempt::Committed(output)) => {
                // Step 13. Deferred-trigger refusals surface here as Integrity.
                tx.commit().await?;
                Ok(Attempt::Committed(output))
            }
            other => match tx.rollback().await {
                Ok(()) => other,
                Err(rollback) => match other {
                    Err(cause) => Err(DispatchError::RollbackFailed {
                        cause: Box::new(cause),
                        rollback,
                    }),
                    Ok(_) => Err(DispatchError::Db(rollback)),
                },
            },
        }
    }

    async fn steps<C: Command>(
        &self,
        conn: &mut PgConnection,
        cmd: &C,
        ctx: &CommandCtx,
        keyed: Option<&Keyed<'_>>,
    ) -> Result<Attempt<C::Output>, DispatchError> {
        sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
            .execute(&mut *conn)
            .await?;

        // Step 2.
        if let Some(k) = keyed
            && let Some(stored) = fetch_dedup(&mut *conn, k.key).await?
        {
            return Ok(Attempt::Replay(stored));
        }
        self.reached(FaultPoint::AfterDedup)?;

        // Step 3.
        let write_set = cmd.write_set();
        let read_set = cmd.read_set();
        let rows = sqlx::query("SELECT stream_id, head FROM ledger.lock_streams($1, $2)")
            .bind(&write_set)
            .bind(&read_set)
            .fetch_all(&mut *conn)
            .await?;
        let mut heads = Heads::default();
        for row in rows {
            let id: String = row.try_get("stream_id")?;
            let head: i64 = row.try_get("head")?;
            heads.0.insert(id, to_u64(head)?);
        }
        self.reached(FaultPoint::AfterLock)?;
        check_tenancy(&mut *conn, &heads, ctx).await?;

        // Step 4 (fold) has nothing to read in M1. Step 5.
        let appends = cmd.decide(&heads, ctx)?;
        let writable: BTreeSet<&str> = write_set.iter().map(String::as_str).collect();
        for a in &appends {
            if !writable.contains(a.stream_id.as_str()) {
                return Err(DispatchError::Integrity(format!(
                    "{} appends to {}, which is not in its write set",
                    C::TYPE,
                    a.stream_id
                )));
            }
            let actual = heads.head(&a.stream_id);
            if actual != a.expected.head() {
                return Err(DispatchError::Conflict {
                    stream: a.stream_id.clone(),
                    expected: a.expected,
                    actual,
                });
            }
        }

        // Step 6.
        let mut new_heads = BTreeMap::new();
        for a in &appends {
            let head = advance_head(&mut *conn, a, &ctx.tenant).await?;
            new_heads.insert(a.stream_id.clone(), head);
        }
        self.reached(FaultPoint::AfterHeads)?;

        // Positions for the events of step 8, allocated now so the artifacts of step 7
        // can cite them; `created_pos` is a deferred FK resolved at COMMIT (§3.1.5).
        let count: usize = appends.iter().map(|a| a.events.len()).sum();
        let mut positions = allocate_positions(&mut *conn, count).await?.into_iter();

        let command_id = Uuid::new_v4();
        let mut planned = Vec::with_capacity(count);
        for a in &appends {
            let base = a.expected.head();
            for (i, e) in a.events.iter().enumerate() {
                let pos = positions.next().ok_or_else(|| {
                    DispatchError::Integrity("fewer positions than events".to_string())
                })?;
                planned.push((a, e, base + 1 + i as u64, pos));
            }
        }

        // Step 7.
        for (_, e, _, pos) in &planned {
            for art in &e.artifacts {
                sqlx::query(
                    "INSERT INTO ledger.artifacts
                       (sha, media_type, canonicalization, byte_len, bytes, created_pos)
                     VALUES ($1, $2, $3, $4, $5, $6)
                     ON CONFLICT (sha) DO NOTHING",
                )
                .bind(art.sha.as_str())
                .bind(&art.media_type)
                .bind(art.canon.as_str())
                .bind(to_i64(art.bytes.len())?)
                .bind(&art.bytes)
                .bind(*pos)
                .execute(&mut *conn)
                .await?;
            }
        }
        self.reached(FaultPoint::AfterArtifacts)?;

        // Step 8.
        let mut recorded = Vec::with_capacity(count);
        for (a, e, seq, pos) in &planned {
            sqlx::query(
                "INSERT INTO ledger.events
                   (global_pos, stream_id, seq, event_type, schema_version, payload, payload_sha,
                    command_id, correlation_id, causation_pos, actor, actor_kind)
                 OVERRIDING SYSTEM VALUE
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NULL, $10, $11)",
            )
            .bind(*pos)
            .bind(&a.stream_id)
            .bind(to_i64_u(*seq)?)
            .bind(e.def.event_type)
            .bind(e.def.schema_version)
            .bind(Json(&e.payload))
            .bind(e.payload_sha.as_str())
            .bind(command_id)
            .bind(ctx.correlation_id)
            .bind(&ctx.actor.id)
            .bind(ctx.actor.kind.as_str())
            .execute(&mut *conn)
            .await?;
            recorded.push(Recorded {
                stream_id: a.stream_id.clone(),
                seq: *seq,
                global_pos: *pos,
                event_type: e.def.event_type,
                payload: e.payload.clone(),
                payload_sha: e.payload_sha.clone(),
            });
        }
        self.reached(FaultPoint::AfterEvents)?;

        // Step 9.
        {
            let mut ptx = ProjTx::new(&mut *conn, &recorded);
            for rec in &recorded {
                apply_inline(&mut ptx, rec).await?;
            }
        }
        self.reached(FaultPoint::AfterProjections)?;

        // Step 10: no provenance writes in M1. Step 11.
        for (_, e, _, pos) in &planned {
            for job in &e.outbox {
                sqlx::query(
                    "INSERT INTO ledger.outbox (topic, dedupe_key, source_pos, payload)
                     VALUES ($1, $2, $3, $4)
                     ON CONFLICT (dedupe_key) DO NOTHING",
                )
                .bind(&job.topic)
                .bind(&job.dedupe_key)
                .bind(*pos)
                .bind(Json(&job.payload))
                .execute(&mut *conn)
                .await?;
            }
        }
        self.reached(FaultPoint::AfterOutbox)?;

        // Step 12.
        let output = cmd.output(&recorded)?;
        if let Some(k) = keyed {
            let result = serde_json::to_value(&output).map_err(|e| {
                DispatchError::Integrity(format!("{} output does not serialise: {e}", C::TYPE))
            })?;
            let inserted = sqlx::query(
                "INSERT INTO ledger.command_dedup
                   (idempotency_key, command_type, request_sha, first_pos, result)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (idempotency_key) DO NOTHING
                 RETURNING 1",
            )
            .bind(k.key)
            .bind(C::TYPE)
            .bind(k.request_sha.as_str())
            .bind(recorded.first().map(|r| r.global_pos))
            .bind(Json(&result))
            .fetch_optional(&mut *conn)
            .await?;
            if inserted.is_none() {
                // A concurrent attempt with this key committed first.
                return match fetch_dedup(&mut *conn, k.key).await? {
                    Some(stored) => Ok(Attempt::Replay(stored)),
                    None => Err(DispatchError::Integrity(format!(
                        "dedup key {:?} conflicted but no row is visible",
                        k.key
                    ))),
                };
            }
        }

        // Step 14, inside the transaction so it is delivered only on commit.
        for (stream, head) in &new_heads {
            sqlx::query("SELECT pg_notify('abw_events', $1)")
                .bind(format!("{stream}:{head}"))
                .execute(&mut *conn)
                .await?;
        }
        self.reached(FaultPoint::BeforeCommit)?;
        Ok(Attempt::Committed(output))
    }

    async fn lookup(&self, key: &str) -> Result<Option<StoredResult>, DispatchError> {
        let mut conn = self.pool.acquire().await?;
        fetch_dedup(&mut conn, key).await
    }

    /// Aborts here if a fault was injected at `point` (ADR-0004 item 4).
    #[cfg(feature = "fault-injection")]
    fn reached(&self, point: FaultPoint) -> Result<(), DispatchError> {
        if self.fault == Some(point) {
            Err(DispatchError::Injected(point))
        } else {
            Ok(())
        }
    }

    #[cfg(not(feature = "fault-injection"))]
    #[inline(always)]
    fn reached(&self, _point: FaultPoint) -> Result<(), DispatchError> {
        Ok(())
    }
}

fn validate_ctx(ctx: &CommandCtx) -> Result<(), Rejection> {
    let ok = |s: &str| !s.trim().is_empty() && s.len() <= 256 && !s.chars().any(char::is_control);
    if !ok(&ctx.tenant) {
        return Err(Rejection::InvalidTenant(ctx.tenant.clone()));
    }
    if !ok(&ctx.actor.id) {
        return Err(Rejection::InvalidActor(ctx.actor.id.clone()));
    }
    Ok(())
}

/// `sha256:` over the JCS of the command type, the tenant and the canonical request. A key
/// reused by another tenant or for another request is a mismatch, never a replay.
fn request_sha<C: Command>(cmd: &C, ctx: &CommandCtx) -> Result<Sha256Ref, DispatchError> {
    let request = json!({
        "command": C::TYPE,
        "tenant": ctx.tenant,
        "request": cmd.request(),
    });
    sha256_jcs(&request).map_err(|e| DispatchError::Rejected(Rejection::Payload(e.to_string())))
}

fn replay<C: Command>(
    k: &Keyed<'_>,
    stored: StoredResult,
) -> Result<Dispatched<C::Output>, DispatchError> {
    if stored.command_type != C::TYPE || stored.request_sha != k.request_sha.as_str() {
        return Err(DispatchError::IdempotencyMismatch {
            key: k.key.to_string(),
        });
    }
    let output =
        serde_json::from_value(stored.result).map_err(|e| DispatchError::CorruptResult {
            key: k.key.to_string(),
            reason: e.to_string(),
        })?;
    Ok(Dispatched {
        output,
        replayed: true,
    })
}

async fn fetch_dedup(
    conn: &mut PgConnection,
    key: &str,
) -> Result<Option<StoredResult>, DispatchError> {
    let row = sqlx::query(
        "SELECT command_type, request_sha, result FROM ledger.command_dedup
          WHERE idempotency_key = $1",
    )
    .bind(key)
    .fetch_optional(&mut *conn)
    .await?;
    match row {
        None => Ok(None),
        Some(r) => Ok(Some(StoredResult {
            command_type: r.try_get("command_type")?,
            request_sha: r.try_get("request_sha")?,
            result: r.try_get::<Json<Value>, _>("result")?.0,
        })),
    }
}

/// A command may only touch streams of its own tenant.
async fn check_tenancy(
    conn: &mut PgConnection,
    heads: &Heads,
    ctx: &CommandCtx,
) -> Result<(), DispatchError> {
    if heads.0.is_empty() {
        return Ok(());
    }
    let ids: Vec<&str> = heads.0.keys().map(String::as_str).collect();
    let rows =
        sqlx::query("SELECT stream_id, tenant_id FROM ledger.streams WHERE stream_id = ANY($1)")
            .bind(&ids)
            .fetch_all(&mut *conn)
            .await?;
    for r in rows {
        let tenant: String = r.try_get("tenant_id")?;
        if tenant != ctx.tenant {
            return Err(DispatchError::Rejected(Rejection::TenantMismatch {
                stream: r.try_get("stream_id")?,
            }));
        }
    }
    Ok(())
}

/// Step 6a (new stream) or 6b (CAS on an existing head). Returns the new head.
async fn advance_head(
    conn: &mut PgConnection,
    a: &StreamAppend,
    tenant: &str,
) -> Result<u64, DispatchError> {
    let n = a.events.len() as u64;
    let from = a.expected.head();
    let new_head = to_i64_u(from + n)?;
    let row = if from == 0 {
        sqlx::query(
            "INSERT INTO ledger.streams (stream_id, kind, tenant_id, head) VALUES ($1, $2, $3, $4)
             ON CONFLICT (stream_id) DO NOTHING RETURNING head",
        )
        .bind(&a.stream_id)
        .bind(a.kind)
        .bind(tenant)
        .bind(new_head)
        .fetch_optional(&mut *conn)
        .await?
    } else {
        sqlx::query(
            "UPDATE ledger.streams SET head = head + $2 WHERE stream_id = $1 AND head = $3
             RETURNING head",
        )
        .bind(&a.stream_id)
        .bind(to_i64_u(n)?)
        .bind(to_i64_u(from)?)
        .fetch_optional(&mut *conn)
        .await?
    };
    match row {
        Some(r) => to_u64(r.try_get("head")?),
        None => {
            // Another transaction created or advanced the stream first.
            let actual: Option<i64> =
                sqlx::query_scalar("SELECT head FROM ledger.streams WHERE stream_id = $1")
                    .bind(&a.stream_id)
                    .fetch_optional(&mut *conn)
                    .await?;
            Err(DispatchError::Conflict {
                stream: a.stream_id.clone(),
                expected: a.expected,
                actual: to_u64(actual.unwrap_or(0))?,
            })
        }
    }
}

async fn allocate_positions(conn: &mut PgConnection, n: usize) -> Result<Vec<i64>, DispatchError> {
    let mut positions: Vec<i64> = sqlx::query_scalar(
        "SELECT nextval('ledger.events_global_pos_seq') FROM generate_series(1, $1)",
    )
    .bind(to_i64(n)?)
    .fetch_all(&mut *conn)
    .await?;
    positions.sort_unstable();
    Ok(positions)
}

fn to_u64(v: i64) -> Result<u64, DispatchError> {
    u64::try_from(v).map_err(|_| DispatchError::Integrity(format!("negative head {v}")))
}

fn to_i64(v: usize) -> Result<i64, DispatchError> {
    i64::try_from(v).map_err(|_| DispatchError::Integrity(format!("{v} out of range")))
}

fn to_i64_u(v: u64) -> Result<i64, DispatchError> {
    i64::try_from(v).map_err(|_| DispatchError::Integrity(format!("{v} out of range")))
}
