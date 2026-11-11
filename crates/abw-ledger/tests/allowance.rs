#![cfg(any())]
//! The allowance aggregate (economy.md §3-§4; ADR-0006; ADR-0008 §2-§4): grants follow a
//! human-accepted grant policy, no event moves allowance between owners, every allowance
//! event is idempotent, and the database guards the projection.
//!
//! T9 is superseded by ECO-1..ECO-7 (register note), so these tests carry T9 as well. The
//! ECO-4 and ECO-6 tests also carry CAP-12.03 (ADR-0008 §1 item 4).

mod common;
#[path = "common/m2.rs"]
mod m2;

use abw_ledger::{Actor, CommandCtx, DispatchError, Rejection, admin, allowance};
use common::{TestDb, assert_rejected};
use m2::*;
use uuid::Uuid;

// @req ECO-3, ECO-1, T9
#[tokio::test]
async fn opening_an_allowance_grants_the_onboarding_amount_once() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let grant = accept_grant(&ledger, TENANT_A, 250, 100, 1000).await;
    let opened = open_allowance(&ledger, TENANT_A, "owner-1", &grant).await;
    assert_eq!(opened.onboarding_cu, 250);
    assert_eq!(
        allowance_row(&db, "owner-1").await,
        Allowance {
            free_tier: 0,
            granted: 250,
            purchased: 0,
            held: 0,
            available: 250,
            total_consumed: 0
        }
    );
    let evs = events(&db, &allowance_stream("owner-1")).await;
    let types: Vec<&str> = evs.iter().map(|e| e.event_type.as_str()).collect();
    assert_eq!(types, ["allowance.opened", "allowance.granted"]);
    assert_eq!(
        evs[0].txid, evs[1].txid,
        "opened and onboarding commit together"
    );
    assert_eq!(text(&evs[0], "idempotency_key"), "open:owner-1");
    assert_eq!(text(&evs[1], "kind"), "onboarding");
    assert_eq!(int(&evs[1], "amount_cu"), 250);
    assert_eq!(text(&evs[1], "grant_policy_version"), grant.to_string());
    assert_eq!(text(&evs[1], "idempotency_key"), "onboarding:owner-1");
    db.finish().await;
}

// @req ECO-3, ECO-6, CAP-12.03, T9
#[tokio::test]
async fn onboarding_is_granted_once_per_owner_even_when_retried() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let grant = accept_grant(&ledger, TENANT_A, 250, 100, 1000).await;
    open_allowance(&ledger, TENANT_A, "owner-1", &grant).await;

    // An identical retry replays the stored result and appends nothing.
    let retry = ok(
        ledger
            .dispatch(
                allowance::Open::new("owner-1", grant.clone()),
                &human(TENANT_A),
            )
            .await,
        "identical retry",
    );
    assert!(retry.replayed, "an identical retry must replay");

    // Any other open for the same owner (here under a richer policy) is refused.
    let richer = accept_grant(&ledger, TENANT_A, 9_000, 100, 1000).await;
    match ledger
        .dispatch(allowance::Open::new("owner-1", richer), &human(TENANT_A))
        .await
    {
        Err(DispatchError::IdempotencyMismatch { .. }) => {}
        other => panic!("a second onboarding was not refused: {other:?}"),
    }
    assert_eq!(allowance_row(&db, "owner-1").await.granted, 250);
    assert_eq!(
        events_of_type(&db, "allowance.granted").await.len(),
        1,
        "exactly one onboarding grant"
    );
    db.finish().await;
}

