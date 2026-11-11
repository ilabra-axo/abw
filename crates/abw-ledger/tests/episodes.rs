#![cfg(any())]
//! Episodes as streams with TX-A / TX-B / TX-C at the ledger level (target-system-design
//! §4.5-§4.7; economy.md §3, §5; ADR-0008 §4 `episode`, `reaper`).
//!
//! The executor drives these commands in `abw-agent`; here each transaction is dispatched
//! directly so its atomicity, refusals and idempotency are visible on their own.

mod common;
#[path = "common/m2.rs"]
mod m2;

use abw_ledger::episode::{
    Abandon, Close, CloseOutcome, EpisodeStatus, ReasonClass, RecordTool, ToolOutcome, ToolRecord,
};
use abw_ledger::policy::PolicyRef;
use abw_ledger::{
    DispatchError, Expected, FaultPoint, Ledger, Rejection, agent, allowance, artifact_json, reaper,
};
use common::TestDb;
use m2::*;
use serde_json::{Value, json};
use uuid::Uuid;

const OWNER: &str = "owner-1";
const AGENT: &str = "agent-1";

/// A ledger with a grant policy (onboarding 100, free tier cap 40, operator limit 1000), a
/// price policy (1 CU per input token, holds up to 500), `OWNER`'s allowance and `AGENT`
/// allowed to call `echo`.
struct World {
    db: TestDb,
    ledger: Ledger,
    read: abw_ledger::ReadDb,
    _fence: abw_ledger::SingletonFence,
    grant: PolicyRef,
    price: PolicyRef,
}

async fn world() -> World {
    let db = TestDb::migrated().await;
    let (ledger, read, fence) = db.ledger().await;
    let grant = accept_grant(&ledger, TENANT_A, 100, 40, 1000).await;
    let price = accept_price(&ledger, TENANT_A, 1, 500).await;
    open_allowance(&ledger, TENANT_A, OWNER, &grant).await;
    put_agent(&ledger, TENANT_A, AGENT, &["echo"]).await;
    World {
        db,
        ledger,
        read,
        _fence: fence,
        grant,
        price,
    }
}

impl World {
    fn req(&self, eid: Uuid, hold: u64) -> abw_ledger::episode::OpenEpisode {
        open_request(
            eid,
            OWNER,
            AGENT,
            hold,
            &self.price,
            &self.grant,
            at(2026, 11, 3, 9, 0),
        )
    }

    async fn open(&self, hold: u64) -> (Uuid, abw_ledger::episode::Opened) {
        let eid = Uuid::new_v4();
        let o = open_episode(&self.ledger, TENANT_A, self.req(eid, hold)).await;
        (eid, o)
    }

    async fn close(
        &self,
        eid: Uuid,
        expected: u64,
        outcome: CloseOutcome,
    ) -> Result<abw_ledger::Dispatched<abw_ledger::episode::Closed>, DispatchError> {
        self.ledger
            .dispatch(
                Close::new(eid, Expected::Exact(expected), outcome),
                &platform(TENANT_A),
            )
            .await
    }
}

fn success(input_tokens: u64) -> CloseOutcome {
    CloseOutcome {
        status: EpisodeStatus::Success,
        reason_class: None,
        raw_output: Some("final answer".into()),
        usage: usage(input_tokens, 0, 0),
    }
}

fn echo_record(iteration: u32, call_index: u32, input: Value) -> ToolRecord {
    ToolRecord {
        iteration,
        call_index,
        tool_name: "echo".into(),
        tool_version: "1".into(),
        allowed: true,
        output: ToolOutcome::Ok(json!({"echo": input.clone()})),
        input,
        duration_ms: 3,
    }
}

// @req ECO-5, CAP-12.01, T9
#[tokio::test]
async fn opening_an_episode_places_the_hold_in_the_same_transaction() {
    let w = world().await;
    let (eid, opened) = w.open(60).await;
    assert_eq!(opened.seq, 1, "episode.opened is the stream's first event");
    assert_eq!(opened.version_number, 1);

    let ep = events(&w.db, &episode_stream(eid)).await;
    let al = events(&w.db, &allowance_stream(OWNER)).await;
    assert_eq!(ep.len(), 1);
    assert_eq!(ep[0].event_type, "episode.opened");
    let hold = match al.iter().find(|e| e.event_type == "allowance.hold_placed") {
        Some(h) => h,
        None => panic!("no hold_placed event"),
    };
    assert_eq!(hold.txid, ep[0].txid, "hold and episode commit in one TX-A");
    assert_eq!(int(hold, "amount_cu"), 60);
    assert_eq!(text(hold, "episode_id"), eid.to_string());
    assert_eq!(text(hold, "hold_id"), opened.hold_id.to_string());
    assert_eq!(text(hold, "idempotency_key"), format!("episode:{eid}:hold"));
    assert!(hold.payload.get("workspace_id").is_some_and(Value::is_null));
    assert_eq!(text(&ep[0], "hold_id"), opened.hold_id.to_string());
    assert_eq!(text(&ep[0], "card_sha"), opened.card_sha.to_string());
    assert_eq!(text(&ep[0], "price_version"), w.price.to_string());
    assert_eq!(ep[0].payload.get("tools_allowed"), Some(&json!(["echo"])));

    // Onboarding 100 + free tier 40 = 140; 60 held.
    let row = allowance_row(&w.db, OWNER).await;
    assert_eq!((row.held, row.available), (60, 80));
    let e = episode_row(&w.db, eid).await;
    assert_eq!((e.status.as_str(), e.hold_cu), ("running", 60));
    assert_eq!(hold_status(&w.db, eid).await, "open");
    w.db.finish().await;
}

