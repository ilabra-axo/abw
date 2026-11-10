//! T6 (the database enforces immutability), T1 (runtime half of the write boundary) and
//! the §13.2 boot checks. Cases follow target-system-design Appendix D.

mod common;

use abw_ledger::probe::Append;
use abw_ledger::{Canon, Expected, Ledger, LedgerConfig, SingletonFence, artifact_bytes};
use common::{TestDb, append, assert_rejected, ctx};
use serde_json::json;

async fn seeded() -> (TestDb, Ledger, SingletonFence) {
    let db = TestDb::migrated().await;
    let (ledger, _read, fence) = db.ledger().await;
    let art = artifact_bytes(b"hello, ABW\n".to_vec(), Canon::RawUtf8, "text/plain");
    let cmd = Append::new("seed", Expected::NoStream, json!({"n": 1}))
        .artifact(art)
        .idempotency_key("seed-1");
    if let Err(e) = ledger.dispatch(cmd, &ctx()).await {
        panic!("seed append failed: {e:?}");
    }
    (db, ledger, fence)
}

// @req T6
#[tokio::test]
async fn app_role_cannot_update_delete_or_truncate_history() {
    let (db, _ledger, _fence) = seeded().await;
    let mut app = db.session(&db.app_url).await;
    // Appendix D cases 1-3, extended to every history table.
    for sql in [
        "UPDATE ledger.events SET actor = 'x'",
        "DELETE FROM ledger.events",
        "TRUNCATE ledger.events",
        "UPDATE ledger.artifacts SET media_type = 'x'",
        "DELETE FROM ledger.artifacts",
        "TRUNCATE ledger.artifacts",
        "DELETE FROM ledger.streams",
        "UPDATE ledger.event_types SET introduced_by = 'x'",
        "UPDATE ledger.command_dedup SET result = '{}'",
    ] {
        assert_rejected(&mut app, sql, "permission denied").await;
    }
    db.finish().await;
}

// @req T6
#[tokio::test]
async fn owner_role_is_stopped_by_triggers() {
    let (db, _ledger, _fence) = seeded().await;
    let mut owner = db.session(&db.owner_url).await;
    for sql in [
        "UPDATE ledger.events SET actor = 'x'",
        "DELETE FROM ledger.events",
        "TRUNCATE ledger.events CASCADE",
        "UPDATE ledger.artifacts SET media_type = 'x'",
        "DELETE FROM ledger.artifacts",
        "TRUNCATE ledger.artifacts CASCADE",
        "DELETE FROM ledger.streams",
        "UPDATE ledger.event_types SET introduced_by = 'x'",
        "DELETE FROM ledger.event_types WHERE event_type = 'probe.recorded'",
        "UPDATE ledger.command_dedup SET result = '{}'",
        "DELETE FROM ledger.command_dedup",
    ] {
        assert_rejected(&mut owner, sql, "ABW-T").await;
    }
    db.finish().await;
}

// @req T6
#[tokio::test]
async fn policy_versions_are_immutable() {
    let (db, _ledger, _fence) = seeded().await;
    // Insert the row with triggers off (superuser, replica mode), so the test depends only
    // on the immutability trigger, not on how policy rows are normally justified.
    let mut admin = db.session(&db.admin_url).await;
    let insert = "SET LOCAL session_replication_role = replica;
        INSERT INTO ledger.policy_versions
        (policy_id, version, policy_kind, tenant_id, doc, doc_sha, supersedes, accepted_by, rationale, active_from, event_pos)
        SELECT '00000000-0000-0000-0000-000000000001', 1, 'lattice', NULL, '{}',
               'sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a',
               NULL, 'ivan', 'test', now(), min(global_pos) FROM ledger.events";
    if let Some(e) = common::try_tx(&mut admin, insert).await {
        panic!("seeding a policy version failed: {e}");
    }
    let mut owner = db.session(&db.owner_url).await;
    assert_rejected(
        &mut owner,
        "UPDATE ledger.policy_versions SET rationale = 'edited'",
        "ABW-T6",
    )
    .await;
    assert_rejected(&mut owner, "DELETE FROM ledger.policy_versions", "ABW-T6").await;
    db.finish().await;
}