// @req ECO-3, T9
#[tokio::test]
async fn a_grant_without_a_valid_grant_policy_is_rejected() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let real = accept_grant(&ledger, TENANT_A, 10, 10, 1000).await;
    let price = accept_price(&ledger, TENANT_A, 1, 100).await;
    let other_tenant = accept_grant(&ledger, TENANT_B, 10, 10, 1000).await;

    let mut unknown = real.clone();
    unknown.policy_id = Uuid::new_v4();
    let mut wrong_version = real.clone();
    wrong_version.version = 7;
    let mut wrong_sha = real.clone();
    wrong_sha.doc_sha = price.doc_sha.clone();

    for (what, r) in [
        ("unknown policy", unknown),
        ("unknown version", wrong_version),
        ("doc_sha mismatch", wrong_sha),
    ] {
        match ledger
            .dispatch(allowance::Open::new("owner-1", r), &human(TENANT_A))
            .await
        {
            Err(DispatchError::Rejected(Rejection::PolicyNotFound { .. })) => {}
            other => panic!("{what}: {other:?}"),
        }
    }
    match ledger
        .dispatch(allowance::Open::new("owner-1", price), &human(TENANT_A))
        .await
    {
        Err(DispatchError::Rejected(Rejection::PolicyKindMismatch { .. })) => {}
        other => panic!("a price policy granted allowance: {other:?}"),
    }
    match ledger
        .dispatch(
            allowance::Open::new("owner-1", other_tenant),
            &human(TENANT_A),
        )
        .await
    {
        Err(DispatchError::Rejected(
            Rejection::PolicyNotFound { .. } | Rejection::TenantMismatch { .. },
        )) => {}
        other => panic!("another tenant's policy granted allowance: {other:?}"),
    }
    assert_eq!(
        db.scalar_i64("SELECT count(*) FROM ledger.events WHERE event_type LIKE 'allowance.%'")
            .await,
        0
    );
    open_allowance(&ledger, TENANT_A, "owner-1", &real).await;
    db.finish().await;
}

// @req ECO-3, T9
#[tokio::test]
async fn operator_grants_need_a_human_operator_and_a_reason() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let grant = accept_grant(&ledger, TENANT_A, 0, 0, 1000).await;
    open_allowance(&ledger, TENANT_A, "owner-1", &grant).await;
    for actor in [
        Actor::agent("helpful_agent"),
        Actor::platform("scheduler"),
        Actor::tool("grant_tool"),
    ] {
        let cmd = allowance::OperatorGrant::new(
            "owner-1",
            Uuid::new_v4(),
            100,
            "pilot tenant",
            grant.clone(),
        );
        match ledger
            .dispatch(cmd, &CommandCtx::new(TENANT_A, actor.clone()))
            .await
        {
            Err(DispatchError::Rejected(Rejection::NotHuman { .. })) => {}
            other => panic!("{actor:?} made an operator grant: {other:?}"),
        }
    }
    for reason in ["", "  \n "] {
        let cmd =
            allowance::OperatorGrant::new("owner-1", Uuid::new_v4(), 100, reason, grant.clone());
        match ledger.dispatch(cmd, &human(TENANT_A)).await {
            Err(DispatchError::Rejected(Rejection::ReasonRequired)) => {}
            other => panic!("an operator grant without a reason: {other:?}"),
        }
    }
    assert_eq!(allowance_row(&db, "owner-1").await.granted, 0);

    let id = Uuid::new_v4();
    let g = ok(
        ledger
            .dispatch(
                allowance::OperatorGrant::new("owner-1", id, 100, "pilot tenant", grant.clone()),
                &human(TENANT_A),
            )
            .await,
        "operator grant",
    )
    .output;
    assert_eq!(g.amount_cu, 100);
    let row = allowance_row(&db, "owner-1").await;
    assert_eq!((row.granted, row.free_tier, row.available), (100, 0, 100));
    let ev = match events_of_type(&db, "allowance.granted").await.pop() {
        Some(e) => e,
        None => panic!("no grant event"),
    };
    assert_eq!(text(&ev, "kind"), "operator");
    assert_eq!(text(&ev, "reason"), "pilot tenant");
    assert_eq!(text(&ev, "grant_id"), id.to_string());
    assert_eq!(text(&ev, "idempotency_key"), format!("operator:{id}"));
    assert_eq!(ev.actor_kind, "human");
    db.finish().await;
}

