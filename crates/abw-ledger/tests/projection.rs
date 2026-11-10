//! T3: a projection row changes only in the transaction that appends its justifying event,
//! enforced by the database; rebuilding from events reproduces the projection exactly.
//! Appendix D cases 6 and 22-24.

mod common;

use abw_ledger::{Expected, admin};
use common::{TestDb, append, assert_rejected};
use serde_json::json;
use sqlx::Row;

// @req T3
#[tokio::test]
async fn inline_projection_is_written_with_its_event() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let a = append(&ledger, "p1", Expected::NoStream, json!({"v": "one"})).await;
    let b = append(&ledger, "p1", Expected::Exact(1), json!({"v": "two"})).await;
    let row = match sqlx::query(
        "SELECT seq, body, applied_pos FROM proj.probes WHERE stream_id = 'probe:p1'",
    )
    .fetch_one(&db.admin)
    .await
    {
        Ok(r) => r,
        Err(e) => panic!("{e}"),
    };
    assert_eq!(row.try_get::<i64, _>("seq").ok(), Some(2));
    assert_eq!(
        row.try_get::<serde_json::Value, _>("body").ok(),
        Some(json!({"v": "two"}))
    );
    assert_eq!(
        row.try_get::<i64, _>("applied_pos").ok(),
        Some(b.global_pos)
    );
    assert_ne!(a.global_pos, b.global_pos);
    db.finish().await;
}

// @req T3
#[tokio::test]
async fn projection_rows_cannot_change_without_an_event() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let first = append(&ledger, "p2", Expected::NoStream, json!({"v": 1})).await;
    for url in [&db.app_url, &db.owner_url] {
        let mut conn = db.session(url).await;
        // Case 6 / 24: an update keeping the old position, with no event in this transaction.
        assert_rejected(
            &mut conn,
            "UPDATE proj.probes SET body = '{\"v\": 999}' WHERE stream_id = 'probe:p2'",
            "ABW-T3",
        )
        .await;
        // Case 22: an insert citing an event this transaction did not append.
        let insert = format!(
            "INSERT INTO proj.probes (stream_id, seq, body, applied_pos) VALUES ('probe:other', 1, '{{}}', {})",
            first.global_pos
        );
        assert_rejected(&mut conn, &insert, "ABW-T3").await;
    }
    // abw_app has no DELETE grant; the owner, which has it, is stopped by the trigger.
    let mut owner = db.session(&db.owner_url).await;
    assert_rejected(&mut owner, "DELETE FROM proj.probes", "ABW-T3").await;
    db.finish().await;
}

// @req T3
#[tokio::test]
async fn read_role_cannot_touch_projections() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    append(&ledger, "p3", Expected::NoStream, json!({})).await;
    let mut read = db.session(&db.read_url).await;
    assert_rejected(
        &mut read,
        "SET TRANSACTION READ WRITE; UPDATE proj.probes SET body = '{}'",
        "permission denied",
    )
    .await;
    db.finish().await;
}

// @req T3
#[tokio::test]
async fn rebuild_from_events_matches_the_live_projection() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    for id in ["r1", "r2", "r3"] {
        append(&ledger, id, Expected::NoStream, json!({"id": id, "n": 0})).await;
        append(
            &ledger,
            id,
            Expected::Exact(1),
            json!({"id": id, "n": 1, "f": 2.5}),
        )
        .await;
    }
    match admin::rebuild_diff(&db.owner_url, "proj.probes").await {
        Ok(diff) => assert!(diff.is_empty(), "rebuild differs from live state: {diff:?}"),
        Err(e) => panic!("rebuild failed: {e:?}"),
    }
    db.finish().await;
}

// @req T3
#[tokio::test]
async fn rebuild_detects_a_projection_tampered_with_outside_the_ledger() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    append(&ledger, "t1", Expected::NoStream, json!({"v": "real"})).await;
    // A superuser bypass (triggers off), which the design says the audit must catch (§3.10).
    let mut admin_conn = db.session(&db.admin_url).await;
    common::exec(
        &mut admin_conn,
        "SET session_replication_role = replica;
         UPDATE proj.probes SET body = '{\"v\": \"forged\"}' WHERE stream_id = 'probe:t1';
         SET session_replication_role = origin;",
    )
    .await;
    match admin::rebuild_diff(&db.owner_url, "proj.probes").await {
        Ok(diff) => assert!(!diff.is_empty(), "a forged projection row went undetected"),
        Err(e) => panic!("rebuild failed: {e:?}"),
    }
    db.finish().await;
}
