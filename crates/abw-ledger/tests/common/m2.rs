//! Helpers for the M2 suites (ADR-0008): policies, allowances, agents and episodes, plus
//! read-back of the projections the tests inspect. Shared by `abw-ledger`, `abw-agent` and
//! `abw-http` tests, which include it next to `common` with
//! `#[path = ".../abw-ledger/tests/common/m2.rs"] mod m2;`.
//!
//! It uses only the API pinned in ADR-0008 and reads projections through the superuser pool,
//! never through product read paths, so a bug in a read path can't hide a bug in a write.

#![allow(dead_code)] // each test binary uses a subset

use crate::common::TestDb;
use abw_ledger::episode::{OpenEpisode, Usage};
use abw_ledger::policy::{Accept, PolicyKind, PolicyRef};
use abw_ledger::{Actor, CommandCtx, Ledger, Sha256Ref, agent, allowance, episode};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

pub const TENANT_A: &str = "tenant-a";
pub const TENANT_B: &str = "tenant-b";

/// Unwraps a result or fails the test with what was being done.
pub fn ok<T, E: std::fmt::Debug>(r: Result<T, E>, what: &str) -> T {
    match r {
        Ok(v) => v,
        Err(e) => panic!("{what}: {e:?}"),
    }
}

/// A human operator of `tenant` (policy acceptance and operator grants need one).
pub fn human(tenant: &str) -> CommandCtx {
    CommandCtx::new(tenant, Actor::human("ivan"))
}

/// The platform actor running episodes for `tenant`.
pub fn platform(tenant: &str) -> CommandCtx {
    CommandCtx::new(tenant, Actor::platform("executor"))
}

/// A UTC instant.
pub fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
    match Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).single() {
        Some(t) => t,
        None => panic!("invalid test date {y}-{mo}-{d} {h}:{mi}"),
    }
}

/// When every test policy becomes active: before any test instant.
pub fn policy_epoch() -> DateTime<Utc> {
    at(2026, 1, 1, 0, 0)
}

pub fn grant_doc(onboarding_cu: u64, free_tier_cap_cu: u64, operator_limit_cu: u64) -> Value {
    json!({
        "policy_kind": "grant",
        "onboarding_cu": onboarding_cu,
        "free_tier_cap_cu": free_tier_cap_cu,
        "operator_limit_cu": operator_limit_cu,
    })
}

/// A price document. `hold_cap_cu` bounds every hold.
pub fn price_doc(
    cu_per_input_token: u64,
    cu_per_output_token: u64,
    cu_per_tool_call: u64,
    hold_gas_cu: u64,
    hold_cap_cu: u64,
) -> Value {
    json!({
        "policy_kind": "price",
        "cu_per_input_token": cu_per_input_token,
        "cu_per_output_token": cu_per_output_token,
        "cu_per_tool_call": cu_per_tool_call,
        "hold_gas_cu": hold_gas_cu,
        "hold_cap_cu": hold_cap_cu,
    })
}

/// Accepts version 1 of a new policy as a human of `tenant`.
pub async fn accept(ledger: &Ledger, tenant: &str, kind: PolicyKind, doc: Value) -> PolicyRef {
    let cmd = Accept::new(
        Uuid::new_v4(),
        kind,
        doc,
        "placeholder values for the M2 tests",
        policy_epoch(),
    );
    ok(ledger.dispatch(cmd, &human(tenant)).await, "policy accept").output
}

pub async fn accept_grant(
    ledger: &Ledger,
    tenant: &str,
    onboarding_cu: u64,
    free_tier_cap_cu: u64,
    operator_limit_cu: u64,
) -> PolicyRef {
    accept(
        ledger,
        tenant,
        PolicyKind::Grant,
        grant_doc(onboarding_cu, free_tier_cap_cu, operator_limit_cu),
    )
    .await
}

/// A price policy charging `cu_per_input_token` per input token and nothing else, with
/// holds capped at `hold_cap_cu`.
pub async fn accept_price(
    ledger: &Ledger,
    tenant: &str,
    cu_per_input_token: u64,
    hold_cap_cu: u64,
) -> PolicyRef {
    accept(
        ledger,
        tenant,
        PolicyKind::Price,
        price_doc(cu_per_input_token, 0, 0, 0, hold_cap_cu),
    )
    .await
}

/// Opens `owner`'s allowance under `grant` as a human of `tenant`.
pub async fn open_allowance(
    ledger: &Ledger,
    tenant: &str,
    owner: &str,
    grant: &PolicyRef,
) -> allowance::Opened {
    let cmd = allowance::Open::new(owner, grant.clone());
    ok(ledger.dispatch(cmd, &human(tenant)).await, "allowance open").output
}

