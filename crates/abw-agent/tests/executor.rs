//! The episode pipeline end to end with a scripted LLM (target-system-design §4.3-§4.7;
//! economy.md §5 "Run an episode"; ADR-0008 §7, §10).
//!
//! No real LLM is called: `ScriptedProvider` returns a fixed sequence of turns, tool calls,
//! usage counts and errors, so every exit path is deterministic. Database state is read back
//! through the superuser pool (`m2` helpers), never through the executor.

#[path = "../../abw-ledger/tests/common/mod.rs"]
mod common;
#[path = "../../abw-ledger/tests/common/m2.rs"]
mod m2;

use abw_agent::BoxFuture;
use abw_agent::budget::{FLUSH_TURNS, MAX_ITERATIONS, estimate_hold};
use abw_agent::executor::{Executor, PipelineFault, RunError, RunOutcome, RunRequest};
use abw_agent::llm::{
    LlmError, LlmProvider, LlmRequest, Message, Scripted, ScriptedProvider, TokenUsage, ToolCall,
    Turn,
};
use abw_agent::tools::{Tool, ToolError, ToolRegistry};
use abw_ledger::episode::{EpisodeStatus, ReasonClass};
use abw_ledger::policy::{PolicyKind, PricePolicy};
use abw_ledger::{
    DispatchError, FaultPoint, Ledger, Rejection, SingletonFence, agent, artifact_json, reaper,
};
use common::TestDb;
use m2::*;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const OWNER: &str = "owner-1";
const AGENT: &str = "agent-1";
const MAX_TOKENS: u32 = 1000;

fn t0() -> chrono::DateTime<chrono::Utc> {
    at(2026, 11, 3, 9, 0)
}

/// A deterministic tool that echoes its input and counts its calls.
struct EchoTool {
    name: &'static str,
    calls: AtomicUsize,
}

impl EchoTool {
    fn new(name: &'static str) -> Arc<EchoTool> {
        Arc::new(EchoTool {
            name,
            calls: AtomicUsize::new(0),
        })
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Tool for EchoTool {
    fn name(&self) -> &str {
        self.name
    }
    fn version(&self) -> &str {
        "1"
    }
    fn description(&self) -> &str {
        "echoes its input"
    }
    fn input_schema(&self) -> Value {
        json!({"type": "object"})
    }
    fn call<'a>(&'a self, input: &'a Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(json!({"echo": input}))
        })
    }
}

/// One database with: a grant policy (onboarding 1000 CU, no free tier), a price policy of
/// 1 CU per input token with holds capped at `hold_cap`, `OWNER`'s allowance, and `AGENT`
/// whose allowlist is `["echo"]`. The registry also holds `secret`, which is not allowed.
struct Rig {
    db: TestDb,
    ledger: Arc<Ledger>,
    _fence: SingletonFence,
    echo: Arc<EchoTool>,
    secret: Arc<EchoTool>,
    hold_cap: u64,
}

async fn rig(hold_cap: u64) -> Rig {
    let db = TestDb::migrated().await;
    let (ledger, _read, fence) = db.ledger().await;
    let grant = accept_grant(&ledger, TENANT_A, 1000, 0, 0).await;
    accept_price(&ledger, TENANT_A, 1, hold_cap).await;
    open_allowance(&ledger, TENANT_A, OWNER, &grant).await;
    put_agent(&ledger, TENANT_A, AGENT, &["echo"]).await;
    Rig {
        db,
        ledger: Arc::new(ledger),
        _fence: fence,
        echo: EchoTool::new("echo"),
        secret: EchoTool::new("secret"),
        hold_cap,
    }
}

impl Rig {
    fn executor(&self, provider: &Arc<ScriptedProvider>) -> Executor {
        let mut tools = ToolRegistry::new();
        tools.register(self.echo.clone());
        tools.register(self.secret.clone());
        let provider: Arc<dyn LlmProvider> = provider.clone();
        Executor::new(self.ledger.clone(), provider, tools)
    }

