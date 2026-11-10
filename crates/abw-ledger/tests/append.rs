//! T2 through the command path: expected-version CAS, gap-free sequences, typed and
//! versioned envelopes, canonical payload hashes. SUB-05: idempotent commands.

mod common;

use abw_canon::sha256_jcs;
use abw_ledger::probe::Append;
use abw_ledger::{Actor, CommandCtx, DispatchError, Expected};
use common::{TestDb, append, ctx};
use serde_json::json;
use sqlx::Row;
use std::sync::Arc;

// @req T2
#[tokio::test]
async fn append_assigns_dense_sequence_numbers() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let a = append(&ledger, "s1", Expected::NoStream, json!({"i": 1})).await;
    let b = append(&ledger, "s1", Expected::Exact(1), json!({"i": 2})).await;
    let c = append(&ledger, "s1", Expected::Exact(2), json!({"i": 3})).await;
    assert_eq!((a.seq, b.seq, c.seq), (1, 2, 3));
    assert_eq!(a.stream_id, "probe:s1");
    assert!(a.global_pos < b.global_pos && b.global_pos < c.global_pos);
    assert_eq!(
        db.scalar_i64("SELECT head FROM ledger.streams WHERE stream_id = 'probe:s1'")
            .await,
        3
    );
    assert_eq!(
        db.scalar_i64("SELECT count(*) FROM ledger.events WHERE stream_id = 'probe:s1'")
            .await,
        3
    );
    db.finish().await;
}

// @req T2
#[tokio::test]
async fn wrong_expected_version_is_a_typed_conflict() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    append(&ledger, "s2", Expected::NoStream, json!({})).await;

    let again = ledger
        .dispatch(Append::new("s2", Expected::NoStream, json!({})), &ctx())
        .await;
    assert!(
        matches!(
            again,
            Err(DispatchError::Conflict {
                expected: Expected::NoStream,
                actual: 1,
                ..
            })
        ),
        "{again:?}"
    );
    let stale = ledger
        .dispatch(Append::new("s2", Expected::Exact(0), json!({})), &ctx())
        .await;
    assert!(
        matches!(stale, Err(DispatchError::Conflict { actual: 1, .. })),
        "{stale:?}"
    );
    let ahead = ledger
        .dispatch(Append::new("s2", Expected::Exact(5), json!({})), &ctx())
        .await;
    assert!(
        matches!(ahead, Err(DispatchError::Conflict { actual: 1, .. })),
        "{ahead:?}"
    );
    let missing = ledger
        .dispatch(Append::new("nope", Expected::Exact(1), json!({})), &ctx())
        .await;
    assert!(
        matches!(missing, Err(DispatchError::Conflict { actual: 0, .. })),
        "{missing:?}"
    );
    assert_eq!(db.scalar_i64("SELECT count(*) FROM ledger.events").await, 1);
    db.finish().await;
}

// @req T2
#[tokio::test]
async fn concurrent_appends_at_the_same_version_have_exactly_one_winner() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    append(&ledger, "race", Expected::NoStream, json!({})).await;
    let ledger = Arc::new(ledger);
    let mut tasks = Vec::new();
    for i in 0..32 {
        let l = Arc::clone(&ledger);
        // Request-owned tasks, all awaited below.
        tasks.push(tokio::spawn(async move {
            l.dispatch(
                Append::new("race", Expected::Exact(1), json!({"writer": i})),
                &ctx(),
            )
            .await
        }));
    }
    let (mut won, mut conflicts) = (0, 0);
    for t in tasks {
        match t.await {
            Ok(Ok(d)) => {
                assert_eq!(d.output.seq, 2);
                won += 1;
            }
            Ok(Err(DispatchError::Conflict {
                expected: Expected::Exact(1),
                actual: 2,
                ..
            })) => conflicts += 1,
            Ok(Err(e)) => panic!("unexpected error: {e:?}"),
            Err(e) => panic!("task failed: {e}"),
        }
    }
    assert_eq!((won, conflicts), (1, 31));
    assert_eq!(
        db.scalar_i64("SELECT head FROM ledger.streams WHERE stream_id = 'probe:race'")
            .await,
        2
    );
    assert_eq!(
        db.scalar_i64("SELECT count(*) FROM ledger.events WHERE stream_id = 'probe:race'")
            .await,
        2
    );
    db.finish().await;
}

// @req T2
#[tokio::test]
async fn envelopes_are_typed_versioned_and_attributed() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let corr = uuid::Uuid::new_v4();
    let cx = CommandCtx::new("tenant-z", Actor::agent("agent-7")).with_correlation(corr);
    let out = match ledger
        .dispatch(Append::new("env", Expected::NoStream, json!({"x": 1})), &cx)
        .await
    {
        Ok(d) => d.output,
        Err(e) => panic!("{e:?}"),
    };
    let row = match sqlx::query(
        "SELECT e.event_type, e.schema_version, e.actor, e.actor_kind, e.correlation_id,
                e.command_id IS NOT NULL AS has_cmd, e.txid IS NOT NULL AS has_txid, s.tenant_id
         FROM ledger.events e JOIN ledger.streams s USING (stream_id)
         WHERE e.global_pos = $1",
    )
    .bind(out.global_pos)
    .fetch_one(&db.admin)
    .await
    {
        Ok(r) => r,
        Err(e) => panic!("{e}"),
    };
    let get = |c: &str| -> String {
        match row.try_get::<String, _>(c) {
            Ok(v) => v,
            Err(e) => panic!("{c}: {e}"),
        }
    };
    assert_eq!(get("event_type"), "probe.recorded");
    assert_eq!(row.try_get::<i32, _>("schema_version").ok(), Some(1));
    assert_eq!(get("actor"), "agent-7");
    assert_eq!(get("actor_kind"), "agent");
    assert_eq!(get("tenant_id"), "tenant-z");
    assert_eq!(
        row.try_get::<uuid::Uuid, _>("correlation_id").ok(),
        Some(corr)
    );
    assert_eq!(row.try_get::<bool, _>("has_cmd").ok(), Some(true));
    assert_eq!(row.try_get::<bool, _>("has_txid").ok(), Some(true));
    db.finish().await;
}