/// A small resolved card with the given tool allowlist.
pub fn card(agent_id: &str, tools: &[&str]) -> Value {
    json!({
        "agent_id": agent_id,
        "system_prompt": format!("You are {agent_id}, a test agent."),
        "capabilities": {"model": "scripted-model", "tools_allowed": tools},
        "$excluded": ["performance", "usage", "ontology_stats", "wallet", "declared_prompt_sha256"],
    })
}

pub async fn put_agent(
    ledger: &Ledger,
    tenant: &str,
    agent_id: &str,
    tools: &[&str],
) -> agent::Versioned {
    let cmd = agent::PutVersion::new(agent_id, card(agent_id, tools), agent::VersionReason::Seed);
    ok(ledger.dispatch(cmd, &human(tenant)).await, "agent put").output
}

pub fn usage(input_tokens: u64, output_tokens: u64, tool_calls: u64) -> Usage {
    Usage {
        input_tokens,
        output_tokens,
        tool_calls,
    }
}

pub fn open_request(
    episode_id: Uuid,
    owner: &str,
    agent_id: &str,
    hold_cu: u64,
    price: &PolicyRef,
    grant: &PolicyRef,
    when: DateTime<Utc>,
) -> OpenEpisode {
    OpenEpisode {
        episode_id,
        owner_id: owner.to_string(),
        agent_id: agent_id.to_string(),
        input: json!({"task": "test task"}),
        hold_cu,
        price_policy: price.clone(),
        grant_policy: grant.clone(),
        at: when,
    }
}

/// Opens an episode (TX-A) as the platform actor of `tenant`.
pub async fn open_episode(ledger: &Ledger, tenant: &str, req: OpenEpisode) -> episode::Opened {
    ok(
        ledger
            .dispatch(episode::Open::new(req), &platform(tenant))
            .await,
        "episode open",
    )
    .output
}

/// The `proj.allowances` row of `owner`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allowance {
    pub free_tier: i64,
    pub granted: i64,
    pub purchased: i64,
    pub held: i64,
    pub available: i64,
    pub total_consumed: i64,
}

pub async fn allowance_row(db: &TestDb, owner: &str) -> Allowance {
    let row = match sqlx::query(
        "SELECT free_tier, granted, purchased, held, available, total_consumed
           FROM proj.allowances WHERE owner_id = $1",
    )
    .bind(owner)
    .fetch_one(&db.admin)
    .await
    {
        Ok(r) => r,
        Err(e) => panic!("proj.allowances for {owner}: {e}"),
    };
    let get = |c: &str| match row.try_get::<i64, _>(c) {
        Ok(v) => v,
        Err(e) => panic!("proj.allowances.{c}: {e}"),
    };
    Allowance {
        free_tier: get("free_tier"),
        granted: get("granted"),
        purchased: get("purchased"),
        held: get("held"),
        available: get("available"),
        total_consumed: get("total_consumed"),
    }
}

/// The `proj.episodes` row of an episode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeRow {
    pub status: String,
    pub reason_class: Option<String>,
    pub owner_id: String,
    pub hold_cu: i64,
    pub consumed_cu: Option<i64>,
}

pub async fn episode_row(db: &TestDb, episode_id: Uuid) -> EpisodeRow {
    let row = match sqlx::query(
        "SELECT status, reason_class, owner_id, hold_cu, consumed_cu
           FROM proj.episodes WHERE episode_id = $1",
    )
    .bind(episode_id)
    .fetch_one(&db.admin)
    .await
    {
        Ok(r) => r,
        Err(e) => panic!("proj.episodes for {episode_id}: {e}"),
    };
    let s = |c: &str| match row.try_get::<String, _>(c) {
        Ok(v) => v,
        Err(e) => panic!("proj.episodes.{c}: {e}"),
    };
    EpisodeRow {
        status: s("status"),
        owner_id: s("owner_id"),
        reason_class: match row.try_get::<Option<String>, _>("reason_class") {
            Ok(v) => v,
            Err(e) => panic!("proj.episodes.reason_class: {e}"),
        },
        hold_cu: match row.try_get::<i64, _>("hold_cu") {
            Ok(v) => v,
            Err(e) => panic!("proj.episodes.hold_cu: {e}"),
        },
        consumed_cu: match row.try_get::<Option<i64>, _>("consumed_cu") {
            Ok(v) => v,
            Err(e) => panic!("proj.episodes.consumed_cu: {e}"),
        },
    }
}