// @req ECO-5, ECO-7, CAP-12.01, T9
#[tokio::test]
async fn an_episode_the_allowance_cannot_cover_is_refused_before_any_work() {
    let w = world().await;
    let before = allowance_row(&w.db, OWNER).await;
    let eid = Uuid::new_v4();
    // 140 available after the free-tier top-up, which is part of the refused transaction.
    match w
        .ledger
        .dispatch(
            abw_ledger::episode::Open::new(w.req(eid, 141)),
            &platform(TENANT_A),
        )
        .await
    {
        Err(DispatchError::Rejected(Rejection::InsufficientAllowance {
            available: 140,
            requested: 141,
        })) => {}
        other => panic!("an uncovered hold was placed: {other:?}"),
    }
    match w
        .ledger
        .dispatch(
            abw_ledger::episode::Open::new(w.req(eid, 0)),
            &platform(TENANT_A),
        )
        .await
    {
        Err(DispatchError::Rejected(Rejection::InsufficientAllowance { .. })) => {}
        other => panic!("a zero hold was accepted: {other:?}"),
    }
    assert_eq!(allowance_row(&w.db, OWNER).await, before, "nothing written");
    assert!(events(&w.db, &episode_stream(eid)).await.is_empty());
    assert_eq!(
        w.db.scalar_i64(
            "SELECT count(*) FROM ledger.events WHERE event_type = 'allowance.granted'
                           AND payload->>'kind' = 'free_tier'"
        )
        .await,
        0,
        "the top-up rolled back with the refused hold"
    );
    // The conforming twin: exactly the available amount.
    w.open(140).await;
    assert_eq!(allowance_row(&w.db, OWNER).await.available, 0);
    w.db.finish().await;
}

// @req ECO-5, CAP-12.01, T9
#[tokio::test]
async fn a_hold_above_the_price_policy_cap_is_refused() {
    let w = world().await;
    let big = accept_grant(&w.ledger, TENANT_A, 10_000, 0, 1000).await;
    open_allowance(&w.ledger, TENANT_A, "rich", &big).await;
    let mut req = w.req(Uuid::new_v4(), 501);
    req.owner_id = "rich".into();
    req.grant_policy = big.clone();
    match w
        .ledger
        .dispatch(
            abw_ledger::episode::Open::new(req.clone()),
            &platform(TENANT_A),
        )
        .await
    {
        Err(DispatchError::Rejected(Rejection::HoldAboveCap {
            cap: 500,
            requested: 501,
        })) => {}
        other => panic!("a hold above the cap was placed: {other:?}"),
    }
    req.hold_cu = 500;
    open_episode(&w.ledger, TENANT_A, req).await;
    w.db.finish().await;
}

