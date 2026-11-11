//! The allowance aggregate (economy.md §3-§5; ADR-0006; ADR-0008 §2-§4).
//!
//! One stream `allowance:<owner_id>` per owner, in compute units. Units come only from
//! grants under a human-accepted grant policy (onboarding when the allowance opens, the
//! monthly free tier at the first hold of a month, operator grants) and leave only by
//! consumption at an episode's close. No event names a second owner (ECO-4), every event
//! carries its idempotency key (ECO-6), and `proj.allowances` is guarded by the database
//! (ECO-1, ECO-7). `allowance.funded` arrives with M4 (ADR-0008 §1 item 6).

use crate::command::{
    BoxFuture, Expected, Heads, NewEvent, Recorded, StreamAppend, sealed::CommandSpec, stream_id,
};
use crate::ctx::{ActorKind, CommandCtx};
use crate::episode::{Split, Usage};
use crate::error::{DispatchError, Rejection};
use crate::fold::{self, StoredEvent};
use crate::policy::{self, Cited, GrantPolicy, PolicyRef, decode, to_value};
use crate::registry::{
    ALLOWANCE_CONSUMED, ALLOWANCE_GRANTED, ALLOWANCE_HOLD_PLACED, ALLOWANCE_HOLD_RELEASED,
    ALLOWANCE_OPENED,
};
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::PgConnection;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub(crate) const KIND: &str = "allowance";

/// The free-tier period of `at`: its calendar month in UTC, `YYYY-MM` (economy.md §3).
pub fn period(at: DateTime<Utc>) -> String {
    format!("{:04}-{:02}", at.year(), at.month())
}

pub(crate) fn stream_of(owner_id: &str) -> Result<String, Rejection> {
    stream_id(KIND, owner_id)
}

// ---- payloads (ADR-0008 §2) -------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GrantKind {
    Onboarding,
    FreeTier,
    Operator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OpenedPayload {
    pub(crate) owner_id: String,
    pub(crate) grant_policy_version: String,
    pub(crate) idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GrantedPayload {
    pub(crate) owner_id: String,
    pub(crate) kind: GrantKind,
    pub(crate) amount_cu: u64,
    pub(crate) grant_policy_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) grant_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) period: Option<String>,
    pub(crate) idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HoldPlacedPayload {
    pub(crate) owner_id: String,
    pub(crate) episode_id: Uuid,
    pub(crate) hold_id: Uuid,
    pub(crate) amount_cu: u64,
    /// Always null until workspaces and spend caps arrive (ADR-0008 §1 item 2).
    pub(crate) workspace_id: Option<Uuid>,
    pub(crate) idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ConsumedPayload {
    pub(crate) owner_id: String,
    pub(crate) episode_id: Uuid,
    pub(crate) hold_id: Uuid,
    pub(crate) held_cu: u64,
    pub(crate) priced_cu: u64,
    pub(crate) consumed_cu: u64,
    pub(crate) released_cu: u64,
    pub(crate) split: Split,
    pub(crate) usage: Usage,
    pub(crate) price_version: String,
    pub(crate) idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HoldReleasedPayload {
    pub(crate) owner_id: String,
    pub(crate) episode_id: Uuid,
    pub(crate) hold_id: Uuid,
    pub(crate) amount_cu: u64,
    pub(crate) reason_class: String,
    pub(crate) idempotency_key: String,
}

// ---- the fold ---------------------------------------------------------------------------

/// Bucket balances and lifetime totals (economy.md §3), folded from the stream's events.
/// One implementation for decisions, the inline projector and rebuild.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Balances {
    pub(crate) free_tier: u64,
    pub(crate) granted: u64,
    pub(crate) purchased: u64,
    pub(crate) held: u64,
    pub(crate) total_granted_free_tier: u64,
    pub(crate) total_granted: u64,
    pub(crate) total_purchased: u64,
    pub(crate) total_consumed: u64,
}

fn add(a: u64, b: u64) -> Result<u64, String> {
    a.checked_add(b)
        .ok_or_else(|| "allowance overflow".to_string())
}

fn sub(a: u64, b: u64, what: &str) -> Result<u64, String> {
    a.checked_sub(b)
        .ok_or_else(|| format!("{what} would go negative ({a} - {b})"))
}

impl Balances {
    /// `available = free_tier + granted + purchased - held` (economy.md §3).
    pub(crate) fn available(&self) -> u64 {
        self.free_tier
            .saturating_add(self.granted)
            .saturating_add(self.purchased)
            .saturating_sub(self.held)
    }

    /// Applies one allowance event. Other event types leave the balances unchanged.
    pub(crate) fn apply(&mut self, event_type: &str, payload: &Value) -> Result<(), String> {
        let de = |e: serde_json::Error| format!("{event_type}: {e}");
        match event_type {
            "allowance.granted" => {
                let g: GrantedPayload = serde_json::from_value(payload.clone()).map_err(de)?;
                if g.kind == GrantKind::FreeTier {
                    self.free_tier = add(self.free_tier, g.amount_cu)?;
                    self.total_granted_free_tier = add(self.total_granted_free_tier, g.amount_cu)?;
                } else {
                    self.granted = add(self.granted, g.amount_cu)?;
                    self.total_granted = add(self.total_granted, g.amount_cu)?;
                }
            }
            "allowance.hold_placed" => {
                let h: HoldPlacedPayload = serde_json::from_value(payload.clone()).map_err(de)?;
                self.held = add(self.held, h.amount_cu)?;
            }
            "allowance.consumed" => {
                let c: ConsumedPayload = serde_json::from_value(payload.clone()).map_err(de)?;
                self.held = sub(self.held, c.held_cu, "held")?;
                self.free_tier = sub(self.free_tier, c.split.free_tier, "free_tier")?;
                self.granted = sub(self.granted, c.split.granted, "granted")?;
                self.purchased = sub(self.purchased, c.split.purchased, "purchased")?;
                self.total_consumed = add(self.total_consumed, c.consumed_cu)?;
            }
            "allowance.hold_released" => {
                let r: HoldReleasedPayload = serde_json::from_value(payload.clone()).map_err(de)?;
                self.held = sub(self.held, r.amount_cu, "held")?;
            }
            _ => {}
        }
        Ok(())
    }
}

/// One hold as the allowance stream records it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Hold {
    pub(crate) hold_id: Uuid,
    pub(crate) amount_cu: u64,
    pub(crate) open: bool,
}

