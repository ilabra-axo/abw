//! The M2 inline projections as pure folds (target-system-design §3.4).
//!
//! Each `apply_*` takes the previous row (if any) and one event and returns the new row,
//! or `None` when the event doesn't touch the table. The inline projector (`projection.rs`)
//! runs them inside the append transaction and `admin::rebuild_diff` replays them over
//! `ledger.events`, so the live table and its rebuild come from one implementation.
//! Row fields are named and typed as the columns of `0003_m2.sql`; rows travel to and from
//! the database as `to_jsonb` / `jsonb_populate_record` documents.

use crate::agent::{RetiredPayload, VersionCreatedPayload};
use crate::allowance::{Balances, ConsumedPayload, HoldPlacedPayload, HoldReleasedPayload};
use crate::episode::{ClosedPayload, OpenedPayload};
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// One event as a projector sees it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EventView<'a> {
    pub(crate) stream_id: &'a str,
    pub(crate) global_pos: i64,
    pub(crate) event_type: &'a str,
    pub(crate) payload: &'a Value,
    pub(crate) tenant: &'a str,
    pub(crate) actor: &'a str,
}

fn de<T: DeserializeOwned>(ev: &EventView<'_>) -> Result<T, String> {
    serde_json::from_value(ev.payload.clone()).map_err(|e| format!("{}: {e}", ev.event_type))
}

fn i(v: u64) -> Result<i64, String> {
    i64::try_from(v).map_err(|_| format!("{v} does not fit bigint"))
}

fn u(v: i64) -> Result<u64, String> {
    u64::try_from(v).map_err(|_| format!("negative amount {v}"))
}

fn i32_of(v: u64) -> Result<i32, String> {
    i32::try_from(v).map_err(|_| format!("{v} does not fit int"))
}

// ---- proj.allowances --------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AllowanceRow {
    pub(crate) owner_id: String,
    pub(crate) stream_id: String,
    pub(crate) tenant_id: String,
    pub(crate) free_tier: i64,
    pub(crate) granted: i64,
    pub(crate) purchased: i64,
    pub(crate) held: i64,
    pub(crate) available: i64,
    pub(crate) total_granted_free_tier: i64,
    pub(crate) total_granted: i64,
    pub(crate) total_purchased: i64,
    pub(crate) total_consumed: i64,
    pub(crate) applied_pos: i64,
}

impl AllowanceRow {
    fn balances(&self) -> Result<Balances, String> {
        Ok(Balances {
            free_tier: u(self.free_tier)?,
            granted: u(self.granted)?,
            purchased: u(self.purchased)?,
            held: u(self.held)?,
            total_granted_free_tier: u(self.total_granted_free_tier)?,
            total_granted: u(self.total_granted)?,
            total_purchased: u(self.total_purchased)?,
            total_consumed: u(self.total_consumed)?,
        })
    }
}

/// The key of `proj.allowances` an event touches: the owner of its allowance stream.
pub(crate) fn allowance_key(ev: &EventView<'_>) -> Option<String> {
    if !ev.event_type.starts_with("allowance.") {
        return None;
    }
    ev.stream_id.strip_prefix("allowance:").map(str::to_string)
}

pub(crate) fn apply_allowance(
    prev: Option<&AllowanceRow>,
    ev: &EventView<'_>,
) -> Result<Option<AllowanceRow>, String> {
    let Some(owner) = allowance_key(ev) else {
        return Ok(None);
    };
    let mut b = match (prev, ev.event_type) {
        (None, "allowance.opened") => Balances::default(),
        (Some(_), "allowance.opened") => return Err(format!("allowance {owner} opened twice")),
        (Some(p), _) => p.balances()?,
        (None, t) => return Err(format!("{t} before allowance.opened for {owner}")),
    };
    b.apply(ev.event_type, ev.payload)?;
    Ok(Some(AllowanceRow {
        owner_id: owner,
        stream_id: ev.stream_id.to_string(),
        tenant_id: ev.tenant.to_string(),
        free_tier: i(b.free_tier)?,
        granted: i(b.granted)?,
        purchased: i(b.purchased)?,
        held: i(b.held)?,
        available: i(b.available())?,
        total_granted_free_tier: i(b.total_granted_free_tier)?,
        total_granted: i(b.total_granted)?,
        total_purchased: i(b.total_purchased)?,
        total_consumed: i(b.total_consumed)?,
        applied_pos: ev.global_pos,
    }))
}