// @req T6
#[tokio::test]
async fn foreign_keys_into_history_restrict_deletes() {
    let (db, _ledger, _fence) = seeded().await;
    let mut owner = db.session(&db.owner_url).await;
    // stream_kinds has no immutability trigger; the RESTRICT foreign key from streams stops it.
    assert_rejected(
        &mut owner,
        "DELETE FROM ledger.stream_kinds WHERE kind = 'probe'",
        "23503",
    )
    .await;
    db.finish().await;
}

// @req T1
#[tokio::test]
async fn read_role_cannot_write_or_escalate() {
    let (db, _ledger, _fence) = seeded().await;
    let mut read = db.session(&db.read_url).await;
    // Case 12/13: the session default is read-only.
    assert_rejected(
        &mut read,
        "INSERT INTO ledger.stream_kinds (kind, concurrency, description) VALUES ('zz', 'exact', 'x')",
        "read-only transaction",
    )
    .await;
    // Case 18: even an explicit read-write transaction has no grant.
    assert_rejected(
        &mut read,
        "SET TRANSACTION READ WRITE; INSERT INTO ledger.stream_kinds (kind, concurrency, description) VALUES ('zz', 'exact', 'x')",
        "permission denied",
    )
    .await;
    // Case 37: abw_read cannot assume a writing role.
    assert_rejected(&mut read, "SET ROLE abw_app", "permission denied").await;
    assert_rejected(&mut read, "SET ROLE abw_crud", "permission denied").await;
    db.finish().await;
}

// @req T1
#[tokio::test]
async fn boot_refuses_an_app_url_whose_role_owns_tables() {
    let db = TestDb::migrated().await;
    let fence = match SingletonFence::acquire(&db.app_url).await {
        Ok(f) => f,
        Err(e) => panic!("{e:?}"),
    };
    // Ownership bypasses REVOKE, so the app handle must not be the owner (§13.2).
    let cfg = LedgerConfig::new(&db.owner_url, &db.read_url);
    assert!(
        Ledger::connect(&cfg, &fence).await.is_err(),
        "owner URL accepted as the app handle"
    );
    drop(fence);
    db.finish().await;
}

// @req T1
#[tokio::test]
async fn boot_refuses_a_read_url_that_can_write() {
    let db = TestDb::migrated().await;
    let fence = match SingletonFence::acquire(&db.app_url).await {
        Ok(f) => f,
        Err(e) => panic!("{e:?}"),
    };
    let cfg = LedgerConfig::new(&db.app_url, &db.app_url);
    assert!(
        Ledger::connect(&cfg, &fence).await.is_err(),
        "a writable role accepted as ReadDb"
    );
    drop(fence);
    db.finish().await;
}

// @req T2
#[tokio::test]
async fn boot_refuses_an_event_registry_that_differs_from_the_table() {
    let db = TestDb::migrated().await;
    let mut admin = db.session(&db.admin_url).await;
    common::exec(
        &mut admin,
        "INSERT INTO ledger.event_types
           (event_type, schema_version, stream_kind, epistemic, json_schema, json_schema_sha, upcasts_from, introduced_by)
         VALUES ('probe.unregistered', 1, 'probe', 'structural', '{}',
                 'sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a', NULL, 'test')",
    )
    .await;
    let fence = match SingletonFence::acquire(&db.app_url).await {
        Ok(f) => f,
        Err(e) => panic!("{e:?}"),
    };
    assert!(
        Ledger::connect(&db.config(), &fence).await.is_err(),
        "boot accepted an event type the Rust registry does not declare (OD-08)"
    );
    drop(fence);
    db.finish().await;
}

// @req T1
#[tokio::test]
async fn read_handle_serves_reads() {
    let db = TestDb::migrated().await;
    let (ledger, read, _fence) = db.ledger().await;
    let out = append(&ledger, "r1", Expected::NoStream, json!({"k": "v"})).await;
    let rows = match read
        .fetch_all(abw_ledger::read_query!(
            "SELECT seq FROM proj.probes WHERE stream_id = 'probe:r1'"
        ))
        .await
    {
        Ok(r) => r,
        Err(e) => panic!("read through ReadDb failed: {e:?}"),
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(out.seq, 1);
    db.finish().await;
}