    async fn run(&self, executor: &Executor, eid: Uuid) -> Result<RunOutcome, RunError> {
        let req = RunRequest {
            episode_id: eid,
            owner_id: OWNER.into(),
            agent_id: AGENT.into(),
            task: "summarise the inputs".into(),
            max_tokens: MAX_TOKENS,
            at: t0(),
        };
        executor.run(req, &platform(TENANT_A)).await
    }

    async fn run_script(&self, script: Vec<Scripted>) -> (Uuid, Arc<ScriptedProvider>, RunOutcome) {
        let provider = Arc::new(ScriptedProvider::new(script));
        let eid = Uuid::new_v4();
        let out = ok(self.run(&self.executor(&provider), eid).await, "run");
        (eid, provider, out)
    }

    async fn tool_events(&self, eid: Uuid) -> Vec<Ev> {
        events(&self.db, &episode_stream(eid))
            .await
            .into_iter()
            .filter(|e| e.event_type == "tool.invoked")
            .collect()
    }
}

fn used(input_tokens: u64) -> TokenUsage {
    TokenUsage {
        input_tokens,
        output_tokens: 0,
    }
}

fn text_turn(text: &str, input_tokens: u64) -> Scripted {
    Scripted::Turn(Turn {
        text: text.into(),
        tool_calls: vec![],
        usage: used(input_tokens),
    })
}

fn call(id: &str, name: &str, input: Value) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: name.into(),
        input,
    }
}

fn tool_turn(name: &str, input: Value, input_tokens: u64) -> Scripted {
    Scripted::Turn(Turn {
        text: String::new(),
        tool_calls: vec![call("c1", name, input)],
        usage: used(input_tokens),
    })
}

fn offered(req: &LlmRequest) -> Vec<String> {
    let mut names: Vec<String> = req.tools.iter().map(|t| t.name.clone()).collect();
    names.sort();
    names
}

/// A provider that, on every call, counts the episode's committed holds before answering.
struct HoldProbe {
    pool: sqlx::PgPool,
    episode_id: Mutex<Option<Uuid>>,
    holds_seen: Mutex<Vec<i64>>,
    inner: ScriptedProvider,
}

impl LlmProvider for HoldProbe {
    fn complete<'a>(&'a self, req: &'a LlmRequest) -> BoxFuture<'a, Result<Turn, LlmError>> {
        Box::pin(async move {
            let eid = match self.episode_id.lock() {
                Ok(g) => *g,
                Err(e) => panic!("{e}"),
            };
            let n = match sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM ledger.events
                  WHERE event_type = 'allowance.hold_placed' AND payload->>'episode_id' = $1",
            )
            .bind(eid.map(|u| u.to_string()).unwrap_or_default())
            .fetch_one(&self.pool)
            .await
            {
                Ok(n) => n,
                Err(e) => panic!("{e}"),
            };
            match self.holds_seen.lock() {
                Ok(mut g) => g.push(n),
                Err(e) => panic!("{e}"),
            }
            self.inner.complete(req).await
        })
    }
}