/// `proj.allowance_holds.status` of the episode's hold.
pub async fn hold_status(db: &TestDb, episode_id: Uuid) -> String {
    match sqlx::query_scalar::<_, String>(
        "SELECT status FROM proj.allowance_holds WHERE episode_id = $1",
    )
    .bind(episode_id)
    .fetch_one(&db.admin)
    .await
    {
        Ok(s) => s,
        Err(e) => panic!("proj.allowance_holds for {episode_id}: {e}"),
    }
}

/// One event as stored.
#[derive(Debug, Clone)]
pub struct Ev {
    pub global_pos: i64,
    pub seq: i64,
    pub event_type: String,
    pub payload: Value,
    pub txid: String,
    pub actor_kind: String,
}

/// Every event of `stream_id`, in sequence order.
pub async fn events(db: &TestDb, stream_id: &str) -> Vec<Ev> {
    events_where(db, "stream_id = $1", stream_id).await
}

/// Every event of `event_type`, in position order.
pub async fn events_of_type(db: &TestDb, event_type: &str) -> Vec<Ev> {
    events_where(db, "event_type = $1", event_type).await
}

async fn events_where(db: &TestDb, cond: &str, arg: &str) -> Vec<Ev> {
    let sql = format!(
        "SELECT global_pos, seq, event_type, payload, txid::text AS txid, actor_kind
           FROM ledger.events WHERE {cond} ORDER BY global_pos"
    );
    let rows = match sqlx::query(&sql).bind(arg).fetch_all(&db.admin).await {
        Ok(r) => r,
        Err(e) => panic!("{sql}: {e}"),
    };
    rows.iter()
        .map(|r| {
            let s = |c: &str| match r.try_get::<String, _>(c) {
                Ok(v) => v,
                Err(e) => panic!("ledger.events.{c}: {e}"),
            };
            let i = |c: &str| match r.try_get::<i64, _>(c) {
                Ok(v) => v,
                Err(e) => panic!("ledger.events.{c}: {e}"),
            };
            Ev {
                global_pos: i("global_pos"),
                seq: i("seq"),
                event_type: s("event_type"),
                payload: match r.try_get::<Value, _>("payload") {
                    Ok(v) => v,
                    Err(e) => panic!("ledger.events.payload: {e}"),
                },
                txid: s("txid"),
                actor_kind: s("actor_kind"),
            }
        })
        .collect()
}

pub fn episode_stream(episode_id: Uuid) -> String {
    format!("episode:{}", episode_id.hyphenated())
}

pub fn allowance_stream(owner: &str) -> String {
    format!("allowance:{owner}")
}

/// The integer at `payload[key]`, failing the test if absent.
pub fn int(ev: &Ev, key: &str) -> i64 {
    match ev.payload.get(key).and_then(Value::as_i64) {
        Some(v) => v,
        None => panic!("{} has no integer {key}: {}", ev.event_type, ev.payload),
    }
}

/// The string at `payload[key]`, failing the test if absent.
pub fn text(ev: &Ev, key: &str) -> String {
    match ev.payload.get(key).and_then(Value::as_str) {
        Some(v) => v.to_string(),
        None => panic!("{} has no string {key}: {}", ev.event_type, ev.payload),
    }
}

/// The sha at `payload[key]`.
pub fn sha(ev: &Ev, key: &str) -> Sha256Ref {
    let s = text(ev, key);
    match Sha256Ref::parse(&s) {
        Ok(r) => r,
        Err(e) => panic!("{} {key} is not a sha256 ref ({s}): {e}", ev.event_type),
    }
}

/// SQL that appends one probe event as `abw_owner` in the current transaction, so the T3
/// justification triggers are satisfied and only the constraint under test can refuse what
/// follows. `{pos}` in `then` is replaced by that event's position.
pub fn with_justifying_event(then: &str) -> String {
    let stream = format!("probe:justify-{}", Uuid::new_v4().simple());
    let pos = format!("(SELECT max(global_pos) FROM ledger.events WHERE stream_id = '{stream}')");
    format!(
        "INSERT INTO ledger.streams (stream_id, kind, tenant_id, head) VALUES ('{stream}', 'probe', '{TENANT_A}', 1);
         INSERT INTO ledger.events (stream_id, seq, event_type, schema_version, payload, payload_sha,
             command_id, correlation_id, actor, actor_kind)
         VALUES ('{stream}', 1, 'probe.recorded', 1, '{{}}',
             'sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a',
             gen_random_uuid(), gen_random_uuid(), 'test', 'platform');
         {}",
        then.replace("{pos}", &pos)
    )
}

/// Whether the transaction `sql` commits as `abw_owner`: used to show the conforming twin of
/// a constraint test is accepted.
pub async fn commits_as_owner(db: &TestDb, sql: &str) -> bool {
    let mut conn = db.session(&db.owner_url).await;
    crate::common::try_tx(&mut conn, sql).await.is_none()
}
