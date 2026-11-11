//! Grant and price policies as human-accepted policy versions (ADR-0008 §4 `policy`;
//! target-system-design §3.1.7, §11.2, §11.4; economy.md §2 "Grant policy", "Price policy").
//!
//! ECO-3 needs every grant to cite "an active, human-accepted grant_policy_version"; these
//! tests show what makes a policy version exist and which version is active.

mod common;
#[path = "common/m2.rs"]
mod m2;

use abw_ledger::policy::{self, Accept, PolicyKind};
use abw_ledger::{Actor, CommandCtx, DispatchError, Expected, PLATFORM_TENANT, Rejection};
use common::TestDb;
use m2::*;
use serde_json::json;
use uuid::Uuid;

// @req ECO-3
#[tokio::test]
async fn only_a_human_with_a_rationale_can_accept_a_policy() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    for actor in [
        Actor::agent("outer_loop"),
        Actor::platform("seeder"),
        Actor::tool("calc"),
    ] {
        let cmd = Accept::new(
            Uuid::new_v4(),
            PolicyKind::Grant,
            grant_doc(100, 50, 1000),
            "looks right",
            policy_epoch(),
        );
        match ledger
            .dispatch(cmd, &CommandCtx::new(TENANT_A, actor.clone()))
            .await
        {
            Err(DispatchError::Rejected(Rejection::NotHuman { .. })) => {}
            other => panic!("{actor:?} accepted a policy: {other:?}"),
        }
    }
    let blank = Accept::new(
        Uuid::new_v4(),
        PolicyKind::Grant,
        grant_doc(100, 50, 1000),
        "   ",
        policy_epoch(),
    );
    match ledger.dispatch(blank, &human(TENANT_A)).await {
        Err(DispatchError::Rejected(Rejection::ReasonRequired)) => {}
        other => panic!("a policy without a rationale was accepted: {other:?}"),
    }
    assert_eq!(
        db.scalar_i64("SELECT count(*) FROM ledger.policy_versions")
            .await,
        0
    );

    // The conforming twin: a human with a rationale.
    let id = Uuid::new_v4();
    let cmd = Accept::new(
        id,
        PolicyKind::Grant,
        grant_doc(100, 50, 1000),
        "pilot values",
        policy_epoch(),
    );
    let r = ok(ledger.dispatch(cmd, &human(TENANT_A)).await, "accept").output;
    assert_eq!((r.policy_id, r.version), (id, 1));
    assert_eq!(r.to_string(), format!("{id}@1"));
    let stored = db
        .scalar_i64(&format!(
            "SELECT count(*) FROM ledger.policy_versions
              WHERE policy_id = '{id}' AND version = 1 AND policy_kind = 'grant'
                AND accepted_by = 'ivan' AND doc_sha = '{}' AND tenant_id = '{TENANT_A}'",
            r.doc_sha
        ))
        .await;
    assert_eq!(stored, 1, "policy_versions row written with the event");
    let evs = events(&db, &format!("policy:{}", id.hyphenated())).await;
    assert_eq!(evs.len(), 1);
    assert_eq!(evs[0].event_type, "policy.adjusted");
    assert_eq!(evs[0].actor_kind, "human");
    db.finish().await;
}

// @req ECO-3
#[tokio::test]
async fn policy_documents_are_validated_for_their_kind() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let bad = [
        (PolicyKind::Grant, price_doc(1, 1, 1, 0, 100)),
        (PolicyKind::Price, grant_doc(1, 1, 1)),
        (
            PolicyKind::Grant,
            json!({"policy_kind": "grant", "onboarding_cu": -5, "free_tier_cap_cu": 1, "operator_limit_cu": 1}),
        ),
        (
            PolicyKind::Grant,
            json!({"policy_kind": "grant", "onboarding_cu": 1.5, "free_tier_cap_cu": 1, "operator_limit_cu": 1}),
        ),
        (
            PolicyKind::Grant,
            json!({"policy_kind": "grant", "onboarding_cu": 1, "free_tier_cap_cu": 1}),
        ),
        (
            PolicyKind::Grant,
            json!({"policy_kind": "grant", "onboarding_cu": 1, "free_tier_cap_cu": 1,
                   "operator_limit_cu": 1, "transfer_limit_cu": 9}),
        ),
    ];
    for (kind, doc) in bad {
        let cmd = Accept::new(Uuid::new_v4(), kind, doc.clone(), "x", policy_epoch());
        match ledger.dispatch(cmd, &human(TENANT_A)).await {
            Err(DispatchError::Rejected(Rejection::PolicyInvalid(_))) => {}
            other => panic!("{kind:?} accepted {doc}: {other:?}"),
        }
    }
    accept(
        &ledger,
        TENANT_A,
        PolicyKind::Price,
        price_doc(1, 2, 3, 4, 500),
    )
    .await;
    accept(&ledger, TENANT_A, PolicyKind::Grant, grant_doc(0, 0, 0)).await;
    db.finish().await;
}