// @req ECO-4, CAP-12.03, T9
#[tokio::test]
async fn an_episode_cannot_hold_on_an_allowance_it_does_not_own() {
    let w = world().await;
    let grant_b = accept_grant(&w.ledger, TENANT_B, 100, 0, 1000).await;
    open_allowance(&w.ledger, TENANT_B, "owner-b", &grant_b).await;
    // Tenant A's run naming tenant B's owner as payer.
    let mut cross = w.req(Uuid::new_v4(), 10);
    cross.owner_id = "owner-b".into();
    match w
        .ledger
        .dispatch(
            abw_ledger::episode::Open::new(cross.clone()),
            &platform(TENANT_A),
        )
        .await
    {
        Err(DispatchError::Rejected(Rejection::TenantMismatch { .. })) => {}
        other => panic!("tenant A held on tenant B's allowance: {other:?}"),
    }
    // An owner with no allowance can't borrow anyone's.
    let mut nobody = w.req(Uuid::new_v4(), 10);
    nobody.owner_id = "nobody".into();
    match w
        .ledger
        .dispatch(abw_ledger::episode::Open::new(nobody), &platform(TENANT_A))
        .await
    {
        Err(DispatchError::Rejected(Rejection::AllowanceNotOpen { .. })) => {}
        other => panic!("a hold without an allowance: {other:?}"),
    }
    // Tenant A can't run tenant B's agent either.
    put_agent(&w.ledger, TENANT_B, "agent-b", &[]).await;
    let mut agent_b = w.req(Uuid::new_v4(), 10);
    agent_b.agent_id = "agent-b".into();
    match w
        .ledger
        .dispatch(abw_ledger::episode::Open::new(agent_b), &platform(TENANT_A))
        .await
    {
        Err(DispatchError::Rejected(Rejection::TenantMismatch { .. })) => {}
        other => panic!("tenant A ran tenant B's agent: {other:?}"),
    }
    assert_eq!(allowance_row(&w.db, "owner-b").await.held, 0);
    assert_eq!(
        w.db.scalar_i64("SELECT count(*) FROM ledger.streams WHERE kind = 'episode'")
            .await,
        0
    );
    w.db.finish().await;
}

// @req ECO-5, CAP-12.02, T9
#[tokio::test]
async fn closing_settles_and_releases_in_one_transaction() {
    let w = world().await;
    let (eid, opened) = w.open(60).await;
    let closed = ok(w.close(eid, 1, success(25)).await, "close").output;
    assert_eq!(closed.status, EpisodeStatus::Success);
    assert_eq!((closed.consumed_cu, closed.released_cu), (25, 35));
    assert_eq!(closed.seq, 2);

    let ep = events(&w.db, &episode_stream(eid)).await;
    let al = events(&w.db, &allowance_stream(OWNER)).await;
    let closed_ev = match ep.last() {
        Some(e) if e.event_type == "episode.closed" => e,
        other => panic!("last episode event: {other:?}"),
    };
    let consumed = match al.iter().find(|e| e.event_type == "allowance.consumed") {
        Some(c) => c,
        None => panic!("no consumed event"),
    };
    assert_eq!(
        consumed.txid, closed_ev.txid,
        "close and settle commit in one TX-C"
    );
    assert_eq!(int(consumed, "held_cu"), 60);
    assert_eq!(int(consumed, "consumed_cu"), 25);
    assert_eq!(int(consumed, "released_cu"), 35);
    assert_eq!(int(consumed, "priced_cu"), 25);
    assert_eq!(text(consumed, "hold_id"), opened.hold_id.to_string());
    assert_eq!(text(consumed, "price_version"), w.price.to_string());
    assert_eq!(
        text(consumed, "idempotency_key"),
        format!("episode:{eid}:settle")
    );
    assert!(
        !al.iter().any(|e| e.event_type == "allowance.hold_released"),
        "settlement releases the remainder in the consumed event itself"
    );
    assert_eq!(text(closed_ev, "status"), "success");
    assert_eq!(int(closed_ev, "consumed_cu"), 25);

    let row = allowance_row(&w.db, OWNER).await;
    assert_eq!((row.held, row.available, row.total_consumed), (0, 115, 25));
    let e = episode_row(&w.db, eid).await;
    assert_eq!((e.status.as_str(), e.consumed_cu), ("success", Some(25)));
    assert_eq!(hold_status(&w.db, eid).await, "consumed");
    w.db.finish().await;
}

// @req ECO-5, CAP-12.02, T9
#[tokio::test]
async fn consumption_never_exceeds_the_hold() {
    let w = world().await;
    let (eid, _) = w.open(50).await;
    let outcome = CloseOutcome {
        status: EpisodeStatus::Partial,
        reason_class: Some(ReasonClass::BudgetExhausted),
        raw_output: None,
        usage: usage(80, 0, 0),
    };
    let closed = ok(w.close(eid, 1, outcome).await, "close").output;
    assert_eq!((closed.consumed_cu, closed.released_cu), (50, 0));
    let consumed = match events_of_type(&w.db, "allowance.consumed").await.pop() {
        Some(c) => c,
        None => panic!("no consumed event"),
    };
    assert_eq!(
        int(&consumed, "priced_cu"),
        80,
        "the overshoot stays visible"
    );
    assert_eq!(int(&consumed, "consumed_cu"), 50);
    let row = allowance_row(&w.db, OWNER).await;
    assert_eq!((row.held, row.available, row.total_consumed), (0, 90, 50));
    let e = episode_row(&w.db, eid).await;
    assert_eq!(e.reason_class.as_deref(), Some("budget_exhausted"));
    assert_ne!(e.status, "success");
    w.db.finish().await;
}