// @req ECO-5, CAP-12.01, T9
#[tokio::test]
async fn the_hold_is_committed_before_any_llm_call() {
    let r = rig(100).await;
    let eid = Uuid::new_v4();
    let probe = Arc::new(HoldProbe {
        pool: r.db.admin.clone(),
        episode_id: Mutex::new(Some(eid)),
        holds_seen: Mutex::new(Vec::new()),
        inner: ScriptedProvider::new(vec![
            tool_turn("echo", json!({"q": 1}), 5),
            text_turn("done", 5),
        ]),
    });
    let mut tools = ToolRegistry::new();
    tools.register(r.echo.clone());
    let provider: Arc<dyn LlmProvider> = probe.clone();
    let exec = Executor::new(r.ledger.clone(), provider, tools);
    let out = ok(r.run(&exec, eid).await, "run");
    assert_eq!(out.status, EpisodeStatus::Success);
    let seen = match probe.holds_seen.lock() {
        Ok(g) => g.clone(),
        Err(e) => panic!("{e}"),
    };
    assert_eq!(seen, vec![1, 1], "every LLM call saw the committed hold");
    let hold = estimate_hold(
        &PricePolicy {
            cu_per_input_token: 1,
            cu_per_output_token: 0,
            cu_per_tool_call: 0,
            hold_gas_cu: 0,
            hold_cap_cu: r.hold_cap,
        },
        MAX_TOKENS,
    );
    assert_eq!(hold, 100, "1 CU x 1000 tokens x 6 turns, capped at 100");
    assert_eq!(out.held_cu, hold);
    r.db.finish().await;
}

// @req ECO-5, CAP-12.01, T9
#[tokio::test]
async fn a_run_the_allowance_cannot_cover_never_reaches_the_provider() {
    // Holds are capped at 2000 and the estimate reaches the cap, but only 1000 is available.
    let r = rig(2000).await;
    let provider = Arc::new(ScriptedProvider::new(vec![text_turn("never", 1)]));
    let eid = Uuid::new_v4();
    match r.run(&r.executor(&provider), eid).await {
        Err(RunError::Rejected(Rejection::InsufficientAllowance {
            available: 1000,
            requested: 2000,
        })) => {}
        other => panic!("an uncovered run started: {other:?}"),
    }
    assert_eq!(provider.calls(), 0, "no LLM call without a hold");
    assert!(events(&r.db, &episode_stream(eid)).await.is_empty());
    assert_eq!(allowance_row(&r.db, OWNER).await.held, 0);
    r.db.finish().await;
}

// @req ECO-3, ECO-5, T9
#[tokio::test]
async fn a_run_without_an_active_price_policy_never_reaches_the_provider() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let grant = accept_grant(&ledger, TENANT_A, 1000, 0, 0).await;
    open_allowance(&ledger, TENANT_A, OWNER, &grant).await;
    put_agent(&ledger, TENANT_A, AGENT, &[]).await;
    let ledger = Arc::new(ledger);
    let provider = Arc::new(ScriptedProvider::new(vec![text_turn("never", 1)]));
    let dynp: Arc<dyn LlmProvider> = provider.clone();
    let exec = Executor::new(ledger.clone(), dynp, ToolRegistry::new());
    let req = || RunRequest {
        episode_id: Uuid::new_v4(),
        owner_id: OWNER.into(),
        agent_id: AGENT.into(),
        task: "t".into(),
        max_tokens: MAX_TOKENS,
        at: t0(),
    };
    match exec.run(req(), &platform(TENANT_A)).await {
        Err(RunError::Rejected(Rejection::PolicyNotFound { .. })) => {}
        other => panic!("a run without a price policy: {other:?}"),
    }
    assert_eq!(provider.calls(), 0);
    // The conforming twin: once a price policy is accepted, the same run goes ahead.
    accept(
        &ledger,
        TENANT_A,
        PolicyKind::Price,
        price_doc(1, 0, 0, 0, 10),
    )
    .await;
    let out = ok(
        exec.run(req(), &platform(TENANT_A)).await,
        "run with a policy",
    );
    assert_eq!(out.status, EpisodeStatus::Success);
    assert_eq!(provider.calls(), 1);
    db.finish().await;
}