/// What decisions read about an allowance.
#[derive(Debug, Clone, Default)]
pub(crate) struct State {
    pub(crate) balances: Balances,
    pub(crate) operator_total: u64,
    pub(crate) free_tier_periods: BTreeSet<String>,
    pub(crate) holds: BTreeMap<Uuid, Hold>,
}

impl State {
    /// The state after `events`, or `None` if the allowance was never opened.
    pub(crate) fn fold(events: Option<&Vec<StoredEvent>>) -> Result<Option<State>, DispatchError> {
        let Some(events) = events.filter(|e| !e.is_empty()) else {
            return Ok(None);
        };
        let mut s = State::default();
        for e in events {
            s.balances
                .apply(&e.event_type, &e.payload)
                .map_err(DispatchError::Integrity)?;
            match e.event_type.as_str() {
                "allowance.granted" => {
                    let g: GrantedPayload = e.decode()?;
                    match g.kind {
                        GrantKind::Operator => {
                            s.operator_total = s.operator_total.saturating_add(g.amount_cu);
                        }
                        GrantKind::FreeTier => {
                            if let Some(p) = g.period {
                                s.free_tier_periods.insert(p);
                            }
                        }
                        GrantKind::Onboarding => {}
                    }
                }
                "allowance.hold_placed" => {
                    let h: HoldPlacedPayload = e.decode()?;
                    s.holds.insert(
                        h.episode_id,
                        Hold {
                            hold_id: h.hold_id,
                            amount_cu: h.amount_cu,
                            open: true,
                        },
                    );
                }
                "allowance.consumed" | "allowance.hold_released" => {
                    let episode: Uuid = match e.payload.get("episode_id").and_then(Value::as_str) {
                        Some(t) => Uuid::parse_str(t)
                            .map_err(|err| DispatchError::Integrity(err.to_string()))?,
                        None => {
                            return Err(DispatchError::Integrity(format!(
                                "{} without episode_id",
                                e.event_type
                            )));
                        }
                    };
                    if let Some(h) = s.holds.get_mut(&episode) {
                        h.open = false;
                    }
                }
                _ => {}
            }
        }
        Ok(Some(s))
    }
}