// @req ECO-7, CAP-12.02, T9
#[tokio::test]
async fn consumption_draws_free_tier_then_granted_then_purchased() {
    let w = world().await;
    // free_tier 40 (top-up at the first hold), granted 100.
    let (eid, opened) = w.open(70).await;
    assert_eq!(opened.free_tier_granted_cu, Some(40));
    let closed = ok(w.close(eid, 1, success(55)).await, "close").output;
    assert_eq!(
        (
            closed.split.free_tier,
            closed.split.granted,
            closed.split.purchased
        ),
        (40, 15, 0)
    );
    let consumed = match events_of_type(&w.db, "allowance.consumed").await.pop() {
        Some(c) => c,
        None => panic!("no consumed event"),
    };
    assert_eq!(
        consumed.payload.get("split"),
        Some(&json!({"free_tier": 40, "granted": 15, "purchased": 0}))
    );
    let row = allowance_row(&w.db, OWNER).await;
    assert_eq!((row.free_tier, row.granted, row.purchased), (0, 85, 0));
    assert_eq!(row.available, 85);
    w.db.finish().await;
}

// @req ECO-3, T9
#[tokio::test]
async fn the_free_tier_tops_up_to_the_cap_once_per_month() {
    let w = world().await;
    let open_at = |when| {
        let mut r = w.req(Uuid::new_v4(), 10);
        r.at = when;
        r
    };

    // November: first hold tops 0 up to 40.
    let r1 = open_at(at(2026, 11, 3, 9, 0));
    let e1 = r1.episode_id;
    let o1 = open_episode(&w.ledger, TENANT_A, r1).await;
    assert_eq!(o1.free_tier_granted_cu, Some(40));
    ok(w.close(e1, 1, success(10)).await, "close 1");
    assert_eq!(allowance_row(&w.db, OWNER).await.free_tier, 30);
    // Same month: no second top-up.
    let o2 = open_episode(&w.ledger, TENANT_A, open_at(at(2026, 11, 28, 23, 59))).await;
    assert_eq!(o2.free_tier_granted_cu, None);
    // December: topped up by 10 back to the cap, never above it.
    let o3 = open_episode(&w.ledger, TENANT_A, open_at(at(2026, 12, 1, 0, 0))).await;
    assert_eq!(o3.free_tier_granted_cu, Some(10));
    assert_eq!(allowance_row(&w.db, OWNER).await.free_tier, 40);
    // January, still at the cap: the month is marked with a zero grant, nothing added.
    let o4 = open_episode(&w.ledger, TENANT_A, open_at(at(2027, 1, 15, 8, 0))).await;
    assert_eq!(o4.free_tier_granted_cu, Some(0));
    assert_eq!(allowance_row(&w.db, OWNER).await.free_tier, 40);

    let evs: Vec<Ev> = events_of_type(&w.db, "allowance.granted")
        .await
        .into_iter()
        .filter(|e| text(e, "kind") == "free_tier")
        .collect();
    let periods: Vec<String> = evs.iter().map(|e| text(e, "period")).collect();
    assert_eq!(periods, ["2026-11", "2026-12", "2027-01"]);
    let amounts: Vec<i64> = evs.iter().map(|e| int(e, "amount_cu")).collect();
    assert_eq!(amounts, [40, 10, 0]);
    assert_eq!(
        text(&evs[1], "idempotency_key"),
        format!("free_tier:{OWNER}:2026-12")
    );
    assert_eq!(allowance::period(at(2026, 12, 31, 23, 59)), "2026-12");
    // Each top-up is the first allowance event of its TX-A.
    for e in &evs {
        let tx_events: Vec<Ev> = events(&w.db, &allowance_stream(OWNER))
            .await
            .into_iter()
            .filter(|x| x.txid == e.txid)
            .collect();
        assert_eq!(tx_events[0].event_type, "allowance.granted");
        assert_eq!(tx_events[1].event_type, "allowance.hold_placed");
    }
    w.db.finish().await;
}

// @req ECO-6, CAP-12.03, T9
#[tokio::test]
async fn settling_is_idempotent() {
    let w = world().await;
    let (eid, _) = w.open(60).await;
    let first = ok(w.close(eid, 1, success(20)).await, "close");
    let retry = ok(w.close(eid, 1, success(20)).await, "retry");
    assert!(!first.replayed);
    assert!(retry.replayed, "a retried settle returns the stored result");
    assert_eq!(retry.output.consumed_cu, 20);
    match w.close(eid, 1, success(21)).await {
        Err(DispatchError::IdempotencyMismatch { .. }) => {}
        other => panic!("a second, different settle: {other:?}"),
    }
    match w.close(eid, 2, success(21)).await {
        Err(DispatchError::IdempotencyMismatch { .. }) => {}
        other => panic!("a settle at the new head: {other:?}"),
    }
    assert_eq!(events_of_type(&w.db, "allowance.consumed").await.len(), 1);
    assert_eq!(allowance_row(&w.db, OWNER).await.total_consumed, 20);
    w.db.finish().await;
}