// ---- proj.allowance_holds ---------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HoldRow {
    pub(crate) hold_id: Uuid,
    pub(crate) owner_id: String,
    pub(crate) episode_id: Uuid,
    pub(crate) tenant_id: String,
    pub(crate) amount_cu: i64,
    pub(crate) status: String,
    pub(crate) applied_pos: i64,
}

pub(crate) fn hold_key(ev: &EventView<'_>) -> Result<Option<Uuid>, String> {
    match ev.event_type {
        "allowance.hold_placed" => Ok(Some(de::<HoldPlacedPayload>(ev)?.hold_id)),
        "allowance.consumed" => Ok(Some(de::<ConsumedPayload>(ev)?.hold_id)),
        "allowance.hold_released" => Ok(Some(de::<HoldReleasedPayload>(ev)?.hold_id)),
        _ => Ok(None),
    }
}

pub(crate) fn apply_hold(
    prev: Option<&HoldRow>,
    ev: &EventView<'_>,
) -> Result<Option<HoldRow>, String> {
    let closed = |status: &str| match prev {
        Some(p) if p.status == "open" => Ok(Some(HoldRow {
            status: status.to_string(),
            applied_pos: ev.global_pos,
            ..p.clone()
        })),
        _ => Err(format!("{} on a hold that is not open", ev.event_type)),
    };
    match ev.event_type {
        "allowance.hold_placed" => {
            if prev.is_some() {
                return Err("a hold placed twice".into());
            }
            let h: HoldPlacedPayload = de(ev)?;
            Ok(Some(HoldRow {
                hold_id: h.hold_id,
                owner_id: h.owner_id,
                episode_id: h.episode_id,
                tenant_id: ev.tenant.to_string(),
                amount_cu: i(h.amount_cu)?,
                status: "open".into(),
                applied_pos: ev.global_pos,
            }))
        }
        "allowance.consumed" => closed("consumed"),
        "allowance.hold_released" => closed("released"),
        _ => Ok(None),
    }
}

// ---- proj.episodes ----------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct EpisodeRow {
    pub(crate) episode_id: Uuid,
    pub(crate) stream_id: String,
    pub(crate) tenant_id: String,
    pub(crate) owner_id: String,
    pub(crate) agent_id: String,
    pub(crate) version_number: i32,
    pub(crate) card_sha: String,
    pub(crate) workspace_id: Option<Uuid>,
    pub(crate) hold_id: Uuid,
    pub(crate) hold_cu: i64,
    pub(crate) consumed_cu: Option<i64>,
    pub(crate) status: String,
    pub(crate) reason_class: Option<String>,
    pub(crate) input_sha: String,
    pub(crate) raw_output_sha: Option<String>,
    pub(crate) tool_call_shas: Vec<String>,
    pub(crate) opened_at: DateTime<Utc>,
    pub(crate) opened_pos: i64,
    pub(crate) closed_pos: Option<i64>,
    pub(crate) applied_pos: i64,
}

pub(crate) fn episode_key(ev: &EventView<'_>) -> Option<Uuid> {
    match ev.event_type {
        "episode.opened" | "episode.closed" => ev
            .stream_id
            .strip_prefix("episode:")
            .and_then(|s| Uuid::parse_str(s).ok()),
        _ => None,
    }
}