// @req ECO-3, T9
#[tokio::test]
async fn operator_grants_stay_within_the_per_owner_limit() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let grant = accept_grant(&ledger, TENANT_A, 50, 0, 300).await;
    open_allowance(&ledger, TENANT_A, "owner-1", &grant).await;
    open_allowance(&ledger, TENANT_A, "owner-2", &grant).await;
    let give = |owner: &'static str, amount: u64| {
        allowance::OperatorGrant::new(owner, Uuid::new_v4(), amount, "pilot", grant.clone())
    };
    ok(
        ledger
            .dispatch(give("owner-1", 200), &human(TENANT_A))
            .await,
        "200",
    );
    ok(
        ledger
            .dispatch(give("owner-1", 100), &human(TENANT_A))
            .await,
        "300 total",
    );
    match ledger.dispatch(give("owner-1", 1), &human(TENANT_A)).await {
        Err(DispatchError::Rejected(Rejection::GrantLimitExceeded {
            limit: 300,
            requested_total: 301,
        })) => {}
        other => panic!("an operator grant past the limit: {other:?}"),
    }
    // The limit is per owner and counts operator grants only (onboarding is separate).
    ok(
        ledger
            .dispatch(give("owner-2", 300), &human(TENANT_A))
            .await,
        "owner-2",
    );
    match ledger
        .dispatch(give("owner-2", 301), &human(TENANT_A))
        .await
    {
        Err(DispatchError::Rejected(Rejection::GrantLimitExceeded { .. })) => {}
        other => panic!("a single grant above the limit: {other:?}"),
    }
    assert_eq!(allowance_row(&db, "owner-1").await.granted, 350);
    assert_eq!(allowance_row(&db, "owner-2").await.granted, 350);
    db.finish().await;
}

// @req ECO-6, CAP-12.03, T9
#[tokio::test]
async fn a_retried_operator_grant_never_grants_twice() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let grant = accept_grant(&ledger, TENANT_A, 0, 0, 1000).await;
    open_allowance(&ledger, TENANT_A, "owner-1", &grant).await;
    let id = Uuid::new_v4();
    let cmd = || allowance::OperatorGrant::new("owner-1", id, 120, "pilot", grant.clone());
    let first = ok(ledger.dispatch(cmd(), &human(TENANT_A)).await, "first");
    let again = ok(ledger.dispatch(cmd(), &human(TENANT_A)).await, "retry");
    assert!(!first.replayed);
    assert!(again.replayed);
    assert_eq!(again.output.amount_cu, first.output.amount_cu);
    let different = allowance::OperatorGrant::new("owner-1", id, 999, "pilot", grant.clone());
    match ledger.dispatch(different, &human(TENANT_A)).await {
        Err(DispatchError::IdempotencyMismatch { .. }) => {}
        other => panic!("one grant id granted two amounts: {other:?}"),
    }
    assert_eq!(allowance_row(&db, "owner-1").await.granted, 120);
    db.finish().await;
}

// @req ECO-3, ECO-4, CAP-12.03, T9
#[tokio::test]
async fn grants_need_an_open_allowance_in_the_callers_tenant() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let grant_a = accept_grant(&ledger, TENANT_A, 0, 0, 1000).await;
    let grant_b = accept_grant(&ledger, TENANT_B, 0, 0, 1000).await;
    let cmd = allowance::OperatorGrant::new("nobody", Uuid::new_v4(), 5, "x", grant_a.clone());
    match ledger.dispatch(cmd, &human(TENANT_A)).await {
        Err(DispatchError::Rejected(Rejection::AllowanceNotOpen { .. })) => {}
        other => panic!("a grant created an allowance: {other:?}"),
    }
    open_allowance(&ledger, TENANT_B, "owner-b", &grant_b).await;
    let cross = allowance::OperatorGrant::new("owner-b", Uuid::new_v4(), 5, "x", grant_a);
    match ledger.dispatch(cross, &human(TENANT_A)).await {
        Err(DispatchError::Rejected(Rejection::TenantMismatch { .. })) => {}
        other => panic!("tenant A granted into tenant B's allowance: {other:?}"),
    }
    let own = allowance::OperatorGrant::new("owner-b", Uuid::new_v4(), 5, "x", grant_b);
    ok(
        ledger.dispatch(own, &human(TENANT_B)).await,
        "tenant B grants its own",
    );
    db.finish().await;
}