// @req ECO-5, CAP-01.02, T9
#[tokio::test]
async fn abandoning_closes_the_episode_and_releases_the_hold_together() {
    let w = world().await;
    let (eid, opened) = w.open(60).await;
    let ab = ok(
        w.ledger
            .dispatch(
                Abandon::new(eid, Expected::Exact(1), ReasonClass::AbandonedDropped),
                &platform(TENANT_A),
            )
            .await,
        "abandon",
    )
    .output;
    assert_eq!(ab.released_cu, 60);
    let closed = match events(&w.db, &episode_stream(eid)).await.pop() {
        Some(e) => e,
        None => panic!("no events"),
    };
    let released = match events_of_type(&w.db, "allowance.hold_released").await.pop() {
        Some(e) => e,
        None => panic!("no hold_released"),
    };
    assert_eq!(closed.event_type, "episode.closed");
    assert_eq!(released.txid, closed.txid);
    assert_eq!(text(&closed, "status"), "failure");
    assert_eq!(text(&closed, "reason_class"), "abandoned_dropped");
    assert_eq!(int(&released, "amount_cu"), 60);
    assert_eq!(text(&released, "hold_id"), opened.hold_id.to_string());
    assert_eq!(text(&released, "reason_class"), "abandoned_dropped");
    assert_eq!(
        text(&released, "idempotency_key"),
        format!("episode:{eid}:release")
    );
    assert!(events_of_type(&w.db, "allowance.consumed").await.is_empty());
    let row = allowance_row(&w.db, OWNER).await;
    assert_eq!((row.held, row.available, row.total_consumed), (0, 140, 0));
    assert_eq!(hold_status(&w.db, eid).await, "released");
    w.db.finish().await;
}

// @req CAP-01.02, ECO-5, T9
#[tokio::test]
async fn a_closed_episode_accepts_no_further_events() {
    let w = world().await;
    let (eid, _) = w.open(60).await;
    ok(w.close(eid, 1, success(5)).await, "close");
    let late_tool = RecordTool::new(eid, Expected::Exact(2), echo_record(0, 0, json!({"x": 1})));
    match w.ledger.dispatch(late_tool, &platform(TENANT_A)).await {
        Err(DispatchError::Rejected(Rejection::EpisodeClosed { .. })) => {}
        other => panic!("a tool call recorded after close: {other:?}"),
    }
    for expected in [1, 2] {
        let abandon = Abandon::new(
            eid,
            Expected::Exact(expected),
            ReasonClass::AbandonedTimeout,
        );
        match w.ledger.dispatch(abandon, &platform(TENANT_A)).await {
            Err(DispatchError::Rejected(Rejection::EpisodeClosed { .. }))
            | Err(DispatchError::Conflict { .. }) => {}
            other => panic!("a closed episode was abandoned: {other:?}"),
        }
    }
    assert_eq!(events(&w.db, &episode_stream(eid)).await.len(), 2);
    assert!(
        events_of_type(&w.db, "allowance.hold_released")
            .await
            .is_empty()
    );
    w.db.finish().await;
}

// @req CAP-01.02, T9
#[tokio::test]
async fn close_outcomes_follow_the_status_rules() {
    let w = world().await;
    let (eid, _) = w.open(60).await;
    let bad = [
        (EpisodeStatus::Success, Some(ReasonClass::BudgetExhausted)),
        (EpisodeStatus::Success, Some(ReasonClass::ProviderError)),
        (EpisodeStatus::Partial, Some(ReasonClass::ProviderError)),
        (EpisodeStatus::Partial, None),
        (EpisodeStatus::Failure, None),
        (EpisodeStatus::Failure, Some(ReasonClass::AbandonedTimeout)),
        (EpisodeStatus::Failure, Some(ReasonClass::AbandonedDropped)),
        (EpisodeStatus::Failure, Some(ReasonClass::LedgerError)),
    ];
    for (status, reason_class) in bad {
        let outcome = CloseOutcome {
            status,
            reason_class,
            raw_output: None,
            usage: usage(1, 0, 0),
        };
        match w.close(eid, 1, outcome).await {
            Err(DispatchError::Rejected(Rejection::InvalidOutcome(_))) => {}
            other => panic!("{status:?} with {reason_class:?} was accepted: {other:?}"),
        }
    }
    let provider_error = CloseOutcome {
        status: EpisodeStatus::Failure,
        reason_class: Some(ReasonClass::ProviderError),
        raw_output: None,
        usage: usage(4, 0, 0),
    };
    let closed = ok(
        w.close(eid, 1, provider_error).await,
        "provider error close",
    )
    .output;
    assert_eq!(closed.status, EpisodeStatus::Failure);
    assert_eq!(
        closed.consumed_cu, 4,
        "usage before the error is still charged"
    );
    let e = episode_row(&w.db, eid).await;
    assert_eq!(
        (e.status.as_str(), e.reason_class.as_deref()),
        ("failure", Some("provider_error"))
    );
    w.db.finish().await;
}

