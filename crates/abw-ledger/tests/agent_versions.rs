#![cfg(any())]
//! Agent versions as JCS hashes of resolved cards (target-system-design §4.1-§4.2;
//! capabilities.md §1 "Version an agent"; ADR-0008 §4 `agent`).

mod common;
#[path = "common/m2.rs"]
mod m2;

use abw_ledger::agent::{self, PutVersion, Retire, VersionReason};
use abw_ledger::{DispatchError, Expected, Rejection, admin};
use common::{TestDb, assert_rejected};
use m2::*;
use serde_json::{Value, json};

fn put(agent_id: &str, card: Value, reason: VersionReason) -> PutVersion {
    PutVersion::new(agent_id, card, reason)
}

// @req CAP-01.01
#[tokio::test]
async fn the_first_version_is_the_jcs_hash_of_the_resolved_card() {
    let db = TestDb::migrated().await;
    let (ledger, read, _fence) = db.ledger().await;
    let c = card("analyst", &["echo"]);
    let v = ok(
        ledger
            .dispatch(
                put("analyst", c.clone(), VersionReason::Seed),
                &human(TENANT_A),
            )
            .await,
        "put",
    )
    .output;
    let want_sha = ok(abw_canon::sha256_jcs(&c), "jcs");
    assert_eq!(v.card_sha, want_sha);
    assert_eq!(v.version_number, 1);
    assert!(v.created);
    assert_eq!(
        v.prompt_sha,
        abw_canon::sha256_raw("You are analyst, a test agent.".as_bytes())
    );
    // The card bytes are stored, canonical, under the card media type.
    match read.resolve(&v.card_sha).await {
        Ok(Some(bytes)) => assert_eq!(bytes, ok(abw_canon::jcs(&c), "jcs bytes")),
        other => panic!("card artifact missing: {other:?}"),
    }
    let media = match sqlx::query_scalar::<_, String>(
        "SELECT media_type FROM ledger.artifacts WHERE sha = $1",
    )
    .bind(v.card_sha.as_str())
    .fetch_one(&db.admin)
    .await
    {
        Ok(m) => m,
        Err(e) => panic!("{e}"),
    };
    assert_eq!(media, agent::CARD_MEDIA_TYPE);
    assert_eq!(media, "application/vnd.abw.agent-card+json");
    let evs = events(&db, "agent:analyst").await;
    assert_eq!(evs.len(), 1);
    assert_eq!(evs[0].event_type, "agent.version_created");
    assert_eq!(int(&evs[0], "version_number"), 1);
    assert_eq!(sha(&evs[0], "card_sha"), v.card_sha);
    assert_eq!(text(&evs[0], "reason"), "seed");
    match agent::head(&read, TENANT_A, "analyst").await {
        Ok(Some(h)) => {
            assert_eq!((h.version_number, h.retired), (1, false));
            assert_eq!(h.card_sha, v.card_sha);
        }
        other => panic!("agent head: {other:?}"),
    }
    db.finish().await;
}

// @req CAP-01.01
#[tokio::test]
async fn a_card_edit_creates_a_new_version() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let v1 = put_agent(&ledger, TENANT_A, "analyst", &["echo"]).await;
    let mut edited = card("analyst", &["echo"]);
    edited["system_prompt"] = json!("You are a careful analyst.");
    let v2 = ok(
        ledger
            .dispatch(
                put("analyst", edited.clone(), VersionReason::OwnerEdit)
                    .expected(Expected::Exact(1)),
                &human(TENANT_A),
            )
            .await,
        "edit",
    )
    .output;
    assert!(v2.created);
    assert_eq!(v2.version_number, 2);
    assert_ne!(v2.card_sha, v1.card_sha);
    assert_eq!(v2.card_sha, ok(abw_canon::sha256_jcs(&edited), "jcs"));
    // Narrowing the allowlist is a configuration change too: it is part of card_sha.
    let narrowed = card("analyst", &[]);
    let v3 = ok(
        ledger
            .dispatch(
                put("analyst", narrowed, VersionReason::OwnerEdit),
                &human(TENANT_A),
            )
            .await,
        "narrow",
    )
    .output;
    assert_eq!(v3.version_number, 3);
    let versions = db
        .scalar_i64("SELECT count(*) FROM proj.agent_versions WHERE agent_id = 'analyst'")
        .await;
    assert_eq!(versions, 3);
    let head = db
        .scalar_i64(
            "SELECT version_number::bigint FROM proj.agent_heads WHERE agent_id = 'analyst'",
        )
        .await;
    assert_eq!(head, 3);
    db.finish().await;
}