// @req ECO-4, CAP-12.03, T9
#[tokio::test]
async fn the_allowance_event_types_are_closed_and_none_transfers() {
    let db = TestDb::migrated().await;
    let types: Vec<String> = match sqlx::query_scalar(
        "SELECT event_type FROM ledger.event_types WHERE stream_kind = 'allowance' ORDER BY 1",
    )
    .fetch_all(&db.admin)
    .await
    {
        Ok(t) => t,
        Err(e) => panic!("{e}"),
    };
    assert_eq!(
        types,
        [
            "allowance.consumed",
            "allowance.granted",
            "allowance.hold_placed",
            "allowance.hold_released",
            "allowance.opened",
        ],
        "allowance.funded arrives with M4 (ECO-2); nothing else may touch an allowance"
    );
    let epistemic = db
        .scalar_i64(
            "SELECT count(*) FROM ledger.event_types
              WHERE stream_kind = 'allowance' AND epistemic <> 'evidentiary'",
        )
        .await;
    assert_eq!(epistemic, 0);
    // No registered event type of any kind names a transfer or a second owner.
    let transfer_like = db
        .scalar_i64(
            "SELECT count(*) FROM ledger.event_types
              WHERE event_type ~ '(transfer|funded|deposit|debit|credit|mint)'
                 OR json_schema::text ~ '(to_owner|from_owner|recipient|payee|counterparty)'",
        )
        .await;
    assert_eq!(transfer_like, 0);
    db.finish().await;
}

// @req ECO-4, ECO-6, CAP-12.03, T9
#[tokio::test]
async fn every_allowance_event_names_only_its_own_owner_and_an_idempotency_key() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let grant = accept_grant(&ledger, TENANT_A, 100, 40, 1000).await;
    let price = accept_price(&ledger, TENANT_A, 1, 500).await;
    for owner in ["owner-1", "owner-2"] {
        open_allowance(&ledger, TENANT_A, owner, &grant).await;
        put_agent(&ledger, TENANT_A, &format!("agent-{owner}"), &[]).await;
        let eid = Uuid::new_v4();
        let req = open_request(
            eid,
            owner,
            &format!("agent-{owner}"),
            50,
            &price,
            &grant,
            at(2026, 11, 3, 9, 0),
        );
        open_episode(&ledger, TENANT_A, req).await;
        let close = abw_ledger::episode::Close::new(
            eid,
            abw_ledger::Expected::Exact(1),
            abw_ledger::episode::CloseOutcome {
                status: abw_ledger::episode::EpisodeStatus::Success,
                reason_class: None,
                raw_output: Some("done".into()),
                usage: usage(30, 0, 0),
            },
        );
        ok(ledger.dispatch(close, &platform(TENANT_A)).await, "close");
    }
    let all = match sqlx::query_scalar::<_, String>(
        "SELECT stream_id FROM ledger.streams WHERE kind = 'allowance'",
    )
    .fetch_all(&db.admin)
    .await
    {
        Ok(s) => s,
        Err(e) => panic!("{e}"),
    };
    assert_eq!(all.len(), 2);
    let mut seen = 0;
    for stream in all {
        let owner = stream.trim_start_matches("allowance:").to_string();
        for ev in events(&db, &stream).await {
            seen += 1;
            assert_eq!(
                text(&ev, "owner_id"),
                owner,
                "{} on {stream}",
                ev.event_type
            );
            assert!(!text(&ev, "idempotency_key").is_empty());
            // A payload that names any other owner would be a transfer in disguise.
            let body = ev.payload.to_string();
            let other = if owner == "owner-1" {
                "owner-2"
            } else {
                "owner-1"
            };
            assert!(
                !body.contains(other),
                "{} names {other}: {body}",
                ev.event_type
            );
            if let Some(eid) = ev.payload.get("episode_id").and_then(|v| v.as_str()) {
                let row = episode_row(
                    &db,
                    match Uuid::parse_str(eid) {
                        Ok(u) => u,
                        Err(e) => panic!("{eid}: {e}"),
                    },
                )
                .await;
                assert_eq!(
                    row.owner_id, owner,
                    "the cited episode is owned by the stream's owner"
                );
            }
        }
    }
    // opened, onboarding, free tier, hold, consumed for each owner.
    assert_eq!(seen, 10);
    let dupes = db
        .scalar_i64(
            "SELECT count(*) FROM (SELECT stream_id, payload->>'idempotency_key' FROM ledger.events
              WHERE stream_id LIKE 'allowance:%' GROUP BY 1, 2 HAVING count(*) > 1) d",
        )
        .await;
    assert_eq!(dupes, 0);
    db.finish().await;
}