pub(crate) fn apply_episode(
    prev: Option<&EpisodeRow>,
    ev: &EventView<'_>,
) -> Result<Option<EpisodeRow>, String> {
    match ev.event_type {
        "episode.opened" => {
            if prev.is_some() {
                return Err("an episode opened twice".into());
            }
            let o: OpenedPayload = de(ev)?;
            Ok(Some(EpisodeRow {
                episode_id: o.episode_id,
                stream_id: ev.stream_id.to_string(),
                tenant_id: ev.tenant.to_string(),
                owner_id: o.owner_id,
                agent_id: o.agent_id,
                version_number: i32_of(o.version_number)?,
                card_sha: o.card_sha.to_string(),
                workspace_id: o.workspace_id,
                hold_id: o.hold_id,
                hold_cu: i(o.hold_cu)?,
                consumed_cu: None,
                status: "running".into(),
                reason_class: None,
                input_sha: o.input_sha.to_string(),
                raw_output_sha: None,
                tool_call_shas: Vec::new(),
                opened_at: o.opened_at,
                opened_pos: ev.global_pos,
                closed_pos: None,
                applied_pos: ev.global_pos,
            }))
        }
        "episode.closed" => {
            let Some(p) = prev.filter(|p| p.status == "running") else {
                return Err("episode.closed on an episode that is not running".into());
            };
            let c: ClosedPayload = de(ev)?;
            Ok(Some(EpisodeRow {
                consumed_cu: Some(i(c.consumed_cu)?),
                status: c.status.as_str().to_string(),
                reason_class: c.reason_class.map(|r| r.as_str().to_string()),
                raw_output_sha: c.raw_output_sha.map(|s| s.to_string()),
                tool_call_shas: c.tool_call_shas.iter().map(ToString::to_string).collect(),
                closed_pos: Some(ev.global_pos),
                applied_pos: ev.global_pos,
                ..p.clone()
            }))
        }
        _ => Ok(None),
    }
}

// ---- proj.agent_versions and proj.agent_heads -------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AgentVersionRow {
    pub(crate) agent_id: String,
    pub(crate) tenant_id: String,
    pub(crate) version_number: i32,
    pub(crate) card_sha: String,
    pub(crate) prompt_sha: String,
    pub(crate) reason: String,
    pub(crate) actor: String,
    pub(crate) event_pos: i64,
}

pub(crate) fn agent_version(ev: &EventView<'_>) -> Result<Option<AgentVersionRow>, String> {
    if ev.event_type != "agent.version_created" {
        return Ok(None);
    }
    let v: VersionCreatedPayload = de(ev)?;
    Ok(Some(AgentVersionRow {
        agent_id: v.agent_id,
        tenant_id: ev.tenant.to_string(),
        version_number: i32_of(v.version_number)?,
        card_sha: v.card_sha.to_string(),
        prompt_sha: v.prompt_sha.to_string(),
        reason: v.reason.as_str().to_string(),
        actor: ev.actor.to_string(),
        event_pos: ev.global_pos,
    }))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AgentHeadRow {
    pub(crate) agent_id: String,
    pub(crate) tenant_id: String,
    pub(crate) version_number: i32,
    pub(crate) card_sha: String,
    pub(crate) retired: bool,
    pub(crate) applied_pos: i64,
}

pub(crate) fn agent_key(ev: &EventView<'_>) -> Option<String> {
    match ev.event_type {
        "agent.version_created" | "agent.retired" => {
            ev.stream_id.strip_prefix("agent:").map(str::to_string)
        }
        _ => None,
    }
}

pub(crate) fn apply_agent_head(
    prev: Option<&AgentHeadRow>,
    ev: &EventView<'_>,
) -> Result<Option<AgentHeadRow>, String> {
    match ev.event_type {
        "agent.version_created" => {
            let v: VersionCreatedPayload = de(ev)?;
            Ok(Some(AgentHeadRow {
                agent_id: v.agent_id,
                tenant_id: ev.tenant.to_string(),
                version_number: i32_of(v.version_number)?,
                card_sha: v.card_sha.to_string(),
                retired: false,
                applied_pos: ev.global_pos,
            }))
        }
        "agent.retired" => {
            let _: RetiredPayload = de(ev)?;
            match prev {
                Some(p) => Ok(Some(AgentHeadRow {
                    retired: true,
                    applied_pos: ev.global_pos,
                    ..p.clone()
                })),
                None => Err("agent.retired before any version".into()),
            }
        }
        _ => Ok(None),
    }
}
