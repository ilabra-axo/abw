//! Episodes as streams: TX-A (open and hold), TX-B (one tool call), TX-C (close and
//! settle) and abandonment (target-system-design §4.5-§4.7; economy.md §5 "Run an episode";
//! ADR-0008 §4 `episode`).
//!
//! An episode is the stream `episode:<episode_id>`. It accepts nothing once
//! `episode.closed` is appended (§4.6). Its money legs are on the owner's allowance stream
//! and commit in the same transaction as the episode event they belong to: the hold with
//! `episode.opened`, consumption with `episode.closed` (ECO-5), release with the
//! abandonment.

use crate::allowance::{
    self, ConsumedPayload, HoldPlacedPayload, HoldReleasedPayload, State as AllowanceState,
};
use crate::artifact::{Canon, artifact_bytes, artifact_json};
use crate::command::{
    BoxFuture, Expected, Heads, NewEvent, Recorded, StreamAppend, sealed::CommandSpec, stream_id,
};
use crate::ctx::CommandCtx;
use crate::error::{DispatchError, Rejection};
use crate::fold::{self, StoredEvent, Streams};
use crate::policy::{self, Cited, PolicyRef, PricePolicy, decode, to_value};
use crate::registry::{EPISODE_CLOSED, EPISODE_OPENED, GATE_DECIDED, TOOL_INVOKED};
use crate::{agent, registry};
use abw_canon::Sha256Ref;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::PgConnection;
use uuid::Uuid;

pub(crate) const KIND: &str = "episode";

const JSON: &str = "application/json";
const TEXT: &str = "text/plain; charset=utf-8";

pub(crate) fn stream_of(episode_id: Uuid) -> Result<String, Rejection> {
    stream_id(KIND, &episode_id.hyphenated().to_string())
}

/// Metered usage of one run (economy.md §2 "Price policy").
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub tool_calls: u64,
}

/// How a consumption is drawn from the buckets (economy.md §3 "Consumption order").
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Split {
    pub free_tier: u64,
    pub granted: u64,
    pub purchased: u64,
}

/// `episode.closed.status` (§4.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeStatus {
    Success,
    Partial,
    Failure,
}

impl EpisodeStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            EpisodeStatus::Success => "success",
            EpisodeStatus::Partial => "partial",
            EpisodeStatus::Failure => "failure",
        }
    }
}

/// Why an episode did not close `success` (ADR-0008 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonClass {
    BudgetExhausted,
    IterationsExhausted,
    ProviderError,
    LedgerError,
    AbandonedTimeout,
    AbandonedDropped,
}

impl ReasonClass {
    pub fn as_str(self) -> &'static str {
        match self {
            ReasonClass::BudgetExhausted => "budget_exhausted",
            ReasonClass::IterationsExhausted => "iterations_exhausted",
            ReasonClass::ProviderError => "provider_error",
            ReasonClass::LedgerError => "ledger_error",
            ReasonClass::AbandonedTimeout => "abandoned_timeout",
            ReasonClass::AbandonedDropped => "abandoned_dropped",
        }
    }
}

