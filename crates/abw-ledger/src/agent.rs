//! Agent versions as JCS hashes of resolved cards (target-system-design §4.1-§4.3;
//! ADR-0008 §4 `agent`).
//!
//! An agent is the stream `agent:<agent_id>`. Each `agent.version_created` is one version,
//! numbered by its stream seq, identified by `card_sha = sha256_jcs(card)`, with the card's
//! JCS bytes stored as an artifact in the same transaction. An edit that hashes to the
//! head's card appends nothing (§4.2 "No-op edits"). `agent.retired` is a soft retire:
//! versions stay, and nothing new is accepted for the agent.

use crate::artifact::artifact_json;
use crate::command::{
    BoxFuture, Expected, Heads, NewEvent, Recorded, StreamAppend, sealed::CommandSpec, stream_id,
};
use crate::ctx::CommandCtx;
use crate::error::{DispatchError, ReadError, Rejection};
use crate::fold::{self, StoredEvent};
use crate::policy::{decode, to_value};
use crate::read::ReadDb;
use crate::read_query;
use crate::registry::{AGENT_RETIRED, AGENT_VERSION_CREATED};
use abw_canon::{Sha256Ref, sha256_jcs, sha256_raw};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{PgConnection, Row};

pub(crate) const KIND: &str = "agent";

/// `ledger.artifacts.media_type` of a resolved card (§4.2).
pub const CARD_MEDIA_TYPE: &str = "application/vnd.abw.agent-card+json";

/// Why a version was created (`proj.agent_versions.reason`, §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionReason {
    Seed,
    OwnerEdit,
    AdoptFile,
    Revert,
}

impl VersionReason {
    pub fn as_str(self) -> &'static str {
        match self {
            VersionReason::Seed => "seed",
            VersionReason::OwnerEdit => "owner_edit",
            VersionReason::AdoptFile => "adopt_file",
            VersionReason::Revert => "revert",
        }
    }
}

pub(crate) fn stream_of(agent_id: &str) -> Result<String, Rejection> {
    stream_id(KIND, agent_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct VersionCreatedPayload {
    pub(crate) agent_id: String,
    pub(crate) version_number: u64,
    #[serde(with = "fold::sha")]
    pub(crate) card_sha: Sha256Ref,
    #[serde(with = "fold::sha")]
    pub(crate) prompt_sha: Sha256Ref,
    pub(crate) reason: VersionReason,
    /// The card's `capabilities.tools_allowed` (§4.3), carried so an episode can stamp its
    /// allowlist from the version it cites without reading the card artifact.
    pub(crate) tools_allowed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RetiredPayload {
    pub(crate) agent_id: String,
    pub(crate) reason: String,
}

/// The head of an agent stream, folded from its events.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct State {
    pub(crate) version_number: u64,
    pub(crate) card_sha: Sha256Ref,
    pub(crate) prompt_sha: Sha256Ref,
    pub(crate) tools_allowed: Vec<String>,
    pub(crate) retired: bool,
}

impl State {
    pub(crate) fn fold(events: Option<&Vec<StoredEvent>>) -> Result<Option<State>, DispatchError> {
        let mut s: Option<State> = None;
        for e in events.into_iter().flatten() {
            match e.event_type.as_str() {
                "agent.version_created" => {
                    let v: VersionCreatedPayload = e.decode()?;
                    s = Some(State {
                        version_number: v.version_number,
                        card_sha: v.card_sha,
                        prompt_sha: v.prompt_sha,
                        tools_allowed: v.tools_allowed,
                        retired: false,
                    });
                }
                "agent.retired" => {
                    if let Some(st) = s.as_mut() {
                        st.retired = true;
                    }
                }
                _ => {}
            }
        }
        Ok(s)
    }
}

/// The checks a resolved card must pass (ADR-0008 §4): an object whose `system_prompt` is a
/// string. `capabilities.tools_allowed`, when present, is an array of strings (§4.3).
/// Returns the prompt and the allowlist.
pub(crate) fn card_parts(card: &Value) -> Result<(String, Vec<String>), Rejection> {
    let obj = card
        .as_object()
        .ok_or_else(|| Rejection::InvalidCard("a card is a JSON object".into()))?;
    let prompt = obj
        .get("system_prompt")
        .and_then(Value::as_str)
        .ok_or_else(|| Rejection::InvalidCard("system_prompt must be a string".into()))?;
    let tools = match obj.get("capabilities").and_then(|c| c.get("tools_allowed")) {
        None => Vec::new(),
        Some(Value::Array(items)) => items
            .iter()
            .map(|t| {
                t.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| Rejection::InvalidCard("tools_allowed must list strings".into()))
            })
            .collect::<Result<_, _>>()?,
        Some(_) => {
            return Err(Rejection::InvalidCard(
                "capabilities.tools_allowed must be an array".into(),
            ));
        }
    };
    Ok((prompt.to_string(), tools))
}

fn expected_at(heads: &Heads, stream: &str) -> Expected {
    match heads.head(stream) {
        0 => Expected::NoStream,
        n => Expected::Exact(n),
    }
}

async fn load(
    conn: &mut PgConnection,
    agent_id: &str,
) -> Result<Option<Vec<StoredEvent>>, DispatchError> {
    let stream = stream_of(agent_id).map_err(DispatchError::Rejected)?;
    Ok(fold::load_streams(conn, std::slice::from_ref(&stream))
        .await?
        .remove(&stream))
}

/// Create a version from an already resolved card, or return the head if the card hashes
/// to it. Without [`PutVersion::expected`], the head is read under the stream lock
/// (§4.2 "Without the header").
#[derive(Debug, Clone)]
pub struct PutVersion {
    agent_id: String,
    card: Value,
    reason: VersionReason,
    expected: Option<Expected>,
}

impl PutVersion {
    pub fn new(agent_id: impl Into<String>, card: Value, reason: VersionReason) -> PutVersion {
        PutVersion {
            agent_id: agent_id.into(),
            card,
            reason,
            expected: None,
        }
    }

    pub fn expected(mut self, expected: Expected) -> PutVersion {
        self.expected = Some(expected);
        self
    }
}

/// The result of [`PutVersion`]. `created` is false for a no-op edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Versioned {
    pub agent_id: String,
    pub version_number: u64,
    #[serde(with = "fold::sha")]
    pub card_sha: Sha256Ref,
    #[serde(with = "fold::sha")]
    pub prompt_sha: Sha256Ref,
    pub created: bool,
}