// @req CAP-01.03
#[tokio::test]
async fn each_tool_call_is_one_transaction_with_its_input_and_output_artifacts() {
    let w = world().await;
    let (eid, _) = w.open(60).await;
    let mut head = 1;
    let mut txids = Vec::new();
    for (i, input) in [json!({"q": "first"}), json!({"q": "second", "n": [1, 2.5]})]
        .into_iter()
        .enumerate()
    {
        let rec = ok(
            w.ledger
                .dispatch(
                    RecordTool::new(
                        eid,
                        Expected::Exact(head),
                        echo_record(0, i as u32, input.clone()),
                    ),
                    &platform(TENANT_A),
                )
                .await,
            "record tool",
        )
        .output;
        head += 1;
        assert_eq!(rec.seq, head);
        let want_in = ok(artifact_json(&input, "application/json"), "jcs input");
        let want_out = ok(
            artifact_json(&json!({"ok": {"echo": input}}), "application/json"),
            "jcs output",
        );
        assert_eq!(&rec.input_sha, want_in.sha());
        assert_eq!(&rec.output_sha, want_out.sha());
        for sha in [&rec.input_sha, &rec.output_sha] {
            match w.read.resolve(sha).await {
                Ok(Some(bytes)) => assert_eq!(&abw_canon::sha256_raw(&bytes), sha),
                other => panic!("artifact {sha} not stored: {other:?}"),
            }
        }
        let ev = match events(&w.db, &episode_stream(eid)).await.pop() {
            Some(e) => e,
            None => panic!("no events"),
        };
        assert_eq!(ev.event_type, "tool.invoked");
        assert_eq!(sha(&ev, "input_sha"), rec.input_sha);
        assert_eq!(sha(&ev, "output_sha"), rec.output_sha);
        assert_eq!(text(&ev, "tool_name"), "echo");
        assert_eq!(ev.payload.get("allowed"), Some(&json!(true)));
        assert!(ev.payload.get("error_class").is_some_and(Value::is_null));
        txids.push(ev.txid);
    }
    let opened_tx = events(&w.db, &episode_stream(eid)).await[0].txid.clone();
    txids.push(opened_tx);
    txids.sort();
    txids.dedup();
    assert_eq!(
        txids.len(),
        3,
        "TX-A and each TX-B are separate transactions"
    );
    // A retried record replays instead of recording the call twice.
    let again = ok(
        w.ledger
            .dispatch(
                RecordTool::new(
                    eid,
                    Expected::Exact(1),
                    echo_record(0, 0, json!({"q": "first"})),
                ),
                &platform(TENANT_A),
            )
            .await,
        "retry",
    );
    assert!(again.replayed);
    assert_eq!(events(&w.db, &episode_stream(eid)).await.len(), 3);
    w.db.finish().await;
}

// @req CAP-01.03
#[tokio::test]
async fn a_refused_tool_call_is_recorded_with_its_gate_decision() {
    let w = world().await;
    let (eid, _) = w.open(60).await;
    let refused = ToolRecord {
        iteration: 0,
        call_index: 0,
        tool_name: "shell".into(),
        tool_version: "1".into(),
        allowed: false,
        input: json!({"cmd": "rm -rf /"}),
        output: ToolOutcome::Err {
            class: "tool_allowlist".into(),
            message: "shell is not on this agent's allowlist".into(),
        },
        duration_ms: 0,
    };
    let rec = ok(
        w.ledger
            .dispatch(
                RecordTool::new(eid, Expected::Exact(1), refused),
                &platform(TENANT_A),
            )
            .await,
        "record refusal",
    )
    .output;
    assert_eq!(rec.seq, 3, "tool.invoked plus gate.decided");
    let evs = events(&w.db, &episode_stream(eid)).await;
    let types: Vec<&str> = evs.iter().map(|e| e.event_type.as_str()).collect();
    assert_eq!(types, ["episode.opened", "tool.invoked", "gate.decided"]);
    assert_eq!(
        evs[1].txid, evs[2].txid,
        "the refusal and its record share TX-B"
    );
    assert_eq!(evs[1].payload.get("allowed"), Some(&json!(false)));
    assert_eq!(text(&evs[1], "error_class"), "tool_allowlist");
    assert_eq!(text(&evs[2], "gate"), "tool_allowlist");
    assert_eq!(text(&evs[2], "decision"), "refused");
    assert_eq!(text(&evs[2], "reason_class"), "policy");
    assert_eq!(text(&evs[2], "subject"), "tool:shell");
    assert_eq!(text(&evs[2], "episode_id"), eid.to_string());
    let epistemic: String = match sqlx::query_scalar(
        "SELECT epistemic FROM ledger.event_types WHERE event_type = 'gate.decided'",
    )
    .fetch_one(&w.db.admin)
    .await
    {
        Ok(s) => s,
        Err(e) => panic!("{e}"),
    };
    assert_eq!(epistemic, "decision");
    w.db.finish().await;
}