// @req ECO-5, CAP-12.02, T9
#[tokio::test]
async fn the_executor_stops_at_the_hold() {
    let r = rig(100).await;
    let script = vec![
        tool_turn("echo", json!({"n": 1}), 40),
        tool_turn("echo", json!({"n": 2}), 40),
        tool_turn("echo", json!({"n": 3}), 40),
        tool_turn("echo", json!({"n": 4}), 40),
        text_turn("never reached", 1),
    ];
    let (eid, provider, out) = r.run_script(script).await;
    // 40, 80, then 120 >= 100: stop at once, before the third turn's tool call.
    assert_eq!(provider.calls(), 3);
    assert_eq!(provider.remaining(), 2);
    assert_eq!(r.echo.calls(), 2);
    assert_eq!(r.tool_events(eid).await.len(), 2);
    assert_eq!(out.status, EpisodeStatus::Partial);
    assert_eq!(out.reason_class, Some(ReasonClass::BudgetExhausted));
    assert_eq!((out.held_cu, out.consumed_cu), (100, 100));
    assert!(out.consumed_cu <= out.held_cu);
    let e = episode_row(&r.db, eid).await;
    assert_eq!(
        (e.status.as_str(), e.reason_class.as_deref(), e.consumed_cu),
        ("partial", Some("budget_exhausted"), Some(100))
    );
    let row = allowance_row(&r.db, OWNER).await;
    assert_eq!((row.held, row.total_consumed, row.available), (0, 100, 900));
    r.db.finish().await;
}

// @req CAP-01.02
#[tokio::test]
async fn the_iteration_budget_is_five_turns_plus_one_flush() {
    assert_eq!(
        (MAX_ITERATIONS, FLUSH_TURNS),
        (5, 1),
        "target-system-design §4.1"
    );
    let r = rig(500).await;
    let mut script: Vec<Scripted> = (0..5)
        .map(|i| tool_turn("echo", json!({"i": i}), 1))
        .collect();
    script.push(text_turn("summary", 1));
    script.push(text_turn("a seventh call must never happen", 1));
    let (eid, provider, out) = r.run_script(script).await;
    assert_eq!(provider.calls(), 6);
    assert_eq!(provider.remaining(), 1);
    let reqs = provider.requests();
    for req in &reqs[..5] {
        assert_eq!(offered(req), ["echo"]);
    }
    assert!(reqs[5].tools.is_empty(), "the flush turn offers no tools");
    assert_eq!(r.tool_events(eid).await.len(), 5);
    assert_eq!(out.status, EpisodeStatus::Success);
    assert_eq!(out.output.as_deref(), Some("summary"));
    assert_eq!((out.llm_calls, out.tool_calls), (6, 5));
    r.db.finish().await;
}

// @req CAP-01.02
#[tokio::test]
async fn tool_calls_in_the_flush_turn_are_not_executed() {
    let r = rig(500).await;
    let script: Vec<Scripted> = (0..7)
        .map(|i| tool_turn("echo", json!({"i": i}), 1))
        .collect();
    let (eid, provider, out) = r.run_script(script).await;
    assert_eq!(provider.calls(), 6);
    assert_eq!(r.echo.calls(), 5);
    assert_eq!(r.tool_events(eid).await.len(), 5);
    assert_eq!(out.status, EpisodeStatus::Partial);
    assert_eq!(out.reason_class, Some(ReasonClass::IterationsExhausted));
    let e = episode_row(&r.db, eid).await;
    assert_eq!(e.reason_class.as_deref(), Some("iterations_exhausted"));
    r.db.finish().await;
}

// @req CAP-01.02, ECO-5, T9
#[tokio::test]
async fn provider_errors_never_close_as_success() {
    let r = rig(500).await;
    let cases: Vec<(Vec<Scripted>, u64)> = vec![
        (vec![Scripted::Error(LlmError::RateLimited)], 0),
        (
            vec![
                tool_turn("echo", json!({"a": 1}), 10),
                tool_turn("echo", json!({"a": 2}), 10),
                Scripted::Error(LlmError::Provider {
                    status: 529,
                    message: "overloaded".into(),
                }),
            ],
            20,
        ),
        (
            vec![Scripted::Error(LlmError::Transport("reset".into()))],
            0,
        ),
        // An exhausted script is a provider failure too.
        (vec![tool_turn("echo", json!({"a": 3}), 7)], 7),
    ];
    for (script, charged) in cases {
        let (eid, _provider, out) = r.run_script(script).await;
        assert_eq!(out.status, EpisodeStatus::Failure);
        assert_eq!(out.reason_class, Some(ReasonClass::ProviderError));
        assert_eq!(
            out.consumed_cu, charged,
            "usage before the error is charged"
        );
        let e = episode_row(&r.db, eid).await;
        assert_eq!(
            (e.status.as_str(), e.reason_class.as_deref()),
            ("failure", Some("provider_error"))
        );
        assert_eq!(hold_status(&r.db, eid).await, "consumed");
    }
    assert_eq!(allowance_row(&r.db, OWNER).await.held, 0);
    r.db.finish().await;
}