/// The `allowance.granted { kind: free_tier }` of `period` that tops `free_tier` up to the
/// policy cap, or `None` if this period already has one (economy.md §3 "Free tier"; ADR-0008
/// §4: appended even when it grants 0, so the month is marked).
pub(crate) fn free_tier_top_up(
    owner_id: &str,
    state: &State,
    grant: &GrantPolicy,
    grant_ref: &PolicyRef,
    period: &str,
) -> Option<GrantedPayload> {
    if state.free_tier_periods.contains(period) {
        return None;
    }
    Some(GrantedPayload {
        owner_id: owner_id.to_string(),
        kind: GrantKind::FreeTier,
        amount_cu: grant
            .free_tier_cap_cu
            .saturating_sub(state.balances.free_tier),
        grant_policy_version: grant_ref.to_string(),
        reason: None,
        grant_id: None,
        period: Some(period.to_string()),
        idempotency_key: format!("free_tier:{owner_id}:{period}"),
    })
}

/// The split of `consumed` over the buckets: free tier, then granted, then purchased
/// (economy.md §3 "Consumption order").
pub(crate) fn split(b: &Balances, consumed: u64) -> Result<Split, DispatchError> {
    let free_tier = consumed.min(b.free_tier);
    let rest = consumed - free_tier;
    let granted = rest.min(b.granted);
    let rest = rest - granted;
    let purchased = rest.min(b.purchased);
    if rest - purchased > 0 {
        return Err(DispatchError::Integrity(format!(
            "consumption of {consumed} CU exceeds the buckets"
        )));
    }
    Ok(Split {
        free_tier,
        granted,
        purchased,
    })
}

// ---- commands ---------------------------------------------------------------------------

/// Facts shared by the allowance commands: the stream and the cited grant policy.
#[doc(hidden)]
pub struct Facts {
    events: Option<Vec<StoredEvent>>,
    cited: Cited,
}

async fn load(
    conn: &mut PgConnection,
    owner_id: &str,
    grant: &PolicyRef,
    tenant: &str,
    at: DateTime<Utc>,
) -> Result<Facts, DispatchError> {
    let stream = stream_of(owner_id).map_err(DispatchError::Rejected)?;
    let mut streams = fold::load_streams(conn, std::slice::from_ref(&stream)).await?;
    let cited = policy::cite(conn, grant, policy::PolicyKind::Grant, tenant, at).await?;
    Ok(Facts {
        events: streams.remove(&stream),
        cited,
    })
}

fn expected_at(heads: &Heads, stream: &str) -> Expected {
    match heads.head(stream) {
        0 => Expected::NoStream,
        n => Expected::Exact(n),
    }
}

fn event(
    def: &'static crate::registry::EventTypeDef,
    payload: &impl Serialize,
) -> Result<NewEvent, DispatchError> {
    NewEvent::new(def, to_value(payload)?).map_err(DispatchError::Rejected)
}

/// Open `owner_id`'s allowance and grant the onboarding amount of `grant_policy`, in one
/// transaction. The only onboarding path, idempotent on `open:<owner>` (ADR-0008 §4).
#[derive(Debug, Clone)]
pub struct Open {
    owner_id: String,
    grant_policy: PolicyRef,
    at: DateTime<Utc>,
    key: String,
}

impl Open {
    pub fn new(owner_id: impl Into<String>, grant_policy: PolicyRef) -> Open {
        let owner_id = owner_id.into();
        Open {
            key: format!("{KIND}:{owner_id}/open:{owner_id}"),
            owner_id,
            grant_policy,
            // The policy must be active now; the instant is not part of the request.
            at: Utc::now(),
        }
    }
}

/// The result of [`Open`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opened {
    pub owner_id: String,
    pub onboarding_cu: u64,
}

impl CommandSpec for Open {
    const TYPE: &'static str = "allowance.open";
    type Output = Opened;
    type Route = ();
    type Facts = Facts;

    fn validate(&self, _ctx: &CommandCtx) -> Result<(), Rejection> {
        stream_of(&self.owner_id)?;
        policy::ref_stream(&self.grant_policy).map(|_| ())
    }

    fn idempotency_key(&self) -> Option<&str> {
        Some(&self.key)
    }

    fn request(&self) -> Value {
        json!({"owner_id": self.owner_id, "grant_policy": self.grant_policy})
    }