// @req CAP-01.03
#[tokio::test]
async fn a_record_cannot_claim_an_allowed_call_to_a_tool_off_the_allowlist() {
    let w = world().await;
    let (eid, _) = w.open(60).await;
    let mut lie = echo_record(0, 0, json!({}));
    lie.tool_name = "shell".into();
    match w
        .ledger
        .dispatch(
            RecordTool::new(eid, Expected::Exact(1), lie),
            &platform(TENANT_A),
        )
        .await
    {
        Err(DispatchError::Rejected(Rejection::ToolNotAllowed { .. })) => {}
        other => panic!("an off-allowlist call was recorded as allowed: {other:?}"),
    }
    let mut inconsistent = echo_record(0, 0, json!({}));
    inconsistent.allowed = false; // refused, yet with a successful output
    match w
        .ledger
        .dispatch(
            RecordTool::new(eid, Expected::Exact(1), inconsistent),
            &platform(TENANT_A),
        )
        .await
    {
        Err(DispatchError::Rejected(Rejection::InvalidToolRecord(_))) => {}
        other => panic!("a refused call with an output was recorded: {other:?}"),
    }
    assert_eq!(events(&w.db, &episode_stream(eid)).await.len(), 1);
    ok(
        w.ledger
            .dispatch(
                RecordTool::new(eid, Expected::Exact(1), echo_record(0, 0, json!({}))),
                &platform(TENANT_A),
            )
            .await,
        "allowed echo",
    );
    w.db.finish().await;
}

// @req T5, ECO-5, CAP-12.02, T9
#[tokio::test]
#[ignore = "pending M2: abw_ledger::episode and reaper land with the M2 product PR"]
async fn a_fault_at_any_tx_c_step_leaves_the_episode_open_and_the_hold_for_the_reaper() {
    let w = world().await;
    let big = accept_grant(&w.ledger, TENANT_A, 5_000, 0, 1000).await;
    open_allowance(&w.ledger, TENANT_A, "faulty", &big).await;
    let start = at(2026, 11, 3, 9, 0);
    for point in FaultPoint::ALL {
        let eid = Uuid::new_v4();
        let mut req = w.req(eid, 100);
        req.owner_id = "faulty".into();
        req.grant_policy = big.clone();
        open_episode(&w.ledger, TENANT_A, req).await;
        ok(
            w.ledger
                .dispatch(
                    RecordTool::new(eid, Expected::Exact(1), echo_record(0, 0, json!({"k": 1}))),
                    &platform(TENANT_A),
                )
                .await,
            "tool",
        );
        let before = allowance_row(&w.db, "faulty").await;
        let faulty = w.ledger.inject_fault(*point);
        match faulty
            .dispatch(
                Close::new(eid, Expected::Exact(2), success(30)),
                &platform(TENANT_A),
            )
            .await
        {
            Err(DispatchError::Injected(p)) if p == *point => {}
            other => panic!("{point:?}: expected the injected fault, got {other:?}"),
        }
        // No close without settle, no settle without close, no partial release.
        let e = episode_row(&w.db, eid).await;
        assert_eq!(e.status, "running", "{point:?}");
        assert_eq!(e.consumed_cu, None, "{point:?}");
        assert_eq!(allowance_row(&w.db, "faulty").await, before, "{point:?}");
        assert_eq!(hold_status(&w.db, eid).await, "open", "{point:?}");
        let types: Vec<String> = events(&w.db, &episode_stream(eid))
            .await
            .into_iter()
            .map(|e| e.event_type)
            .collect();
        assert_eq!(
            types,
            ["episode.opened", "tool.invoked"],
            "{point:?}: evidence kept"
        );
    }
    let n = FaultPoint::ALL.len();
    assert_eq!(allowance_row(&w.db, "faulty").await.held, 100 * n as i64);

    // Before the deadline the reaper leaves running episodes alone.
    let early = ok(
        reaper::run_once(&w.ledger, start + chrono::Duration::minutes(29)).await,
        "reaper",
    );
    assert!(early.abandoned.is_empty());
    let report = ok(
        reaper::run_once(&w.ledger, start + chrono::Duration::minutes(31)).await,
        "reaper",
    );
    assert_eq!(report.abandoned.len(), n);
    let row = allowance_row(&w.db, "faulty").await;
    assert_eq!((row.held, row.available, row.total_consumed), (0, 5_000, 0));
    for eid in &report.abandoned {
        let e = episode_row(&w.db, *eid).await;
        assert_eq!(
            (e.status.as_str(), e.reason_class.as_deref()),
            ("failure", Some("abandoned_timeout"))
        );
        assert_eq!(hold_status(&w.db, *eid).await, "released");
    }
    // A second pass finds nothing.
    let again = ok(
        reaper::run_once(&w.ledger, start + chrono::Duration::minutes(45)).await,
        "reaper",
    );
    assert!(again.abandoned.is_empty() && again.raced.is_empty());
    w.db.finish().await;
}