impl CommandSpec for PutVersion {
    const TYPE: &'static str = "agent.put_version";
    type Output = Versioned;
    type Route = ();
    type Facts = Option<Vec<StoredEvent>>;

    fn validate(&self, _ctx: &CommandCtx) -> Result<(), Rejection> {
        stream_of(&self.agent_id)?;
        card_parts(&self.card)?;
        sha256_jcs(&self.card)
            .map(|_| ())
            .map_err(|e| Rejection::InvalidCard(e.to_string()))
    }

    fn idempotency_key(&self) -> Option<&str> {
        None
    }

    fn request(&self) -> Value {
        json!({
            "agent_id": self.agent_id,
            "card": self.card,
            "reason": self.reason.as_str(),
            "expected": self.expected.map(|e| e.head()),
        })
    }

    fn route<'a>(
        &'a self,
        _conn: &'a mut PgConnection,
    ) -> BoxFuture<'a, Result<(), DispatchError>> {
        fold::ready(())
    }

    fn write_set(&self, _route: &()) -> Vec<String> {
        stream_of(&self.agent_id).into_iter().collect()
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
        Box::pin(load(conn, &self.agent_id))
    }

    fn decide(
        &self,
        heads: &Heads,
        facts: &Self::Facts,
        _ctx: &CommandCtx,
    ) -> Result<Vec<StreamAppend>, DispatchError> {
        let stream = stream_of(&self.agent_id).map_err(DispatchError::Rejected)?;
        let head = State::fold(facts.as_ref())?;
        if head.as_ref().is_some_and(|h| h.retired) {
            return Err(DispatchError::Rejected(Rejection::AgentRetired {
                agent_id: self.agent_id.clone(),
            }));
        }
        let (prompt, tools_allowed) = card_parts(&self.card).map_err(DispatchError::Rejected)?;
        let artifact = artifact_json(&self.card, CARD_MEDIA_TYPE)
            .map_err(|e| DispatchError::Rejected(Rejection::InvalidCard(e.to_string())))?;
        if head.as_ref().is_some_and(|h| &h.card_sha == artifact.sha()) {
            return Ok(Vec::new());
        }
        let payload = VersionCreatedPayload {
            agent_id: self.agent_id.clone(),
            version_number: heads.head(&stream) + 1,
            card_sha: artifact.sha().clone(),
            prompt_sha: sha256_raw(prompt.as_bytes()),
            reason: self.reason,
            tools_allowed,
        };
        let mut event = NewEvent::new(&AGENT_VERSION_CREATED, to_value(&payload)?)
            .map_err(DispatchError::Rejected)?;
        event.artifacts.push(artifact);
        Ok(vec![StreamAppend {
            expected: self.expected.unwrap_or_else(|| expected_at(heads, &stream)),
            stream_id: stream,
            kind: KIND,
            events: vec![event],
        }])
    }

    fn output(
        &self,
        recorded: &[Recorded],
        facts: &Self::Facts,
    ) -> Result<Versioned, DispatchError> {
        match recorded {
            [r] => {
                let v: VersionCreatedPayload = decode(&r.payload)?;
                Ok(Versioned {
                    agent_id: v.agent_id,
                    version_number: v.version_number,
                    card_sha: v.card_sha,
                    prompt_sha: v.prompt_sha,
                    created: true,
                })
            }
            [] => match State::fold(facts.as_ref())? {
                Some(h) => Ok(Versioned {
                    agent_id: self.agent_id.clone(),
                    version_number: h.version_number,
                    card_sha: h.card_sha,
                    prompt_sha: h.prompt_sha,
                    created: false,
                }),
                None => Err(DispatchError::Integrity(
                    "agent.put_version appended nothing for a new agent".into(),
                )),
            },
            _ => Err(DispatchError::Integrity(
                "agent.put_version expected at most one recorded event".into(),
            )),
        }
    }
}