// ---- payloads (ADR-0008 §2) -------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OpenedPayload {
    pub(crate) episode_id: Uuid,
    pub(crate) tenant_id: String,
    pub(crate) owner_id: String,
    pub(crate) agent_id: String,
    pub(crate) version_number: u64,
    #[serde(with = "fold::sha")]
    pub(crate) card_sha: Sha256Ref,
    #[serde(with = "fold::sha")]
    pub(crate) input_sha: Sha256Ref,
    pub(crate) hold_id: Uuid,
    pub(crate) hold_cu: u64,
    pub(crate) price_version: String,
    pub(crate) grant_policy_version: String,
    pub(crate) tools_allowed: Vec<String>,
    pub(crate) workspace_id: Option<Uuid>,
    pub(crate) opened_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ToolInvokedPayload {
    pub(crate) episode_id: Uuid,
    pub(crate) iteration: u32,
    pub(crate) call_index: u32,
    pub(crate) tool_name: String,
    pub(crate) tool_version: String,
    pub(crate) allowed: bool,
    #[serde(with = "fold::sha")]
    pub(crate) input_sha: Sha256Ref,
    #[serde(with = "fold::sha")]
    pub(crate) output_sha: Sha256Ref,
    pub(crate) duration_ms: u64,
    pub(crate) error_class: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GateDecidedPayload {
    pub(crate) gate: String,
    pub(crate) decision: String,
    pub(crate) reason_class: String,
    pub(crate) subject: String,
    pub(crate) episode_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ClosedPayload {
    pub(crate) episode_id: Uuid,
    pub(crate) status: EpisodeStatus,
    pub(crate) reason_class: Option<ReasonClass>,
    #[serde(with = "fold::opt_sha")]
    pub(crate) raw_output_sha: Option<Sha256Ref>,
    #[serde(with = "fold::vec_sha")]
    pub(crate) tool_call_shas: Vec<Sha256Ref>,
    pub(crate) tokens_in: u64,
    pub(crate) tokens_out: u64,
    pub(crate) tool_calls: u64,
    pub(crate) consumed_cu: u64,
}

/// An episode stream, folded.
#[derive(Debug, Clone)]
pub(crate) struct State {
    pub(crate) opened: OpenedPayload,
    pub(crate) closed: bool,
    /// `output_sha` of each recorded tool call, in order.
    pub(crate) tool_output_shas: Vec<Sha256Ref>,
}

impl State {
    pub(crate) fn fold(events: Option<&Vec<StoredEvent>>) -> Result<Option<State>, DispatchError> {
        let mut s: Option<State> = None;
        for e in events.into_iter().flatten() {
            match e.event_type.as_str() {
                "episode.opened" => {
                    s = Some(State {
                        opened: e.decode()?,
                        closed: false,
                        tool_output_shas: Vec::new(),
                    });
                }
                "tool.invoked" => {
                    let t: ToolInvokedPayload = e.decode()?;
                    if let Some(st) = s.as_mut() {
                        st.tool_output_shas.push(t.output_sha);
                    }
                }
                "episode.closed" => {
                    if let Some(st) = s.as_mut() {
                        st.closed = true;
                    }
                }
                _ => {}
            }
        }
        Ok(s)
    }
}

fn rejected(r: Rejection) -> DispatchError {
    DispatchError::Rejected(r)
}

fn event(
    def: &'static registry::EventTypeDef,
    payload: &impl Serialize,
) -> Result<NewEvent, DispatchError> {
    NewEvent::new(def, to_value(payload)?).map_err(rejected)
}

/// The owner named by an episode's `episode.opened`, which never changes; `None` if the
/// episode doesn't exist. Read before the lock to name the allowance stream.
async fn owner_of(
    conn: &mut PgConnection,
    episode_id: Uuid,
) -> Result<Option<String>, DispatchError> {
    let stream = stream_of(episode_id).map_err(rejected)?;
    let owner: Option<Option<String>> = sqlx::query_scalar(
        "SELECT payload->>'owner_id' FROM ledger.events
          WHERE stream_id = $1 AND seq = 1 AND event_type = 'episode.opened'",
    )
    .bind(stream)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(owner.flatten())
}

// ---- TX-A -------------------------------------------------------------------------------

/// What TX-A needs to open an episode.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenEpisode {
    pub episode_id: Uuid,
    pub owner_id: String,
    pub agent_id: String,
    pub input: Value,
    pub hold_cu: u64,
    pub price_policy: PolicyRef,
    pub grant_policy: PolicyRef,
    pub at: DateTime<Utc>,
}

/// TX-A: open an episode and place its hold, after the month's free-tier top-up if due
/// (§4.5 ⟨3⟩-⟨5⟩; economy.md §3, §5 step 1). Idempotent on `episode:<eid>:hold`.
#[derive(Debug, Clone)]
pub struct Open {
    req: OpenEpisode,
    hold_id: Uuid,
    key: String,
}

impl Open {
    pub fn new(req: OpenEpisode) -> Open {
        let eid = req.episode_id.hyphenated();
        Open {
            key: format!("{KIND}:{eid}/episode:{eid}:hold"),
            // A new hold id; not part of the request, so a retry replays the first.
            hold_id: Uuid::new_v4(),
            req: OpenEpisode {
                at: fold::micros(req.at),
                ..req
            },
        }
    }
}

/// The result of [`Open`]. `free_tier_granted_cu` is the top-up appended by this TX-A, if
/// it was the owner's first hold of the month.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opened {
    pub episode_id: Uuid,
    pub hold_id: Uuid,
    pub version_number: u64,
    #[serde(with = "fold::sha")]
    pub card_sha: Sha256Ref,
    pub seq: u64,
    pub free_tier_granted_cu: Option<u64>,
}

#[doc(hidden)]
pub struct OpenFacts {
    streams: Streams,
    grant: Cited,
    price: Cited,
}

impl CommandSpec for Open {
    const TYPE: &'static str = "episode.open";
    type Output = Opened;
    type Route = ();
    type Facts = OpenFacts;

    fn validate(&self, _ctx: &CommandCtx) -> Result<(), Rejection> {
        stream_of(self.req.episode_id)?;
        allowance::stream_of(&self.req.owner_id)?;
        agent::stream_of(&self.req.agent_id)?;
        policy::ref_stream(&self.req.grant_policy)?;
        policy::ref_stream(&self.req.price_policy)?;
        artifact_json(&self.req.input, JSON)
            .map(|_| ())
            .map_err(|e| Rejection::Payload(format!("episode input: {e}")))
    }

    fn idempotency_key(&self) -> Option<&str> {
        Some(&self.key)
    }