// @req ECO-3
#[tokio::test]
async fn versions_follow_the_stream_and_history_is_immutable() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let id = Uuid::new_v4();
    let v1 = ok(
        ledger
            .dispatch(
                Accept::new(
                    id,
                    PolicyKind::Grant,
                    grant_doc(10, 10, 10),
                    "v1",
                    policy_epoch(),
                ),
                &human(TENANT_A),
            )
            .await,
        "v1",
    )
    .output;
    let v2 = ok(
        ledger
            .dispatch(
                Accept::new(
                    id,
                    PolicyKind::Grant,
                    grant_doc(20, 10, 10),
                    "v2",
                    policy_epoch(),
                )
                .expected(Expected::Exact(1)),
                &human(TENANT_A),
            )
            .await,
        "v2",
    )
    .output;
    assert_eq!((v1.version, v2.version), (1, 2));
    assert_ne!(v1.doc_sha, v2.doc_sha);
    let stale = Accept::new(
        id,
        PolicyKind::Grant,
        grant_doc(30, 10, 10),
        "v3",
        policy_epoch(),
    )
    .expected(Expected::Exact(1));
    match ledger.dispatch(stale, &human(TENANT_A)).await {
        Err(DispatchError::Conflict { actual: 2, .. }) => {}
        other => panic!("a stale policy version was accepted: {other:?}"),
    }
    let mut owner = db.session(&db.owner_url).await;
    common::assert_rejected(
        &mut owner,
        &format!("UPDATE ledger.policy_versions SET doc = '{{}}' WHERE policy_id = '{id}'"),
        "ABW-T6",
    )
    .await;
    db.finish().await;
}

async fn active_grant(
    read: &abw_ledger::ReadDb,
    tenant: &str,
    when: chrono::DateTime<chrono::Utc>,
) -> Option<abw_ledger::policy::PolicyRef> {
    ok(
        policy::active(read, PolicyKind::Grant, tenant, when).await,
        "policy::active",
    )
}

// @req ECO-3
#[tokio::test]
async fn the_active_policy_is_the_tenants_latest_then_the_platform_default() {
    let db = TestDb::migrated().await;
    let (ledger, read, _fence) = db.ledger().await;
    let when = at(2026, 11, 15, 12, 0);
    assert_eq!(
        active_grant(&read, TENANT_A, when).await,
        None,
        "no policy yet"
    );

    let platform = accept(
        &ledger,
        PLATFORM_TENANT,
        PolicyKind::Grant,
        grant_doc(1, 1, 1),
    )
    .await;
    assert_eq!(
        active_grant(&read, TENANT_A, when).await,
        Some(platform.clone()),
        "platform default"
    );

    let id = Uuid::new_v4();
    let tenant_v1 = ok(
        ledger
            .dispatch(
                Accept::new(
                    id,
                    PolicyKind::Grant,
                    grant_doc(2, 2, 2),
                    "t1",
                    policy_epoch(),
                ),
                &human(TENANT_A),
            )
            .await,
        "tenant v1",
    )
    .output;
    assert_eq!(
        active_grant(&read, TENANT_A, when).await,
        Some(tenant_v1.clone())
    );
    assert_eq!(
        active_grant(&read, TENANT_B, when).await,
        Some(platform),
        "B still gets the default"
    );

    // A version that becomes active only after `when` doesn't count yet.
    let future = ok(
        ledger
            .dispatch(
                Accept::new(
                    id,
                    PolicyKind::Grant,
                    grant_doc(3, 3, 3),
                    "t2",
                    at(2027, 1, 1, 0, 0),
                )
                .expected(Expected::Exact(1)),
                &human(TENANT_A),
            )
            .await,
        "tenant v2",
    )
    .output;
    assert_eq!(active_grant(&read, TENANT_A, when).await, Some(tenant_v1));
    match policy::active(&read, PolicyKind::Grant, TENANT_A, at(2027, 2, 1, 0, 0)).await {
        Ok(Some(r)) => assert_eq!(r, future),
        other => panic!("v2 should be active in 2027: {other:?}"),
    }
    // Kinds don't mix.
    match policy::active(&read, PolicyKind::Price, TENANT_A, when).await {
        Ok(None) => {}
        other => panic!("a grant policy resolved as a price policy: {other:?}"),
    }
    db.finish().await;
}