/// Soft-retire an agent (§4.2 `agent.retire`).
#[derive(Debug, Clone)]
pub struct Retire {
    agent_id: String,
    reason: String,
}

impl Retire {
    pub fn new(agent_id: impl Into<String>, reason: impl Into<String>) -> Retire {
        Retire {
            agent_id: agent_id.into(),
            reason: reason.into(),
        }
    }
}

/// The result of [`Retire`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retired {
    pub agent_id: String,
    pub seq: u64,
}

impl CommandSpec for Retire {
    const TYPE: &'static str = "agent.retire";
    type Output = Retired;
    type Route = ();
    type Facts = Option<Vec<StoredEvent>>;

    fn validate(&self, _ctx: &CommandCtx) -> Result<(), Rejection> {
        stream_of(&self.agent_id).map(|_| ())
    }

    fn idempotency_key(&self) -> Option<&str> {
        None
    }

    fn request(&self) -> Value {
        json!({"agent_id": self.agent_id, "reason": self.reason})
    }

    fn route<'a>(
        &'a self,
        _conn: &'a mut PgConnection,
    ) -> BoxFuture<'a, Result<(), DispatchError>> {
        fold::ready(())
    }

    fn write_set(&self, _route: &()) -> Vec<String> {
        stream_of(&self.agent_id).into_iter().collect()
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
        Box::pin(load(conn, &self.agent_id))
    }

    fn decide(
        &self,
        heads: &Heads,
        facts: &Self::Facts,
        _ctx: &CommandCtx,
    ) -> Result<Vec<StreamAppend>, DispatchError> {
        let stream = stream_of(&self.agent_id).map_err(DispatchError::Rejected)?;
        match State::fold(facts.as_ref())? {
            None => Err(DispatchError::Rejected(Rejection::AgentNotFound {
                agent_id: self.agent_id.clone(),
            })),
            Some(h) if h.retired => Err(DispatchError::Rejected(Rejection::AgentRetired {
                agent_id: self.agent_id.clone(),
            })),
            Some(_) => {
                let payload = RetiredPayload {
                    agent_id: self.agent_id.clone(),
                    reason: self.reason.clone(),
                };
                let event = NewEvent::new(&AGENT_RETIRED, to_value(&payload)?)
                    .map_err(DispatchError::Rejected)?;
                Ok(vec![StreamAppend {
                    expected: expected_at(heads, &stream),
                    stream_id: stream,
                    kind: KIND,
                    events: vec![event],
                }])
            }
        }
    }

    fn output(
        &self,
        recorded: &[Recorded],
        _facts: &Self::Facts,
    ) -> Result<Retired, DispatchError> {
        match recorded {
            [r] => Ok(Retired {
                agent_id: self.agent_id.clone(),
                seq: r.seq,
            }),
            _ => Err(DispatchError::Integrity(
                "agent.retire expected one recorded event".into(),
            )),
        }
    }
}

/// An agent's current version as `proj.agent_heads` holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentHead {
    pub agent_id: String,
    pub version_number: u64,
    pub card_sha: Sha256Ref,
    pub retired: bool,
}

/// The head of `agent_id` in `tenant`, or `None` if the tenant has no such agent.
pub async fn head(
    read: &ReadDb,
    tenant: &str,
    agent_id: &str,
) -> Result<Option<AgentHead>, ReadError> {
    let row = read
        .fetch_optional(
            read_query!(
                "SELECT agent_id, version_number, card_sha, retired FROM proj.agent_heads
                  WHERE agent_id = $1 AND tenant_id = $2"
            )
            .bind(agent_id.to_string())
            .bind(tenant.to_string()),
        )
        .await?;
    let Some(r) = row else {
        return Ok(None);
    };
    let version: i32 = r.try_get("version_number")?;
    let sha: String = r.try_get("card_sha")?;
    Ok(Some(AgentHead {
        agent_id: r.try_get("agent_id")?,
        version_number: u64::try_from(version)
            .map_err(|e| ReadError::Db(sqlx::Error::Decode(Box::new(e))))?,
        card_sha: Sha256Ref::parse(&sha)
            .map_err(|e| ReadError::Db(sqlx::Error::Decode(Box::new(e))))?,
        retired: r.try_get("retired")?,
    }))
}