    fn request(&self) -> Value {
        json!({
            "episode_id": self.req.episode_id,
            "owner_id": self.req.owner_id,
            "agent_id": self.req.agent_id,
            "input": self.req.input,
            "hold_cu": self.req.hold_cu,
            "price_policy": self.req.price_policy,
            "grant_policy": self.req.grant_policy,
            "at": fold::rfc3339(self.req.at),
        })
    }

    fn route<'a>(
        &'a self,
        _conn: &'a mut PgConnection,
    ) -> BoxFuture<'a, Result<(), DispatchError>> {
        fold::ready(())
    }

    fn write_set(&self, _route: &()) -> Vec<String> {
        [
            stream_of(self.req.episode_id),
            allowance::stream_of(&self.req.owner_id),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn read_set(&self, _route: &()) -> Vec<String> {
        let mut r: Vec<String> = agent::stream_of(&self.req.agent_id).into_iter().collect();
        r.push(self.req.grant_policy.stream_id());
        r.push(self.req.price_policy.stream_id());
        r
    }

    fn fold<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        _route: &'a (),
        ctx: &'a CommandCtx,
    ) -> BoxFuture<'a, Result<OpenFacts, DispatchError>> {
        Box::pin(async move {
            let ids: Vec<String> = [
                allowance::stream_of(&self.req.owner_id),
                agent::stream_of(&self.req.agent_id),
            ]
            .into_iter()
            .flatten()
            .collect();
            let streams = fold::load_streams(conn, &ids).await?;
            let grant = policy::cite(
                conn,
                &self.req.grant_policy,
                policy::PolicyKind::Grant,
                &ctx.tenant,
                self.req.at,
            )
            .await?;
            let price = policy::cite(
                conn,
                &self.req.price_policy,
                policy::PolicyKind::Price,
                &ctx.tenant,
                self.req.at,
            )
            .await?;
            Ok(OpenFacts {
                streams,
                grant,
                price,
            })
        })
    }

    fn decide(
        &self,
        heads: &Heads,
        facts: &OpenFacts,
        ctx: &CommandCtx,
    ) -> Result<Vec<StreamAppend>, DispatchError> {
        let r = &self.req;
        let ep_stream = stream_of(r.episode_id).map_err(rejected)?;
        let al_stream = allowance::stream_of(&r.owner_id).map_err(rejected)?;
        let ag_stream = agent::stream_of(&r.agent_id).map_err(rejected)?;

        let agent = match agent::State::fold(facts.streams.get(&ag_stream))? {
            None => {
                return Err(rejected(Rejection::AgentNotFound {
                    agent_id: r.agent_id.clone(),
                }));
            }
            Some(a) if a.retired => {
                return Err(rejected(Rejection::AgentRetired {
                    agent_id: r.agent_id.clone(),
                }));
            }
            Some(a) => a,
        };
        let Some(state) = AllowanceState::fold(facts.streams.get(&al_stream))? else {
            return Err(rejected(Rejection::AllowanceNotOpen {
                owner_id: r.owner_id.clone(),
            }));
        };
        let grant =
            policy::check_grant(&r.grant_policy, &facts.grant, &ctx.tenant).map_err(rejected)?;
        let price =
            policy::check_price(&r.price_policy, &facts.price, &ctx.tenant).map_err(rejected)?;
        if r.hold_cu > price.hold_cap_cu {
            return Err(rejected(Rejection::HoldAboveCap {
                cap: price.hold_cap_cu,
                requested: r.hold_cu,
            }));
        }

        let mut al_events = Vec::new();
        let mut available = state.balances.available();
        let period = allowance::period(r.at);
        if let Some(top_up) =
            allowance::free_tier_top_up(&r.owner_id, &state, &grant, &r.grant_policy, &period)
        {
            available = available.saturating_add(top_up.amount_cu);
            al_events.push(allowance::granted_event(&top_up)?);
        }
        // §4.5 ⟨3⟩-⟨5⟩, ADR-0008 ambiguity 1: an uncovered hold is refused, never shrunk.
        if r.hold_cu == 0 || available < r.hold_cu {
            return Err(rejected(Rejection::InsufficientAllowance {
                available,
                requested: r.hold_cu,
            }));
        }
        let eid = r.episode_id.hyphenated();
        al_events.push(allowance::hold_placed_event(&HoldPlacedPayload {
            owner_id: r.owner_id.clone(),
            episode_id: r.episode_id,
            hold_id: self.hold_id,
            amount_cu: r.hold_cu,
            workspace_id: None,
            idempotency_key: format!("episode:{eid}:hold"),
        })?);

        let input = artifact_json(&r.input, JSON)
            .map_err(|e| rejected(Rejection::Payload(format!("episode input: {e}"))))?;
        let mut opened = event(
            &EPISODE_OPENED,
            &OpenedPayload {
                episode_id: r.episode_id,
                tenant_id: ctx.tenant.clone(),
                owner_id: r.owner_id.clone(),
                agent_id: r.agent_id.clone(),
                version_number: agent.version_number,
                card_sha: agent.card_sha.clone(),
                input_sha: input.sha().clone(),
                hold_id: self.hold_id,
                hold_cu: r.hold_cu,
                price_version: r.price_policy.to_string(),
                grant_policy_version: r.grant_policy.to_string(),
                tools_allowed: agent.tools_allowed.clone(),
                workspace_id: None,
                opened_at: r.at,
            },
        )?;
        opened.artifacts.push(input);
        Ok(vec![
            StreamAppend {
                expected: allowance::expected_head(heads, &al_stream),
                stream_id: al_stream,
                kind: allowance::KIND,
                events: al_events,
            },
            StreamAppend {
                stream_id: ep_stream,
                kind: KIND,
                expected: Expected::NoStream,
                events: vec![opened],
            },
        ])
    }

    fn output(&self, recorded: &[Recorded], _facts: &OpenFacts) -> Result<Opened, DispatchError> {
        let mut free_tier_granted_cu = None;
        let mut opened = None;
        for r in recorded {
            if r.event_type == registry::ALLOWANCE_GRANTED.event_type {
                let g: allowance::GrantedPayload = decode(&r.payload)?;
                free_tier_granted_cu = Some(g.amount_cu);
            } else if r.event_type == EPISODE_OPENED.event_type {
                let o: OpenedPayload = decode(&r.payload)?;
                opened = Some((o, r.seq));
            }
        }
        let Some((o, seq)) = opened else {
            return Err(DispatchError::Integrity(
                "episode.open recorded no episode.opened".into(),
            ));
        };
        Ok(Opened {
            episode_id: o.episode_id,
            hold_id: o.hold_id,
            version_number: o.version_number,
            card_sha: o.card_sha,
            seq,
            free_tier_granted_cu,
        })
    }
}