// @req CAP-01.01
#[tokio::test]
async fn an_identical_resolved_card_creates_no_version() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let v1 = put_agent(&ledger, TENANT_A, "analyst", &["echo"]).await;
    // The same card with its keys in another order and other whitespace has the same JCS
    // bytes, so the same identity.
    let reordered: Value = match serde_json::from_str(
        r#"{"$excluded": ["performance", "usage", "ontology_stats", "wallet", "declared_prompt_sha256"],
            "capabilities": {"tools_allowed": ["echo"], "model": "scripted-model"},
            "system_prompt": "You are analyst, a test agent.",
            "agent_id": "analyst"}"#,
    ) {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };
    for reason in [VersionReason::OwnerEdit, VersionReason::AdoptFile] {
        let again = ok(
            ledger
                .dispatch(put("analyst", reordered.clone(), reason), &human(TENANT_A))
                .await,
            "no-op",
        );
        assert!(!again.output.created, "an identical card made a version");
        assert_eq!(again.output.version_number, 1);
        assert_eq!(again.output.card_sha, v1.card_sha);
    }
    assert_eq!(
        events(&db, "agent:analyst").await.len(),
        1,
        "nothing appended"
    );
    db.finish().await;
}

// @req CAP-01.01
#[tokio::test]
async fn reverting_to_an_earlier_card_is_a_new_version() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let v1 = put_agent(&ledger, TENANT_A, "analyst", &["echo"]).await;
    ok(
        ledger
            .dispatch(
                put("analyst", card("analyst", &[]), VersionReason::OwnerEdit),
                &human(TENANT_A),
            )
            .await,
        "edit",
    );
    let back = ok(
        ledger
            .dispatch(
                put("analyst", card("analyst", &["echo"]), VersionReason::Revert),
                &human(TENANT_A),
            )
            .await,
        "revert",
    )
    .output;
    assert!(back.created);
    assert_eq!(back.version_number, 3);
    assert_eq!(
        back.card_sha, v1.card_sha,
        "same configuration, same identity"
    );
    db.finish().await;
}

// @req CAP-01.01
#[tokio::test]
async fn a_stale_version_or_another_tenants_agent_is_refused() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    put_agent(&ledger, TENANT_A, "analyst", &["echo"]).await;
    ok(
        ledger
            .dispatch(
                put("analyst", card("analyst", &[]), VersionReason::OwnerEdit),
                &human(TENANT_A),
            )
            .await,
        "v2",
    );
    let stale = put("analyst", card("analyst", &["x"]), VersionReason::OwnerEdit)
        .expected(Expected::Exact(1));
    match ledger.dispatch(stale, &human(TENANT_A)).await {
        Err(DispatchError::Conflict { actual: 2, .. }) => {}
        other => panic!("an edit against a stale head: {other:?}"),
    }
    let foreign = put("analyst", card("analyst", &["x"]), VersionReason::OwnerEdit);
    match ledger.dispatch(foreign, &human(TENANT_B)).await {
        Err(DispatchError::Rejected(Rejection::TenantMismatch { .. })) => {}
        other => panic!("tenant B edited tenant A's agent: {other:?}"),
    }
    for bad in [
        json!("not an object"),
        json!({"agent_id": "analyst"}),
        json!({"system_prompt": 7}),
    ] {
        match ledger
            .dispatch(
                put("analyst", bad.clone(), VersionReason::OwnerEdit),
                &human(TENANT_A),
            )
            .await
        {
            Err(DispatchError::Rejected(Rejection::InvalidCard(_))) => {}
            other => panic!("{bad} was accepted as a card: {other:?}"),
        }
    }
    assert_eq!(events(&db, "agent:analyst").await.len(), 2);
    db.finish().await;
}