// @req CAP-01.02, ECO-5, T9
#[tokio::test]
async fn every_exit_path_reaches_a_terminal_status() {
    let r = rig(100).await;
    let tools3 = || -> Vec<Scripted> {
        let mut s: Vec<Scripted> = (0..3)
            .map(|i| tool_turn("echo", json!({"i": i}), 1))
            .collect();
        s.push(text_turn("done", 1));
        s
    };
    let mut expect: Vec<(Uuid, &str, Option<&str>)> = Vec::new();

    let (e, _, _) = r.run_script(vec![text_turn("done", 1)]).await;
    expect.push((e, "success", None));
    let (e, _, _) = r
        .run_script(vec![Scripted::Error(LlmError::RateLimited)])
        .await;
    expect.push((e, "failure", Some("provider_error")));
    let (e, _, _) = r
        .run_script(vec![
            tool_turn("echo", json!({}), 60),
            tool_turn("echo", json!({}), 60),
        ])
        .await;
    expect.push((e, "partial", Some("budget_exhausted")));
    let (e, _, _) = r
        .run_script(
            (0..6)
                .map(|i| tool_turn("echo", json!({"i": i}), 1))
                .collect(),
        )
        .await;
    expect.push((e, "partial", Some("iterations_exhausted")));

    // A failed TX-B abandons the run.
    let provider = Arc::new(ScriptedProvider::new(tools3()));
    let e = Uuid::new_v4();
    let exec = r.executor(&provider).with_fault(PipelineFault::TxB {
        call: 1,
        point: FaultPoint::AfterEvents,
    });
    match r.run(&exec, e).await {
        Err(RunError::Ledger(DispatchError::Injected(FaultPoint::AfterEvents))) => {}
        other => panic!("a failed TX-B: {other:?}"),
    }
    expect.push((e, "failure", Some("ledger_error")));

    // A crash leaves the run to the reaper.
    let provider = Arc::new(ScriptedProvider::new(tools3()));
    let e = Uuid::new_v4();
    let exec = r
        .executor(&provider)
        .with_fault(PipelineFault::CrashAfterTxA);
    match r.run(&exec, e).await {
        Err(RunError::Crashed(PipelineFault::CrashAfterTxA)) => {}
        other => panic!("a crash: {other:?}"),
    }
    assert_eq!(episode_row(&r.db, e).await.status, "running");
    ok(
        reaper::run_once(&r.ledger, t0() + chrono::Duration::minutes(31)).await,
        "reaper",
    );
    expect.push((e, "failure", Some("abandoned_timeout")));

    for (eid, status, reason) in expect {
        let row = episode_row(&r.db, eid).await;
        assert_eq!(
            (row.status.as_str(), row.reason_class.as_deref()),
            (status, reason),
            "{eid}"
        );
        let hold = hold_status(&r.db, eid).await;
        assert!(
            hold == "consumed" || hold == "released",
            "{eid}: hold {hold}"
        );
    }
    let row = allowance_row(&r.db, OWNER).await;
    assert_eq!(row.held, 0, "no hold is left open");
    r.db.finish().await;
}