// ---- TX-B -------------------------------------------------------------------------------

/// A tool's result: the value it returned, or its error.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolOutcome {
    Ok(Value),
    Err { class: String, message: String },
}

impl ToolOutcome {
    /// The output artifact's document (ADR-0008 §4 TX-B).
    fn document(&self) -> Value {
        match self {
            ToolOutcome::Ok(v) => json!({"ok": v}),
            ToolOutcome::Err { class, message } => {
                json!({"error": {"class": class, "message": message}})
            }
        }
    }
}

/// One tool call as it happened (§4.5 ⟨12⟩).
#[derive(Debug, Clone, PartialEq)]
pub struct ToolRecord {
    pub iteration: u32,
    pub call_index: u32,
    pub tool_name: String,
    pub tool_version: String,
    pub allowed: bool,
    pub input: Value,
    pub output: ToolOutcome,
    pub duration_ms: u64,
}

/// The `tool_allowlist` error class a refused call's output carries (§4.3).
pub const ALLOWLIST_ERROR_CLASS: &str = "tool_allowlist";

/// TX-B: record one tool call with its input and output artifacts; a refused call also
/// records its gate decision in the same transaction (§4.3 "Recording a refusal").
/// Idempotent on `episode:<eid>:tool:<iteration>:<call_index>`.
#[derive(Debug, Clone)]
pub struct RecordTool {
    episode_id: Uuid,
    expected: Expected,
    record: ToolRecord,
    key: String,
}

impl RecordTool {
    pub fn new(episode_id: Uuid, expected: Expected, record: ToolRecord) -> RecordTool {
        let eid = episode_id.hyphenated();
        RecordTool {
            key: format!(
                "{KIND}:{eid}/episode:{eid}:tool:{}:{}",
                record.iteration, record.call_index
            ),
            episode_id,
            expected,
            record,
        }
    }
}

/// The result of [`RecordTool`]: the head after it and the two artifact shas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRecorded {
    pub seq: u64,
    #[serde(with = "fold::sha")]
    pub input_sha: Sha256Ref,
    #[serde(with = "fold::sha")]
    pub output_sha: Sha256Ref,
}

impl CommandSpec for RecordTool {
    const TYPE: &'static str = "episode.record_tool";
    type Output = ToolRecorded;
    type Route = ();
    type Facts = Option<Vec<StoredEvent>>;

    fn validate(&self, _ctx: &CommandCtx) -> Result<(), Rejection> {
        stream_of(self.episode_id)?;
        let r = &self.record;
        if r.tool_name.trim().is_empty() {
            return Err(Rejection::InvalidToolRecord("empty tool name".into()));
        }
        if !r.allowed {
            let refused = matches!(&r.output, ToolOutcome::Err { class, .. } if class == ALLOWLIST_ERROR_CLASS);
            if !refused {
                return Err(Rejection::InvalidToolRecord(format!(
                    "a refused call's output must be a {ALLOWLIST_ERROR_CLASS} error"
                )));
            }
        }
        artifact_json(&r.input, JSON)
            .and_then(|_| artifact_json(&r.output.document(), JSON))
            .map(|_| ())
            .map_err(|e| Rejection::InvalidToolRecord(e.to_string()))
    }