// @req T2, T4
#[tokio::test]
async fn payload_sha_is_jcs_of_the_payload_and_survives_jsonb() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    // Appendix D case 29: jsonb reorders keys and respells numbers.
    let body = json!({"b": 1, "a": [1.0, "x"], "nested": {"z": 0.1, "y": -0.0}});
    let expected = match sha256_jcs(&body) {
        Ok(s) => s,
        Err(e) => panic!("{e:?}"),
    };
    let out = append(&ledger, "sha", Expected::NoStream, body.clone()).await;
    assert_eq!(out.payload_sha, expected);
    let row =
        match sqlx::query("SELECT payload, payload_sha FROM ledger.events WHERE global_pos = $1")
            .bind(out.global_pos)
            .fetch_one(&db.admin)
            .await
        {
            Ok(r) => r,
            Err(e) => panic!("{e}"),
        };
    let stored: serde_json::Value = match row.try_get("payload") {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };
    let stored_sha: String = match row.try_get("payload_sha") {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };
    assert_eq!(stored_sha, expected.as_str());
    // The nightly audit re-derives the hash from the decoded jsonb (§3.10).
    assert_eq!(sha256_jcs(&stored).ok(), Some(expected));
    db.finish().await;
}

// @req T2
#[tokio::test]
async fn non_object_bodies_and_bad_ids_are_rejected_before_any_write() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    for (id, body) in [
        ("ok", json!([1, 2])),
        ("ok", json!("text")),
        ("bad id", json!({})),
        ("", json!({})),
    ] {
        let r = ledger
            .dispatch(Append::new(id, Expected::NoStream, body), &ctx())
            .await;
        assert!(
            matches!(r, Err(DispatchError::Rejected(_))),
            "{id:?}: {r:?}"
        );
    }
    assert_eq!(
        db.scalar_i64("SELECT count(*) FROM ledger.streams").await,
        0
    );
    db.finish().await;
}

// @req SUB-05
#[tokio::test]
async fn retry_with_the_same_key_replays_the_stored_result() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let cmd = || Append::new("idem", Expected::NoStream, json!({"v": 1})).idempotency_key("req-42");
    let first = match ledger.dispatch(cmd(), &ctx()).await {
        Ok(d) => d,
        Err(e) => panic!("{e:?}"),
    };
    assert!(!first.replayed);
    // A NoStream retry would conflict if it ran again; the replay returns the first result.
    let second = match ledger.dispatch(cmd(), &ctx()).await {
        Ok(d) => d,
        Err(e) => panic!("retry should replay, got {e:?}"),
    };
    assert!(second.replayed);
    assert_eq!(second.output.global_pos, first.output.global_pos);
    assert_eq!(second.output.seq, first.output.seq);
    assert_eq!(db.scalar_i64("SELECT count(*) FROM ledger.events").await, 1);
    assert_eq!(
        db.scalar_i64("SELECT count(*) FROM ledger.command_dedup")
            .await,
        1
    );
    db.finish().await;
}

// @req SUB-05
#[tokio::test]
async fn same_key_with_a_different_request_is_rejected() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let a = Append::new("idem2", Expected::NoStream, json!({"v": 1})).idempotency_key("req-7");
    if let Err(e) = ledger.dispatch(a, &ctx()).await {
        panic!("{e:?}");
    }
    let b = Append::new("idem2", Expected::NoStream, json!({"v": 2})).idempotency_key("req-7");
    let r = ledger.dispatch(b, &ctx()).await;
    assert!(
        matches!(r, Err(DispatchError::IdempotencyMismatch { .. })),
        "{r:?}"
    );
    assert_eq!(db.scalar_i64("SELECT count(*) FROM ledger.events").await, 1);
    db.finish().await;
}

// @req SUB-05
#[tokio::test]
async fn concurrent_retries_with_one_key_append_once() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let ledger = Arc::new(ledger);
    let mut tasks = Vec::new();
    for _ in 0..16 {
        let l = Arc::clone(&ledger);
        tasks.push(tokio::spawn(async move {
            l.dispatch(
                Append::new("idem3", Expected::NoStream, json!({"v": 1}))
                    .idempotency_key("req-race"),
                &ctx(),
            )
            .await
        }));
    }
    let mut positions = Vec::new();
    for t in tasks {
        match t.await {
            Ok(Ok(d)) => positions.push(d.output.global_pos),
            // A loser of the stream race may see the conflict before the dedup row commits.
            Ok(Err(DispatchError::Conflict { .. })) => {}
            Ok(Err(e)) => panic!("unexpected error: {e:?}"),
            Err(e) => panic!("task failed: {e}"),
        }
    }
    assert!(!positions.is_empty());
    assert!(
        positions.iter().all(|p| *p == positions[0]),
        "{positions:?}"
    );
    assert_eq!(db.scalar_i64("SELECT count(*) FROM ledger.events").await, 1);
    db.finish().await;
}