// @req CAP-01.01
#[tokio::test]
async fn retiring_is_soft() {
    let db = TestDb::migrated().await;
    let (ledger, read, _fence) = db.ledger().await;
    put_agent(&ledger, TENANT_A, "analyst", &["echo"]).await;
    ok(
        ledger
            .dispatch(
                put("analyst", card("analyst", &[]), VersionReason::OwnerEdit),
                &human(TENANT_A),
            )
            .await,
        "v2",
    );
    let r = ok(
        ledger
            .dispatch(
                Retire::new("analyst", "superseded by analyst-2"),
                &human(TENANT_A),
            )
            .await,
        "retire",
    )
    .output;
    assert_eq!(r.seq, 3);
    let evs = events(&db, "agent:analyst").await;
    assert_eq!(evs[2].event_type, "agent.retired");
    assert_eq!(text(&evs[2], "reason"), "superseded by analyst-2");
    // History stays, nothing is deleted.
    assert_eq!(
        db.scalar_i64("SELECT count(*) FROM proj.agent_versions WHERE agent_id = 'analyst'")
            .await,
        2
    );
    match agent::head(&read, TENANT_A, "analyst").await {
        Ok(Some(h)) => assert!(h.retired && h.version_number == 2),
        other => panic!("head after retire: {other:?}"),
    }
    // Nothing new is accepted for a retired agent.
    match ledger
        .dispatch(
            put(
                "analyst",
                card("analyst", &["new"]),
                VersionReason::OwnerEdit,
            ),
            &human(TENANT_A),
        )
        .await
    {
        Err(DispatchError::Rejected(Rejection::AgentRetired { .. })) => {}
        other => panic!("a retired agent was edited: {other:?}"),
    }
    match ledger
        .dispatch(Retire::new("analyst", "again"), &human(TENANT_A))
        .await
    {
        Err(DispatchError::Rejected(Rejection::AgentRetired { .. })) => {}
        other => panic!("retired twice: {other:?}"),
    }
    match ledger
        .dispatch(Retire::new("ghost", "x"), &human(TENANT_A))
        .await
    {
        Err(DispatchError::Rejected(Rejection::AgentNotFound { .. })) => {}
        other => panic!("retired an agent that doesn't exist: {other:?}"),
    }
    assert_eq!(events(&db, "agent:analyst").await.len(), 3);
    db.finish().await;
}

// @req CAP-01.01
#[tokio::test]
async fn version_rows_are_immutable_and_heads_need_an_event() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    put_agent(&ledger, TENANT_A, "analyst", &["echo"]).await;
    let mut owner = db.session(&db.owner_url).await;
    assert_rejected(
        &mut owner,
        "UPDATE proj.agent_versions SET reason = 'owner_edit' WHERE agent_id = 'analyst'",
        "ABW-T6",
    )
    .await;
    assert_rejected(&mut owner, "DELETE FROM proj.agent_versions", "ABW-T6").await;
    assert_rejected(
        &mut owner,
        "UPDATE proj.agent_heads SET retired = true WHERE agent_id = 'analyst'",
        "ABW-T3",
    )
    .await;
    for table in ["proj.agent_versions", "proj.agent_heads"] {
        match admin::rebuild_diff(&db.owner_url, table).await {
            Ok(diff) => assert!(diff.is_empty(), "{table}: {diff:?}"),
            Err(e) => panic!("rebuild {table}: {e:?}"),
        }
    }
    db.finish().await;
}