    fn idempotency_key(&self) -> Option<&str> {
        Some(&self.key)
    }

    fn request(&self) -> Value {
        let r = &self.record;
        json!({
            "episode_id": self.episode_id,
            "expected": self.expected.head(),
            "iteration": r.iteration,
            "call_index": r.call_index,
            "tool_name": r.tool_name,
            "tool_version": r.tool_version,
            "allowed": r.allowed,
            "input": r.input,
            "output": r.output.document(),
            "duration_ms": r.duration_ms,
        })
    }

    fn route<'a>(
        &'a self,
        _conn: &'a mut PgConnection,
    ) -> BoxFuture<'a, Result<(), DispatchError>> {
        fold::ready(())
    }

    fn write_set(&self, _route: &()) -> Vec<String> {
        stream_of(self.episode_id).into_iter().collect()
    }

    fn read_set(&self, _route: &()) -> Vec<String> {
        Vec::new()
    }

    fn fold<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        _route: &'a (),
        _ctx: &'a CommandCtx,
    ) -> BoxFuture<'a, Result<Self::Facts, DispatchError>> {
        Box::pin(async move {
            let id = stream_of(self.episode_id).map_err(rejected)?;
            Ok(fold::load_streams(conn, std::slice::from_ref(&id))
                .await?
                .remove(&id))
        })
    }

    fn decide(
        &self,
        _heads: &Heads,
        facts: &Self::Facts,
        _ctx: &CommandCtx,
    ) -> Result<Vec<StreamAppend>, DispatchError> {
        let stream = stream_of(self.episode_id).map_err(rejected)?;
        let state = match State::fold(facts.as_ref())? {
            None => {
                return Err(rejected(Rejection::EpisodeNotFound {
                    episode_id: self.episode_id,
                }));
            }
            Some(s) if s.closed => {
                return Err(rejected(Rejection::EpisodeClosed {
                    episode_id: self.episode_id,
                }));
            }
            Some(s) => s,
        };
        let r = &self.record;
        // ADR-0008 ambiguity 6: a record can't claim an allowed call that wasn't.
        if r.allowed && !state.opened.tools_allowed.contains(&r.tool_name) {
            return Err(rejected(Rejection::ToolNotAllowed {
                tool_name: r.tool_name.clone(),
            }));
        }
        let to_rec = |e: abw_canon::JcsError| rejected(Rejection::InvalidToolRecord(e.to_string()));
        let input = artifact_json(&r.input, JSON).map_err(to_rec)?;
        let output = artifact_json(&r.output.document(), JSON).map_err(to_rec)?;
        let error_class = match &r.output {
            ToolOutcome::Ok(_) => None,
            ToolOutcome::Err { class, .. } => Some(class.clone()),
        };
        let mut invoked = event(
            &TOOL_INVOKED,
            &ToolInvokedPayload {
                episode_id: self.episode_id,
                iteration: r.iteration,
                call_index: r.call_index,
                tool_name: r.tool_name.clone(),
                tool_version: r.tool_version.clone(),
                allowed: r.allowed,
                input_sha: input.sha().clone(),
                output_sha: output.sha().clone(),
                duration_ms: r.duration_ms,
                error_class,
            },
        )?;
        invoked.artifacts.push(input);
        invoked.artifacts.push(output);
        let mut events = vec![invoked];
        if !r.allowed {
            events.push(event(
                &GATE_DECIDED,
                &GateDecidedPayload {
                    gate: ALLOWLIST_ERROR_CLASS.to_string(),
                    decision: "refused".to_string(),
                    reason_class: "policy".to_string(),
                    subject: format!("tool:{}", r.tool_name),
                    episode_id: self.episode_id,
                },
            )?);
        }
        Ok(vec![StreamAppend {
            stream_id: stream,
            kind: KIND,
            expected: self.expected,
            events,
        }])
    }

    fn output(
        &self,
        recorded: &[Recorded],
        _facts: &Self::Facts,
    ) -> Result<ToolRecorded, DispatchError> {
        let invoked = recorded
            .iter()
            .find(|r| r.event_type == TOOL_INVOKED.event_type)
            .ok_or_else(|| DispatchError::Integrity("no tool.invoked recorded".into()))?;
        let p: ToolInvokedPayload = decode(&invoked.payload)?;
        Ok(ToolRecorded {
            seq: recorded.iter().map(|r| r.seq).max().unwrap_or(invoked.seq),
            input_sha: p.input_sha,
            output_sha: p.output_sha,
        })
    }
}

// ---- TX-C and abandonment ---------------------------------------------------------------