    fn route<'a>(
        &'a self,
        _conn: &'a mut PgConnection,
    ) -> BoxFuture<'a, Result<(), DispatchError>> {
        fold::ready(())
    }

    fn write_set(&self, _route: &()) -> Vec<String> {
        stream_of(&self.owner_id).into_iter().collect()
    }

    fn read_set(&self, _route: &()) -> Vec<String> {
        vec![self.grant_policy.stream_id()]
    }

    fn fold<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        _route: &'a (),
        ctx: &'a CommandCtx,
    ) -> BoxFuture<'a, Result<Facts, DispatchError>> {
        Box::pin(load(
            conn,
            &self.owner_id,
            &self.grant_policy,
            &ctx.tenant,
            self.at,
        ))
    }

    fn decide(
        &self,
        heads: &Heads,
        facts: &Facts,
        ctx: &CommandCtx,
    ) -> Result<Vec<StreamAppend>, DispatchError> {
        let stream = stream_of(&self.owner_id).map_err(DispatchError::Rejected)?;
        if State::fold(facts.events.as_ref())?.is_some() {
            return Err(DispatchError::Rejected(Rejection::AllowanceAlreadyOpen {
                owner_id: self.owner_id.clone(),
            }));
        }
        let grant = policy::check_grant(&self.grant_policy, &facts.cited, &ctx.tenant)
            .map_err(DispatchError::Rejected)?;
        let version = self.grant_policy.to_string();
        let mut events = vec![event(
            &ALLOWANCE_OPENED,
            &OpenedPayload {
                owner_id: self.owner_id.clone(),
                grant_policy_version: version.clone(),
                idempotency_key: format!("open:{}", self.owner_id),
            },
        )?];
        if grant.onboarding_cu > 0 {
            events.push(event(
                &ALLOWANCE_GRANTED,
                &GrantedPayload {
                    owner_id: self.owner_id.clone(),
                    kind: GrantKind::Onboarding,
                    amount_cu: grant.onboarding_cu,
                    grant_policy_version: version,
                    reason: None,
                    grant_id: None,
                    period: None,
                    idempotency_key: format!("onboarding:{}", self.owner_id),
                },
            )?);
        }
        Ok(vec![StreamAppend {
            expected: expected_at(heads, &stream),
            stream_id: stream,
            kind: KIND,
            events,
        }])
    }

    fn output(&self, recorded: &[Recorded], _facts: &Facts) -> Result<Opened, DispatchError> {
        let mut onboarding_cu = 0;
        for r in recorded
            .iter()
            .filter(|r| r.event_type == ALLOWANCE_GRANTED.event_type)
        {
            let g: GrantedPayload = decode(&r.payload)?;
            onboarding_cu += g.amount_cu;
        }
        Ok(Opened {
            owner_id: self.owner_id.clone(),
            onboarding_cu,
        })
    }
}

/// An ad-hoc grant by a human operator, with a reason, within the grant policy's per-owner
/// limit (economy.md §3 "Operator"). Idempotent on `operator:<grant_id>`.
#[derive(Debug, Clone)]
pub struct OperatorGrant {
    owner_id: String,
    grant_id: Uuid,
    amount_cu: u64,
    reason: String,
    grant_policy: PolicyRef,
    at: DateTime<Utc>,
    key: String,
}

impl OperatorGrant {
    pub fn new(
        owner_id: impl Into<String>,
        grant_id: Uuid,
        amount_cu: u64,
        reason: impl Into<String>,
        grant_policy: PolicyRef,
    ) -> OperatorGrant {
        let owner_id = owner_id.into();
        OperatorGrant {
            key: format!("{KIND}:{owner_id}/operator:{}", grant_id.hyphenated()),
            owner_id,
            grant_id,
            amount_cu,
            reason: reason.into(),
            grant_policy,
            at: Utc::now(),
        }
    }
}

/// The result of [`OperatorGrant`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Granted {
    pub owner_id: String,
    pub amount_cu: u64,
    pub seq: u64,
}

impl CommandSpec for OperatorGrant {
    const TYPE: &'static str = "allowance.operator_grant";
    type Output = Granted;
    type Route = ();
    type Facts = Facts;

