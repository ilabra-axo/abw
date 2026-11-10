//! T2 at the database: streams are gap-free and their heads only move forward, whatever
//! the client does. Appendix D cases 8, 11 and 17, run as `abw_app`.

mod common;

use abw_ledger::Expected;
use common::{TestDb, append, assert_rejected};
use serde_json::json;

const SHA: &str = "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a";

fn event_insert(stream: &str, seq: i64) -> String {
    format!(
        "INSERT INTO ledger.events (stream_id, seq, event_type, schema_version, payload, payload_sha,
           command_id, correlation_id, actor, actor_kind)
         VALUES ('{stream}', {seq}, 'probe.recorded', 1, '{{}}', '{SHA}',
           gen_random_uuid(), gen_random_uuid(), 'test', 'platform')"
    )
}

// @req T2
#[tokio::test]
async fn head_cannot_advance_without_events() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    append(&ledger, "g1", Expected::NoStream, json!({})).await;
    let mut app = db.session(&db.app_url).await;
    // Case 8: checked by the deferred gap-free trigger at COMMIT.
    assert_rejected(
        &mut app,
        "UPDATE ledger.streams SET head = head + 1 WHERE stream_id = 'probe:g1'",
        "ABW-T2",
    )
    .await;
    // Case 17: a new stream row with no events.
    assert_rejected(
        &mut app,
        "INSERT INTO ledger.streams (stream_id, kind, tenant_id, head) VALUES ('probe:g2', 'probe', 'tenant-a', 1)",
        "ABW-T2",
    )
    .await;
    db.finish().await;
}

// @req T2
#[tokio::test]
async fn events_cannot_run_ahead_of_or_skip_the_head() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    append(&ledger, "g3", Expected::NoStream, json!({})).await;
    let mut app = db.session(&db.app_url).await;
    // seq beyond head (t_events_seq).
    assert_rejected(&mut app, &event_insert("probe:g3", 2), "ABW-T2").await;
    // Head advanced by two with only one event: a gap (t_streams_gap_free at COMMIT).
    let gap = format!(
        "UPDATE ledger.streams SET head = 3 WHERE stream_id = 'probe:g3'; {}",
        event_insert("probe:g3", 3)
    );
    assert_rejected(&mut app, &gap, "ABW-T2").await;
    // A duplicate seq.
    let dup = format!(
        "UPDATE ledger.streams SET head = 2 WHERE stream_id = 'probe:g3'; {}; {}",
        event_insert("probe:g3", 2),
        event_insert("probe:g3", 2)
    );
    assert_rejected(&mut app, &dup, "23505").await;
    // The correct manual append is accepted, which shows the rejections above are specific.
    let ok = format!(
        "UPDATE ledger.streams SET head = 2 WHERE stream_id = 'probe:g3'; {}",
        event_insert("probe:g3", 2)
    );
    if let Some(e) = common::try_tx(&mut app, &ok).await {
        panic!("a gap-free manual append should commit: {e}");
    }
    db.finish().await;
}

// @req T2
#[tokio::test]
async fn head_only_moves_forward_and_identity_is_fixed() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    append(&ledger, "g4", Expected::NoStream, json!({})).await;
    append(&ledger, "g4", Expected::Exact(1), json!({})).await;
    let mut app = db.session(&db.app_url).await;
    // Case 11.
    assert_rejected(
        &mut app,
        "UPDATE ledger.streams SET head = 1 WHERE stream_id = 'probe:g4'",
        "ABW-T2",
    )
    .await;
    assert_rejected(
        &mut app,
        "UPDATE ledger.streams SET head = 2 WHERE stream_id = 'probe:g4'",
        "ABW-T2",
    )
    .await;
    let mut owner = db.session(&db.owner_url).await;
    assert_rejected(
        &mut owner,
        "UPDATE ledger.streams SET tenant_id = 'tenant-b', head = head + 1 WHERE stream_id = 'probe:g4'",
        "ABW-T2",
    )
    .await;
    db.finish().await;
}

// @req T2
#[tokio::test]
async fn stream_ids_are_kind_prefixed() {
    let db = TestDb::migrated().await;
    let mut admin = db.session(&db.admin_url).await;
    for bad in ["Probe:x", "probe:", "probe", "probe:a b", "episode:x"] {
        let sql = format!(
            "SET LOCAL session_replication_role = replica;
             INSERT INTO ledger.streams (stream_id, kind, tenant_id, head) VALUES ('{bad}', 'probe', 't', 0)"
        );
        assert_rejected(&mut admin, &sql, "23514").await;
    }
    db.finish().await;
}