/// How a run ended, as the executor reports it to TX-C.
#[derive(Debug, Clone, PartialEq)]
pub struct CloseOutcome {
    pub status: EpisodeStatus,
    pub reason_class: Option<ReasonClass>,
    pub raw_output: Option<String>,
    pub usage: Usage,
}

/// ADR-0008 §4 outcome rules: `Success` has no reason, `Partial` and `Failure` need one,
/// `ProviderError` means `Failure`, and the abandonment reasons belong to [`Abandon`].
fn check_outcome(o: &CloseOutcome) -> Result<(), Rejection> {
    use EpisodeStatus::*;
    use ReasonClass::*;
    let bad = |m: &str| Err(Rejection::InvalidOutcome(m.to_string()));
    match (o.status, o.reason_class) {
        (Success, None) => Ok(()),
        (Success, Some(_)) => bad("a success has no reason class"),
        (Partial | Failure, None) => bad("a partial or failed close needs a reason class"),
        (_, Some(AbandonedTimeout | AbandonedDropped | LedgerError)) => {
            bad("abandonment reasons are recorded by episode::Abandon")
        }
        (Partial, Some(ProviderError)) => bad("a provider error is a failure"),
        (Partial | Failure, Some(_)) => Ok(()),
    }
}

/// The facts TX-C and abandonment fold.
#[doc(hidden)]
pub struct CloseFacts {
    episode: Option<State>,
    allowance: Option<AllowanceState>,
    price: Option<PricePolicy>,
}

async fn load_close(
    conn: &mut PgConnection,
    episode_id: Uuid,
    owner: &Option<String>,
    with_price: bool,
) -> Result<CloseFacts, DispatchError> {
    let ep = stream_of(episode_id).map_err(rejected)?;
    let mut ids = vec![ep.clone()];
    if let Some(o) = owner {
        ids.push(allowance::stream_of(o).map_err(rejected)?);
    }
    let streams = fold::load_streams(conn, &ids).await?;
    let episode = State::fold(streams.get(&ep))?;
    let allowance = match owner {
        Some(o) => {
            let al = allowance::stream_of(o).map_err(rejected)?;
            AllowanceState::fold(streams.get(&al))?
        }
        None => None,
    };
    let price = match (&episode, with_price) {
        (Some(e), true) => match policy::parse_version_ref(&e.opened.price_version) {
            Some((id, v)) => policy::price_at(conn, id, v).await?,
            None => None,
        },
        _ => None,
    };
    Ok(CloseFacts {
        episode,
        allowance,
        price,
    })
}

/// The open episode and its open hold, or the refusal.
fn running(
    episode_id: Uuid,
    facts: &CloseFacts,
) -> Result<(&State, &AllowanceState, allowance::Hold), DispatchError> {
    let ep = match &facts.episode {
        None => return Err(rejected(Rejection::EpisodeNotFound { episode_id })),
        Some(e) if e.closed => return Err(rejected(Rejection::EpisodeClosed { episode_id })),
        Some(e) => e,
    };
    let al = facts.allowance.as_ref().ok_or_else(|| {
        DispatchError::Integrity(format!("episode {episode_id} has no allowance"))
    })?;
    match al.holds.get(&episode_id) {
        Some(h) if h.open && h.hold_id == ep.opened.hold_id => Ok((ep, al, *h)),
        _ => Err(DispatchError::Integrity(format!(
            "episode {episode_id} is running without an open hold"
        ))),
    }
}

/// TX-C: close the episode and settle its hold in one transaction (§4.5 ⟨17⟩-⟨22⟩;
/// economy.md §5 step 3). The ledger prices the usage under the price version cited at
/// open; consumption is clamped to the hold (ADR-0008 ambiguities 2, 3). Idempotent on
/// `episode:<eid>:settle`.
#[derive(Debug, Clone)]
pub struct Close {
    episode_id: Uuid,
    expected: Expected,
    outcome: CloseOutcome,
    key: String,
}

impl Close {
    pub fn new(episode_id: Uuid, expected: Expected, outcome: CloseOutcome) -> Close {
        let eid = episode_id.hyphenated();
        Close {
            key: format!("{KIND}:{eid}/episode:{eid}:settle"),
            episode_id,
            expected,
            outcome,
        }
    }
}

/// The result of [`Close`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Closed {
    pub status: EpisodeStatus,
    pub consumed_cu: u64,
    pub released_cu: u64,
    pub split: Split,
    pub seq: u64,
}

fn owner_route<'a>(
    conn: &'a mut PgConnection,
    episode_id: Uuid,
) -> BoxFuture<'a, Result<Option<String>, DispatchError>> {
    Box::pin(owner_of(conn, episode_id))
}

fn episode_and_allowance(episode_id: Uuid, owner: &Option<String>) -> Vec<String> {
    let mut w: Vec<String> = stream_of(episode_id).into_iter().collect();
    if let Some(o) = owner {
        w.extend(allowance::stream_of(o));
    }
    w
}