    fn validate(&self, ctx: &CommandCtx) -> Result<(), Rejection> {
        if ctx.actor.kind != ActorKind::Human {
            return Err(Rejection::NotHuman {
                actor_kind: ctx.actor.kind,
            });
        }
        if self.reason.trim().is_empty() {
            return Err(Rejection::ReasonRequired);
        }
        stream_of(&self.owner_id)?;
        policy::ref_stream(&self.grant_policy).map(|_| ())
    }

    fn idempotency_key(&self) -> Option<&str> {
        Some(&self.key)
    }

    fn request(&self) -> Value {
        json!({
            "owner_id": self.owner_id,
            "grant_id": self.grant_id,
            "amount_cu": self.amount_cu,
            "reason": self.reason,
            "grant_policy": self.grant_policy,
        })
    }

    fn route<'a>(
        &'a self,
        _conn: &'a mut PgConnection,
    ) -> BoxFuture<'a, Result<(), DispatchError>> {
        fold::ready(())
    }

    fn write_set(&self, _route: &()) -> Vec<String> {
        stream_of(&self.owner_id).into_iter().collect()
    }

    fn read_set(&self, _route: &()) -> Vec<String> {
        vec![self.grant_policy.stream_id()]
    }

    fn fold<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        _route: &'a (),
        ctx: &'a CommandCtx,
    ) -> BoxFuture<'a, Result<Facts, DispatchError>> {
        Box::pin(load(
            conn,
            &self.owner_id,
            &self.grant_policy,
            &ctx.tenant,
            self.at,
        ))
    }

    fn decide(
        &self,
        heads: &Heads,
        facts: &Facts,
        ctx: &CommandCtx,
    ) -> Result<Vec<StreamAppend>, DispatchError> {
        let stream = stream_of(&self.owner_id).map_err(DispatchError::Rejected)?;
        let Some(state) = State::fold(facts.events.as_ref())? else {
            return Err(DispatchError::Rejected(Rejection::AllowanceNotOpen {
                owner_id: self.owner_id.clone(),
            }));
        };
        let grant = policy::check_grant(&self.grant_policy, &facts.cited, &ctx.tenant)
            .map_err(DispatchError::Rejected)?;
        let requested_total = state.operator_total.saturating_add(self.amount_cu);
        if requested_total > grant.operator_limit_cu {
            return Err(DispatchError::Rejected(Rejection::GrantLimitExceeded {
                limit: grant.operator_limit_cu,
                requested_total,
            }));
        }
        let payload = GrantedPayload {
            owner_id: self.owner_id.clone(),
            kind: GrantKind::Operator,
            amount_cu: self.amount_cu,
            grant_policy_version: self.grant_policy.to_string(),
            reason: Some(self.reason.clone()),
            grant_id: Some(self.grant_id),
            period: None,
            idempotency_key: format!("operator:{}", self.grant_id.hyphenated()),
        };
        Ok(vec![StreamAppend {
            expected: expected_at(heads, &stream),
            stream_id: stream,
            kind: KIND,
            events: vec![event(&ALLOWANCE_GRANTED, &payload)?],
        }])
    }

    fn output(&self, recorded: &[Recorded], _facts: &Facts) -> Result<Granted, DispatchError> {
        match recorded {
            [r] => {
                let g: GrantedPayload = decode(&r.payload)?;
                Ok(Granted {
                    owner_id: g.owner_id,
                    amount_cu: g.amount_cu,
                    seq: r.seq,
                })
            }
            _ => Err(DispatchError::Integrity(
                "allowance.operator_grant expected one recorded event".into(),
            )),
        }
    }
}

pub(crate) fn granted_event(p: &GrantedPayload) -> Result<NewEvent, DispatchError> {
    event(&ALLOWANCE_GRANTED, p)
}

pub(crate) fn hold_placed_event(p: &HoldPlacedPayload) -> Result<NewEvent, DispatchError> {
    event(&ALLOWANCE_HOLD_PLACED, p)
}

pub(crate) fn consumed_event(p: &ConsumedPayload) -> Result<NewEvent, DispatchError> {
    event(&ALLOWANCE_CONSUMED, p)
}

pub(crate) fn hold_released_event(p: &HoldReleasedPayload) -> Result<NewEvent, DispatchError> {
    event(&ALLOWANCE_HOLD_RELEASED, p)
}

pub(crate) fn expected_head(heads: &Heads, stream: &str) -> Expected {
    expected_at(heads, stream)
}