// @req ECO-1, T9
#[tokio::test]
async fn the_allowance_projection_cannot_change_without_an_event() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let grant = accept_grant(&ledger, TENANT_A, 100, 0, 1000).await;
    open_allowance(&ledger, TENANT_A, "owner-1", &grant).await;
    let before = allowance_row(&db, "owner-1").await;
    // 42501 is both the T3 trigger's code and a missing grant (design §13.2 may give abw_app
    // no UPDATE on money projections at all); the owner, which owns the table, reaches the
    // trigger.
    for (url, needle) in [(&db.app_url, "42501"), (&db.owner_url, "ABW-T3")] {
        let mut conn = db.session(url).await;
        assert_rejected(
            &mut conn,
            "UPDATE proj.allowances SET granted = granted + 1000000, available = available + 1000000
              WHERE owner_id = 'owner-1'",
            needle,
        )
        .await;
        assert_rejected(
            &mut conn,
            "INSERT INTO proj.allowances (owner_id, stream_id, tenant_id, free_tier, granted, purchased,
                 held, available, total_granted_free_tier, total_granted, total_purchased,
                 total_consumed, applied_pos)
             SELECT 'minted', 'allowance:minted', 'tenant-a', 0, 5000, 0, 0, 5000, 0, 5000, 0, 0,
                    max(global_pos) FROM ledger.events",
            needle,
        )
        .await;
    }
    let mut owner = db.session(&db.owner_url).await;
    assert_rejected(&mut owner, "DELETE FROM proj.allowances", "ABW-T3").await;
    let mut read = db.session(&db.read_url).await;
    assert_rejected(
        &mut read,
        "SET TRANSACTION READ WRITE; UPDATE proj.allowances SET held = 0",
        "permission denied",
    )
    .await;
    assert_eq!(allowance_row(&db, "owner-1").await, before);
    db.finish().await;
}

// @req ECO-7, T9
#[tokio::test]
async fn the_database_refuses_negative_or_inconsistent_balances() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let grant = accept_grant(&ledger, TENANT_A, 100, 0, 1000).await;
    open_allowance(&ledger, TENANT_A, "owner-1", &grant).await;
    let mut owner = db.session(&db.owner_url).await;
    // Each update is justified by an event in its transaction, so only the CHECK can refuse.
    for set in [
        "free_tier = -1, available = available - 1",
        "granted = -1, available = available - 101",
        "purchased = -1, available = available - 1",
        "held = -1, available = available + 1",
        "held = 101, available = -1",
        "total_consumed = -1",
        "available = available + 1",
    ] {
        let sql = with_justifying_event(&format!(
            "UPDATE proj.allowances SET {set}, applied_pos = {{pos}} WHERE owner_id = 'owner-1'"
        ));
        assert_rejected(&mut owner, &sql, "23514").await;
    }
    // The conforming twin: the same justified update with consistent values commits.
    assert!(
        commits_as_owner(
            &db,
            &with_justifying_event(
                "UPDATE proj.allowances SET held = 40, available = 60, applied_pos = {pos}
                  WHERE owner_id = 'owner-1'"
            )
        )
        .await,
        "a consistent justified update must commit, or the refusals above prove nothing"
    );
    db.finish().await;
}