impl CommandSpec for Close {
    const TYPE: &'static str = "episode.close";
    type Output = Closed;
    type Route = Option<String>;
    type Facts = CloseFacts;

    fn validate(&self, _ctx: &CommandCtx) -> Result<(), Rejection> {
        stream_of(self.episode_id)?;
        check_outcome(&self.outcome)
    }

    fn idempotency_key(&self) -> Option<&str> {
        Some(&self.key)
    }

    fn request(&self) -> Value {
        let o = &self.outcome;
        json!({
            "episode_id": self.episode_id,
            "expected": self.expected.head(),
            "status": o.status.as_str(),
            "reason_class": o.reason_class.map(ReasonClass::as_str),
            "raw_output": o.raw_output,
            "usage": o.usage,
        })
    }

    fn route<'a>(
        &'a self,
        conn: &'a mut PgConnection,
    ) -> BoxFuture<'a, Result<Option<String>, DispatchError>> {
        owner_route(conn, self.episode_id)
    }

    fn write_set(&self, route: &Option<String>) -> Vec<String> {
        episode_and_allowance(self.episode_id, route)
    }

    fn read_set(&self, _route: &Option<String>) -> Vec<String> {
        Vec::new()
    }

    fn fold<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        route: &'a Option<String>,
        _ctx: &'a CommandCtx,
    ) -> BoxFuture<'a, Result<CloseFacts, DispatchError>> {
        Box::pin(load_close(conn, self.episode_id, route, true))
    }

    fn decide(
        &self,
        heads: &Heads,
        facts: &CloseFacts,
        _ctx: &CommandCtx,
    ) -> Result<Vec<StreamAppend>, DispatchError> {
        let (ep, al, hold) = running(self.episode_id, facts)?;
        let price = facts.price.ok_or_else(|| {
            DispatchError::Integrity(format!(
                "price version {} of episode {} is missing",
                ep.opened.price_version, self.episode_id
            ))
        })?;
        let o = &self.outcome;
        let priced_cu = price.price(&o.usage);
        let consumed_cu = priced_cu.min(hold.amount_cu);
        let released_cu = hold.amount_cu - consumed_cu;
        let split = allowance::split(&al.balances, consumed_cu)?;
        let raw = o
            .raw_output
            .as_ref()
            .map(|t| artifact_bytes(t.clone().into_bytes(), Canon::RawUtf8, TEXT));
        let mut closed = event(
            &EPISODE_CLOSED,
            &ClosedPayload {
                episode_id: self.episode_id,
                status: o.status,
                reason_class: o.reason_class,
                raw_output_sha: raw.as_ref().map(|a| a.sha().clone()),
                tool_call_shas: ep.tool_output_shas.clone(),
                tokens_in: o.usage.input_tokens,
                tokens_out: o.usage.output_tokens,
                tool_calls: o.usage.tool_calls,
                consumed_cu,
            },
        )?;
        closed.artifacts.extend(raw);
        let owner = &ep.opened.owner_id;
        let al_stream = allowance::stream_of(owner).map_err(rejected)?;
        let consumed = allowance::consumed_event(&ConsumedPayload {
            owner_id: owner.clone(),
            episode_id: self.episode_id,
            hold_id: hold.hold_id,
            held_cu: hold.amount_cu,
            priced_cu,
            consumed_cu,
            released_cu,
            split,
            usage: o.usage,
            price_version: ep.opened.price_version.clone(),
            idempotency_key: format!("episode:{}:settle", self.episode_id.hyphenated()),
        })?;
        Ok(vec![
            StreamAppend {
                stream_id: stream_of(self.episode_id).map_err(rejected)?,
                kind: KIND,
                expected: self.expected,
                events: vec![closed],
            },
            StreamAppend {
                expected: allowance::expected_head(heads, &al_stream),
                stream_id: al_stream,
                kind: allowance::KIND,
                events: vec![consumed],
            },
        ])
    }

    fn output(&self, recorded: &[Recorded], _facts: &CloseFacts) -> Result<Closed, DispatchError> {
        let mut closed = None;
        let mut consumed = None;
        for r in recorded {
            if r.event_type == EPISODE_CLOSED.event_type {
                closed = Some((decode::<ClosedPayload>(&r.payload)?, r.seq));
            } else if r.event_type == registry::ALLOWANCE_CONSUMED.event_type {
                consumed = Some(decode::<ConsumedPayload>(&r.payload)?);
            }
        }
        match (closed, consumed) {
            (Some((c, seq)), Some(s)) => Ok(Closed {
                status: c.status,
                consumed_cu: s.consumed_cu,
                released_cu: s.released_cu,
                split: s.split,
                seq,
            }),
            _ => Err(DispatchError::Integrity(
                "episode.close must record episode.closed and allowance.consumed".into(),
            )),
        }
    }
}