// @req CAP-01.02, ECO-5, T9
#[tokio::test]
async fn the_reaper_and_a_late_close_are_mutually_safe() {
    let w = world().await;
    let start = at(2026, 11, 3, 9, 0);
    let (eid, _) = w.open(60).await;
    let report = ok(
        reaper::run_once(&w.ledger, start + chrono::Duration::minutes(31)).await,
        "reaper",
    );
    assert_eq!(report.abandoned, vec![eid]);
    assert_eq!(
        reaper::EPISODE_MAX_RUNTIME,
        std::time::Duration::from_secs(30 * 60)
    );
    // The run's own close, arriving late, is refused and charges nothing.
    match w.close(eid, 1, success(10)).await {
        Err(DispatchError::Conflict { .. })
        | Err(DispatchError::Rejected(Rejection::EpisodeClosed { .. })) => {}
        other => panic!("a reaped episode was settled: {other:?}"),
    }
    let row = allowance_row(&w.db, OWNER).await;
    assert_eq!((row.held, row.total_consumed), (0, 0));
    // The other order: a closed episode is never reaped.
    let (eid2, _) = w.open(60).await;
    ok(w.close(eid2, 1, success(10)).await, "close");
    let report = ok(
        reaper::run_once(&w.ledger, start + chrono::Duration::hours(5)).await,
        "reaper",
    );
    assert!(!report.abandoned.contains(&eid2));
    assert_eq!(episode_row(&w.db, eid2).await.status, "success");
    assert_eq!(allowance_row(&w.db, OWNER).await.total_consumed, 10);
    w.db.finish().await;
}

// @req CAP-01.01, CAP-01.02
#[tokio::test]
async fn a_retired_agent_starts_no_new_episodes() {
    let w = world().await;
    ok(
        w.ledger
            .dispatch(agent::Retire::new(AGENT, "superseded"), &human(TENANT_A))
            .await,
        "retire",
    );
    match w
        .ledger
        .dispatch(
            abw_ledger::episode::Open::new(w.req(Uuid::new_v4(), 10)),
            &platform(TENANT_A),
        )
        .await
    {
        Err(DispatchError::Rejected(Rejection::AgentRetired { .. })) => {}
        other => panic!("a retired agent ran: {other:?}"),
    }
    let mut unknown = w.req(Uuid::new_v4(), 10);
    unknown.agent_id = "never-created".into();
    match w
        .ledger
        .dispatch(abw_ledger::episode::Open::new(unknown), &platform(TENANT_A))
        .await
    {
        Err(DispatchError::Rejected(Rejection::AgentNotFound { .. })) => {}
        other => panic!("an unknown agent ran: {other:?}"),
    }
    assert_eq!(allowance_row(&w.db, OWNER).await.held, 0);
    w.db.finish().await;
}

// @req CAP-01.02
#[tokio::test]
async fn the_episode_projection_is_guarded_and_rebuildable() {
    let w = world().await;
    let (eid, _) = w.open(60).await;
    // 42501 is both the T3 trigger's code and a missing grant; the owner reaches the trigger.
    for (url, needle) in [(&w.db.app_url, "42501"), (&w.db.owner_url, "ABW-T3")] {
        let mut conn = w.db.session(url).await;
        common::assert_rejected(
            &mut conn,
            &format!("UPDATE proj.episodes SET status = 'success' WHERE episode_id = '{eid}'"),
            needle,
        )
        .await;
    }
    ok(w.close(eid, 1, success(9)).await, "close");
    match abw_ledger::admin::rebuild_diff(&w.db.owner_url, "proj.episodes").await {
        Ok(diff) => assert!(diff.is_empty(), "{diff:?}"),
        Err(e) => panic!("rebuild: {e:?}"),
    }
    w.db.finish().await;
}