// @req CAP-01.03
#[tokio::test]
async fn a_tool_off_the_allowlist_is_not_offered_not_run_and_recorded_as_refused() {
    let r = rig(500).await;
    let (eid, provider, out) = r
        .run_script(vec![
            tool_turn("secret", json!({"path": "/etc/shadow"}), 1),
            text_turn("ok, I can't", 1),
        ])
        .await;
    let reqs = provider.requests();
    assert_eq!(
        offered(&reqs[0]),
        ["echo"],
        "the model is never offered `secret`"
    );
    assert_eq!(
        r.secret.calls(),
        0,
        "a named but unoffered tool is not executed"
    );
    let refused_reply = reqs[1].messages.iter().any(
        |m| matches!(m, Message::ToolResult { call_id, is_error: true, .. } if call_id == "c1"),
    );
    assert!(refused_reply, "the model gets a tool error back");
    let evs = events(&r.db, &episode_stream(eid)).await;
    let tool = match evs.iter().find(|e| e.event_type == "tool.invoked") {
        Some(t) => t,
        None => panic!("the refusal was not recorded"),
    };
    assert_eq!(text(tool, "tool_name"), "secret");
    assert_eq!(tool.payload.get("allowed"), Some(&json!(false)));
    let gate = match evs.iter().find(|e| e.event_type == "gate.decided") {
        Some(g) => g,
        None => panic!("no gate decision"),
    };
    assert_eq!(gate.txid, tool.txid);
    assert_eq!(text(gate, "gate"), "tool_allowlist");
    assert_eq!(text(gate, "decision"), "refused");
    assert_eq!(out.status, EpisodeStatus::Success);
    r.db.finish().await;
}

// @req CAP-01.03
#[tokio::test]
async fn every_tool_call_is_an_artifact_and_an_event_in_its_own_transaction() {
    let r = rig(500).await;
    let inputs = [json!({"q": "alpha"}), json!({"q": "beta", "k": [1, 2]})];
    let both = Scripted::Turn(Turn {
        text: String::new(),
        tool_calls: vec![
            call("a", "echo", inputs[0].clone()),
            call("b", "echo", inputs[1].clone()),
        ],
        usage: used(3),
    });
    let (eid, _provider, out) = r.run_script(vec![both, text_turn("done", 1)]).await;
    assert_eq!(out.tool_calls, 2);
    let evs = events(&r.db, &episode_stream(eid)).await;
    let tools: Vec<&Ev> = evs
        .iter()
        .filter(|e| e.event_type == "tool.invoked")
        .collect();
    assert_eq!(tools.len(), 2);
    let read = r.ledger.reader();
    for (i, (ev, input)) in tools.iter().zip(inputs.iter()).enumerate() {
        assert_eq!(int(ev, "iteration"), 0);
        assert_eq!(int(ev, "call_index"), i as i64);
        let want_in = ok(artifact_json(input, "application/json"), "jcs");
        let want_out = ok(
            artifact_json(&json!({"ok": {"echo": input}}), "application/json"),
            "jcs",
        );
        assert_eq!(&sha(ev, "input_sha"), want_in.sha());
        assert_eq!(&sha(ev, "output_sha"), want_out.sha());
        for key in ["input_sha", "output_sha"] {
            let s = sha(ev, key);
            match read.resolve(&s).await {
                Ok(Some(bytes)) => assert_eq!(abw_canon::sha256_raw(&bytes), s),
                other => panic!("{key} {s} is not provable: {other:?}"),
            }
        }
    }
    let mut txids: Vec<&str> = evs.iter().map(|e| e.txid.as_str()).collect();
    txids.dedup();
    assert_eq!(txids.len(), 4, "TX-A, two TX-Bs and TX-C");
    let closed = match evs.last() {
        Some(e) if e.event_type == "episode.closed" => e,
        other => panic!("last event: {other:?}"),
    };
    let listed: Vec<Value> = tools.iter().map(|t| json!(text(t, "output_sha"))).collect();
    assert_eq!(
        closed.payload.get("tool_call_shas"),
        Some(&Value::Array(listed))
    );
    r.db.finish().await;
}