// @req ECO-4, CAP-12.03, T9
#[tokio::test]
async fn the_database_refuses_a_hold_citing_another_owners_episode() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let grant = accept_grant(&ledger, TENANT_A, 100, 0, 1000).await;
    let price = accept_price(&ledger, TENANT_A, 1, 500).await;
    open_allowance(&ledger, TENANT_A, "owner-a", &grant).await;
    open_allowance(&ledger, TENANT_A, "owner-b", &grant).await;
    put_agent(&ledger, TENANT_A, "agent-1", &[]).await;
    let eid = Uuid::new_v4();
    open_episode(
        &ledger,
        TENANT_A,
        open_request(
            eid,
            "owner-a",
            "agent-1",
            10,
            &price,
            &grant,
            at(2026, 11, 3, 9, 0),
        ),
    )
    .await;
    let mut owner = db.session(&db.owner_url).await;
    // Re-pointing the hold at owner-b's allowance while it cites owner-a's episode.
    let forged = with_justifying_event(&format!(
        "UPDATE proj.allowance_holds SET owner_id = 'owner-b', applied_pos = {{pos}}
          WHERE episode_id = '{eid}'"
    ));
    assert_rejected(&mut owner, &forged, "23503").await;
    assert_eq!(hold_status(&db, eid).await, "open");
    // The conforming twin cites the episode with its real owner (the UNIQUE episode_id then
    // refuses a second hold, which is the other half of "one hold per episode").
    let second = with_justifying_event(&format!(
        "INSERT INTO proj.allowance_holds (hold_id, owner_id, episode_id, tenant_id, amount_cu, status, applied_pos)
         VALUES ('{}', 'owner-a', '{eid}', 'tenant-a', 10, 'open', {{pos}})",
        Uuid::new_v4()
    ));
    assert_rejected(&mut owner, &second, "23505").await;
    db.finish().await;
}

// @req ECO-1, T9
#[tokio::test]
async fn rebuilding_allowances_from_events_reproduces_the_projection() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let grant = accept_grant(&ledger, TENANT_A, 100, 40, 1000).await;
    let price = accept_price(&ledger, TENANT_A, 1, 500).await;
    open_allowance(&ledger, TENANT_A, "owner-1", &grant).await;
    put_agent(&ledger, TENANT_A, "agent-1", &[]).await;
    ok(
        ledger
            .dispatch(
                allowance::OperatorGrant::new(
                    "owner-1",
                    Uuid::new_v4(),
                    70,
                    "pilot",
                    grant.clone(),
                ),
                &human(TENANT_A),
            )
            .await,
        "operator grant",
    );
    for (i, day) in [3u32, 4].iter().enumerate() {
        let eid = Uuid::new_v4();
        open_episode(
            &ledger,
            TENANT_A,
            open_request(
                eid,
                "owner-1",
                "agent-1",
                60,
                &price,
                &grant,
                at(2026, 11, *day, 9, 0),
            ),
        )
        .await;
        if i == 0 {
            let close = abw_ledger::episode::Close::new(
                eid,
                abw_ledger::Expected::Exact(1),
                abw_ledger::episode::CloseOutcome {
                    status: abw_ledger::episode::EpisodeStatus::Success,
                    reason_class: None,
                    raw_output: None,
                    usage: usage(55, 0, 0),
                },
            );
            ok(ledger.dispatch(close, &platform(TENANT_A)).await, "close");
        }
    }
    for table in ["proj.allowances", "proj.allowance_holds", "proj.episodes"] {
        match admin::rebuild_diff(&db.owner_url, table).await {
            Ok(diff) => assert!(diff.is_empty(), "{table} differs from its events: {diff:?}"),
            Err(e) => panic!("rebuild {table}: {e:?}"),
        }
    }
    db.finish().await;
}