/// Close an episode `failure` and release its whole hold, in one transaction (§4.5 ⟨6⟩,
/// §4.7; economy.md §5 step 4). Used by the executor after a failed TX-B (`LedgerError`),
/// by the reaper (`AbandonedTimeout`) and for dropped runs (`AbandonedDropped`).
/// Idempotent on `episode:<eid>:release`.
#[derive(Debug, Clone)]
pub struct Abandon {
    episode_id: Uuid,
    expected: Expected,
    reason: ReasonClass,
    key: String,
}

impl Abandon {
    pub fn new(episode_id: Uuid, expected: Expected, reason: ReasonClass) -> Abandon {
        let eid = episode_id.hyphenated();
        Abandon {
            key: format!("{KIND}:{eid}/episode:{eid}:release"),
            episode_id,
            expected,
            reason,
        }
    }
}

/// The result of [`Abandon`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Abandoned {
    pub released_cu: u64,
    pub seq: u64,
}

impl CommandSpec for Abandon {
    const TYPE: &'static str = "episode.abandon";
    type Output = Abandoned;
    type Route = Option<String>;
    type Facts = CloseFacts;

    fn validate(&self, _ctx: &CommandCtx) -> Result<(), Rejection> {
        stream_of(self.episode_id)?;
        match self.reason {
            ReasonClass::AbandonedTimeout
            | ReasonClass::AbandonedDropped
            | ReasonClass::LedgerError => Ok(()),
            other => Err(Rejection::InvalidOutcome(format!(
                "{} is a close outcome, not an abandonment",
                other.as_str()
            ))),
        }
    }

    fn idempotency_key(&self) -> Option<&str> {
        Some(&self.key)
    }

    fn request(&self) -> Value {
        json!({
            "episode_id": self.episode_id,
            "expected": self.expected.head(),
            "reason_class": self.reason.as_str(),
        })
    }

    fn route<'a>(
        &'a self,
        conn: &'a mut PgConnection,
    ) -> BoxFuture<'a, Result<Option<String>, DispatchError>> {
        owner_route(conn, self.episode_id)
    }

    fn write_set(&self, route: &Option<String>) -> Vec<String> {
        episode_and_allowance(self.episode_id, route)
    }

    fn read_set(&self, _route: &Option<String>) -> Vec<String> {
        Vec::new()
    }

    fn fold<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        route: &'a Option<String>,
        _ctx: &'a CommandCtx,
    ) -> BoxFuture<'a, Result<CloseFacts, DispatchError>> {
        Box::pin(load_close(conn, self.episode_id, route, false))
    }

    fn decide(
        &self,
        heads: &Heads,
        facts: &CloseFacts,
        _ctx: &CommandCtx,
    ) -> Result<Vec<StreamAppend>, DispatchError> {
        let (ep, _al, hold) = running(self.episode_id, facts)?;
        let closed = event(
            &EPISODE_CLOSED,
            &ClosedPayload {
                episode_id: self.episode_id,
                status: EpisodeStatus::Failure,
                reason_class: Some(self.reason),
                raw_output_sha: None,
                tool_call_shas: ep.tool_output_shas.clone(),
                tokens_in: 0,
                tokens_out: 0,
                tool_calls: ep.tool_output_shas.len() as u64,
                consumed_cu: 0,
            },
        )?;
        let owner = &ep.opened.owner_id;
        let al_stream = allowance::stream_of(owner).map_err(rejected)?;
        let released = allowance::hold_released_event(&HoldReleasedPayload {
            owner_id: owner.clone(),
            episode_id: self.episode_id,
            hold_id: hold.hold_id,
            amount_cu: hold.amount_cu,
            reason_class: self.reason.as_str().to_string(),
            idempotency_key: format!("episode:{}:release", self.episode_id.hyphenated()),
        })?;
        Ok(vec![
            StreamAppend {
                stream_id: stream_of(self.episode_id).map_err(rejected)?,
                kind: KIND,
                expected: self.expected,
                events: vec![closed],
            },
            StreamAppend {
                expected: allowance::expected_head(heads, &al_stream),
                stream_id: al_stream,
                kind: allowance::KIND,
                events: vec![released],
            },
        ])
    }

    fn output(
        &self,
        recorded: &[Recorded],
        _facts: &CloseFacts,
    ) -> Result<Abandoned, DispatchError> {
        let seq = recorded
            .iter()
            .find(|r| r.event_type == EPISODE_CLOSED.event_type)
            .map(|r| r.seq);
        let released = recorded
            .iter()
            .find(|r| r.event_type == registry::ALLOWANCE_HOLD_RELEASED.event_type)
            .map(|r| decode::<HoldReleasedPayload>(&r.payload))
            .transpose()?;
        match (seq, released) {
            (Some(seq), Some(r)) => Ok(Abandoned {
                released_cu: r.amount_cu,
                seq,
            }),
            _ => Err(DispatchError::Integrity(
                "episode.abandon must record episode.closed and allowance.hold_released".into(),
            )),
        }
    }
}