// @req CAP-01.03, ECO-5, T9
#[tokio::test]
async fn a_crash_after_a_tool_call_keeps_its_evidence() {
    let r = rig(500).await;
    let mut script: Vec<Scripted> = (0..3)
        .map(|i| tool_turn("echo", json!({"i": i}), 1))
        .collect();
    script.push(text_turn("done", 1));
    let provider = Arc::new(ScriptedProvider::new(script));
    let eid = Uuid::new_v4();
    let exec = r
        .executor(&provider)
        .with_fault(PipelineFault::CrashAfterTxB { call: 1 });
    match r.run(&exec, eid).await {
        Err(RunError::Crashed(PipelineFault::CrashAfterTxB { call: 1 })) => {}
        other => panic!("expected the simulated crash: {other:?}"),
    }
    assert_eq!(
        provider.calls(),
        2,
        "the process died before the third turn"
    );
    let tools = r.tool_events(eid).await;
    assert_eq!(
        tools.len(),
        2,
        "both committed tool calls survive the crash"
    );
    let read = r.ledger.reader();
    for ev in &tools {
        for key in ["input_sha", "output_sha"] {
            match read.resolve(&sha(ev, key)).await {
                Ok(Some(_)) => {}
                other => panic!("{key} lost in the crash: {other:?}"),
            }
        }
    }
    assert_eq!(episode_row(&r.db, eid).await.status, "running");
    assert_eq!(hold_status(&r.db, eid).await, "open");
    let report = ok(
        reaper::run_once(&r.ledger, t0() + chrono::Duration::minutes(31)).await,
        "reaper",
    );
    assert_eq!(report.abandoned, vec![eid]);
    assert_eq!(
        r.tool_events(eid).await.len(),
        2,
        "recovery keeps the evidence"
    );
    assert_eq!(hold_status(&r.db, eid).await, "released");
    r.db.finish().await;
}

// @req T5, ECO-5, CAP-12.01, T9
#[tokio::test]
#[ignore = "pending M2: abw-agent lands with the M2 product PR"]
async fn a_fault_in_tx_a_leaves_nothing_and_calls_no_provider() {
    let r = rig(100).await;
    let before = allowance_row(&r.db, OWNER).await;
    for point in FaultPoint::ALL {
        let provider = Arc::new(ScriptedProvider::new(vec![text_turn("never", 1)]));
        let eid = Uuid::new_v4();
        let exec = r.executor(&provider).with_fault(PipelineFault::TxA(*point));
        match r.run(&exec, eid).await {
            Err(RunError::Ledger(DispatchError::Injected(p))) if p == *point => {}
            other => panic!("{point:?}: {other:?}"),
        }
        assert_eq!(provider.calls(), 0, "{point:?}");
        assert!(
            events(&r.db, &episode_stream(eid)).await.is_empty(),
            "{point:?}"
        );
    }
    assert_eq!(allowance_row(&r.db, OWNER).await, before);
    r.db.finish().await;
}

