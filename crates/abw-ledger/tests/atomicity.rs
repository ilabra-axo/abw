//! T5: a record is atomic with its effect. Fault injection aborts the dispatch at every
//! statement boundary of target-system-design §3.2.1; afterwards nothing of the command
//! exists, and the same command then succeeds. SUB-03: outbox jobs commit with their event.

mod common;

use abw_ledger::probe::Append;
use abw_ledger::{Canon, DispatchError, Expected, FaultPoint, OutboxJob, artifact_bytes};
use common::{TestDb, append, ctx};
use serde_json::json;
use sqlx::Row;

fn full_command(id: &str, expected: Expected, key: &str) -> Append {
    Append::new(id, expected, json!({"id": id}))
        .idempotency_key(key)
        .artifact(artifact_bytes(
            format!("artifact for {key}").into_bytes(),
            Canon::RawUtf8,
            "text/plain",
        ))
        .outbox(OutboxJob::new("probe.notify", key, json!({"key": key})))
}

async fn residue(db: &TestDb, stream: &str, key: &str) -> Vec<String> {
    let checks = [
        (
            "events",
            format!(
                "SELECT count(*) FROM ledger.events WHERE stream_id = '{stream}' AND seq > (SELECT coalesce(max(seq), 0) FROM proj.probes WHERE stream_id = '{stream}')"
            ),
        ),
        (
            "artifacts",
            format!(
                "SELECT count(*) FROM ledger.artifacts WHERE sha = '{}'",
                abw_canon::sha256_raw(format!("artifact for {key}").as_bytes())
            ),
        ),
        (
            "outbox",
            format!("SELECT count(*) FROM ledger.outbox WHERE dedupe_key = '{key}'"),
        ),
        (
            "dedup",
            format!("SELECT count(*) FROM ledger.command_dedup WHERE idempotency_key = '{key}'"),
        ),
    ];
    let mut found = Vec::new();
    for (what, sql) in checks {
        if db.scalar_i64(&sql).await != 0 {
            found.push(what.to_string());
        }
    }
    found
}

// @req T5
#[tokio::test]
async fn fault_points_cover_every_statement_boundary() {
    for p in [
        FaultPoint::AfterDedup,
        FaultPoint::AfterLock,
        FaultPoint::AfterHeads,
        FaultPoint::AfterArtifacts,
        FaultPoint::AfterEvents,
        FaultPoint::AfterProjections,
        FaultPoint::AfterOutbox,
        FaultPoint::BeforeCommit,
    ] {
        assert!(
            FaultPoint::ALL.contains(&p),
            "{p:?} missing from FaultPoint::ALL"
        );
    }
}

// @req T5, SUB-03
#[tokio::test]
async fn an_aborted_dispatch_leaves_nothing_behind_on_a_new_stream() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    for (i, point) in FaultPoint::ALL.iter().enumerate() {
        let id = format!("new{i}");
        let stream = format!("probe:{id}");
        let key = format!("k-new-{i}");
        let faulty = ledger.inject_fault(*point);
        let r = faulty
            .dispatch(full_command(&id, Expected::NoStream, &key), &ctx())
            .await;
        assert!(
            matches!(r, Err(DispatchError::Injected(p)) if p == *point),
            "{point:?}: {r:?}"
        );
        assert_eq!(
            db.scalar_i64(&format!(
                "SELECT count(*) FROM ledger.streams WHERE stream_id = '{stream}'"
            ))
            .await,
            0,
            "{point:?}: stream row survived"
        );
        assert_eq!(
            db.scalar_i64(&format!(
                "SELECT count(*) FROM proj.probes WHERE stream_id = '{stream}'"
            ))
            .await,
            0,
            "{point:?}: projection row survived"
        );
        assert_eq!(
            residue(&db, &stream, &key).await,
            Vec::<String>::new(),
            "{point:?}"
        );

        // The aborted attempt left nothing to collide with: the same command now commits.
        match ledger
            .dispatch(full_command(&id, Expected::NoStream, &key), &ctx())
            .await
        {
            Ok(d) => assert!(
                !d.replayed,
                "{point:?}: an aborted attempt must not be replayable"
            ),
            Err(e) => panic!("{point:?}: retry after abort failed: {e:?}"),
        }
    }
    db.finish().await;
}

// @req T5, SUB-03
#[tokio::test]
async fn an_aborted_dispatch_leaves_an_existing_stream_untouched() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let first = append(&ledger, "old", Expected::NoStream, json!({"v": 1})).await;
    for (i, point) in FaultPoint::ALL.iter().enumerate() {
        let key = format!("k-old-{i}");
        let r = ledger
            .inject_fault(*point)
            .dispatch(full_command("old", Expected::Exact(1), &key), &ctx())
            .await;
        assert!(
            matches!(r, Err(DispatchError::Injected(_))),
            "{point:?}: {r:?}"
        );
        assert_eq!(
            db.scalar_i64("SELECT head FROM ledger.streams WHERE stream_id = 'probe:old'")
                .await,
            1,
            "{point:?}"
        );
        let applied = db
            .scalar_i64("SELECT applied_pos FROM proj.probes WHERE stream_id = 'probe:old'")
            .await;
        assert_eq!(applied, first.global_pos, "{point:?}: projection moved");
        assert_eq!(
            residue(&db, "probe:old", &key).await,
            Vec::<String>::new(),
            "{point:?}"
        );
    }
    db.finish().await;
}

// @req SUB-03
#[tokio::test]
async fn outbox_job_references_the_event_that_caused_it() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let out = match ledger
        .dispatch(full_command("ob", Expected::NoStream, "k-ob"), &ctx())
        .await
    {
        Ok(d) => d.output,
        Err(e) => panic!("{e:?}"),
    };
    let row = match sqlx::query("SELECT topic, source_pos, payload, done_at IS NULL AS pending FROM ledger.outbox WHERE dedupe_key = 'k-ob'")
        .fetch_one(&db.admin)
        .await
    {
        Ok(r) => r,
        Err(e) => panic!("{e}"),
    };
    assert_eq!(
        row.try_get::<String, _>("topic").ok().as_deref(),
        Some("probe.notify")
    );
    assert_eq!(
        row.try_get::<i64, _>("source_pos").ok(),
        Some(out.global_pos)
    );
    assert_eq!(
        row.try_get::<serde_json::Value, _>("payload").ok(),
        Some(json!({"key": "k-ob"}))
    );
    assert_eq!(row.try_get::<bool, _>("pending").ok(), Some(true));
    // An outbox row cannot exist without its event.
    let mut app = db.session(&db.app_url).await;
    common::assert_rejected(
        &mut app,
        "INSERT INTO ledger.outbox (topic, dedupe_key, source_pos, payload) VALUES ('x', 'orphan', 999999999, '{}')",
        "23503",
    )
    .await;
    db.finish().await;
}