// @req T5, CAP-01.03, ECO-5, T9
#[tokio::test]
#[ignore = "pending M2: abw-agent lands with the M2 product PR"]
async fn a_fault_in_a_tx_b_abandons_the_run_and_keeps_earlier_evidence() {
    let r = rig(100).await;
    for point in FaultPoint::ALL {
        let mut script: Vec<Scripted> = (0..3)
            .map(|i| tool_turn("echo", json!({"i": i}), 1))
            .collect();
        script.push(text_turn("done", 1));
        let provider = Arc::new(ScriptedProvider::new(script));
        let eid = Uuid::new_v4();
        let exec = r.executor(&provider).with_fault(PipelineFault::TxB {
            call: 1,
            point: *point,
        });
        match r.run(&exec, eid).await {
            Err(RunError::Ledger(DispatchError::Injected(p))) if p == *point => {}
            other => panic!("{point:?}: {other:?}"),
        }
        assert_eq!(
            provider.calls(),
            2,
            "{point:?}: no LLM call after a lost record"
        );
        let tools = r.tool_events(eid).await;
        assert_eq!(tools.len(), 1, "{point:?}: call 0 kept, call 1 rolled back");
        assert_eq!(int(&tools[0], "call_index"), 0);
        let e = episode_row(&r.db, eid).await;
        assert_eq!(
            (e.status.as_str(), e.reason_class.as_deref()),
            ("failure", Some("ledger_error")),
            "{point:?}"
        );
        assert_eq!(hold_status(&r.db, eid).await, "released", "{point:?}");
    }
    assert_eq!(allowance_row(&r.db, OWNER).await.held, 0);
    r.db.finish().await;
}

// @req T5, ECO-5, CAP-12.02, T9
#[tokio::test]
#[ignore = "pending M2: abw-agent lands with the M2 product PR"]
async fn a_fault_in_tx_c_leaves_the_hold_for_the_reaper() {
    let r = rig(100).await;
    let mut faults: Vec<PipelineFault> = FaultPoint::ALL
        .iter()
        .map(|p| PipelineFault::TxC(*p))
        .collect();
    faults.push(PipelineFault::CrashBeforeTxC);
    let mut eids = Vec::new();
    for fault in faults {
        let provider = Arc::new(ScriptedProvider::new(vec![
            tool_turn("echo", json!({"x": 1}), 2),
            text_turn("done", 2),
        ]));
        let eid = Uuid::new_v4();
        let exec = r.executor(&provider).with_fault(fault);
        match (fault, r.run(&exec, eid).await) {
            (PipelineFault::TxC(p), Err(RunError::Ledger(DispatchError::Injected(q))))
                if p == q => {}
            (PipelineFault::CrashBeforeTxC, Err(RunError::Crashed(_))) => {}
            (f, other) => panic!("{f:?}: {other:?}"),
        }
        let e = episode_row(&r.db, eid).await;
        assert_eq!(
            (e.status.as_str(), e.consumed_cu),
            ("running", None),
            "{fault:?}"
        );
        assert_eq!(hold_status(&r.db, eid).await, "open", "{fault:?}");
        assert!(
            events_of_type(&r.db, "allowance.consumed").await.is_empty(),
            "{fault:?}: no settle without close"
        );
        assert_eq!(
            r.tool_events(eid).await.len(),
            1,
            "{fault:?}: evidence kept"
        );
        eids.push(eid);
    }
    assert_eq!(
        allowance_row(&r.db, OWNER).await.held,
        100 * eids.len() as i64
    );
    let report = ok(
        reaper::run_once(&r.ledger, t0() + chrono::Duration::minutes(31)).await,
        "reaper",
    );
    let mut abandoned = report.abandoned.clone();
    abandoned.sort();
    eids.sort();
    assert_eq!(abandoned, eids);
    let row = allowance_row(&r.db, OWNER).await;
    assert_eq!((row.held, row.total_consumed, row.available), (0, 0, 1000));
    r.db.finish().await;
}

// @req CAP-01.01, CAP-01.02
#[tokio::test]
async fn a_retired_agent_cannot_run() {
    let r = rig(100).await;
    ok(
        r.ledger
            .dispatch(agent::Retire::new(AGENT, "replaced"), &human(TENANT_A))
            .await,
        "retire",
    );
    let provider = Arc::new(ScriptedProvider::new(vec![text_turn("never", 1)]));
    match r.run(&r.executor(&provider), Uuid::new_v4()).await {
        Err(RunError::Rejected(Rejection::AgentRetired { .. })) => {}
        other => panic!("a retired agent ran: {other:?}"),
    }
    assert_eq!(provider.calls(), 0);
    r.db.finish().await;
}
