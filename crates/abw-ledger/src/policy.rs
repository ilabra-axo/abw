//! Grant and price policies as human-accepted policy versions (ADR-0008 §4 `policy`;
//! target-system-design §3.1.7, §11.2, §11.4; economy.md §2 "Grant policy", "Price policy").
//!
//! A policy is a stream `policy:<policy_id>`; each `policy.adjusted` event is one version
//! (its stream seq) and writes its `ledger.policy_versions` row in the same transaction.
//! Resolution (§11.2): the tenant's policy of a kind if the tenant has one, else the
//! platform default; within it, the latest accepted version with `active_from <= at`.

use crate::command::{
    BoxFuture, Expected, Heads, NewEvent, Recorded, StreamAppend, sealed::CommandSpec, stream_id,
};
use crate::ctx::{ActorKind, CommandCtx, PLATFORM_TENANT};
use crate::episode::Usage;
use crate::error::{DispatchError, ReadError, Rejection};
use crate::fold::{self, StoredEvent};
use crate::read::ReadDb;
use crate::read_query;
use crate::registry::POLICY_ADJUSTED;
use abw_canon::{Sha256Ref, sha256_jcs};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::types::Json;
use sqlx::{PgConnection, Row};
use std::fmt;
use uuid::Uuid;

pub(crate) const KIND: &str = "policy";

/// The policy kinds M2 adds to `ledger.policy_versions.policy_kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyKind {
    Grant,
    Price,
}

impl PolicyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PolicyKind::Grant => "grant",
            PolicyKind::Price => "price",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<PolicyKind> {
        match s {
            "grant" => Some(PolicyKind::Grant),
            "price" => Some(PolicyKind::Price),
            _ => None,
        }
    }
}

/// A citation of one policy version. Displayed as `<policy_id>@<version>`, the form
/// `grant_policy_version` and `price_version` carry in payloads (ADR-0008 §2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyRef {
    pub policy_id: Uuid,
    pub version: u32,
    #[serde(with = "fold::sha")]
    pub doc_sha: Sha256Ref,
}

impl fmt::Display for PolicyRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.policy_id.hyphenated(), self.version)
    }
}

impl PolicyRef {
    pub(crate) fn stream_id(&self) -> String {
        format!("{KIND}:{}", self.policy_id.hyphenated())
    }
}

/// The parsed body of a grant policy (economy.md §3 "Grant kinds").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrantPolicy {
    pub onboarding_cu: u64,
    pub free_tier_cap_cu: u64,
    pub operator_limit_cu: u64,
}

/// The parsed body of a price policy (economy.md §2 "Price policy").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PricePolicy {
    pub cu_per_input_token: u64,
    pub cu_per_output_token: u64,
    pub cu_per_tool_call: u64,
    pub hold_gas_cu: u64,
    pub hold_cap_cu: u64,
}

impl PricePolicy {
    /// The CU charge for `usage` (saturating; ADR-0008 §4).
    pub fn price(&self, usage: &Usage) -> u64 {
        usage
            .input_tokens
            .saturating_mul(self.cu_per_input_token)
            .saturating_add(usage.output_tokens.saturating_mul(self.cu_per_output_token))
            .saturating_add(usage.tool_calls.saturating_mul(self.cu_per_tool_call))
    }
}

const GRANT_KEYS: &[&str] = &[
    "policy_kind",
    "onboarding_cu",
    "free_tier_cap_cu",
    "operator_limit_cu",
];
const PRICE_KEYS: &[&str] = &[
    "policy_kind",
    "cu_per_input_token",
    "cu_per_output_token",
    "cu_per_tool_call",
    "hold_gas_cu",
    "hold_cap_cu",
];

/// Exactly the keys of `kind`, every amount a non-negative integer, `policy_kind` equal to
/// `kind` (ADR-0008 §4 "Documents").
fn fields(kind: PolicyKind, doc: &Value) -> Result<Vec<u64>, Rejection> {
    let keys = match kind {
        PolicyKind::Grant => GRANT_KEYS,
        PolicyKind::Price => PRICE_KEYS,
    };
    let obj = doc
        .as_object()
        .ok_or_else(|| Rejection::PolicyInvalid("the document is not an object".into()))?;
    if let Some(k) = obj.keys().find(|k| !keys.contains(&k.as_str())) {
        return Err(Rejection::PolicyInvalid(format!(
            "unknown key {k:?} in a {} policy",
            kind.as_str()
        )));
    }
    if obj.get("policy_kind").and_then(Value::as_str) != Some(kind.as_str()) {
        return Err(Rejection::PolicyInvalid(format!(
            "policy_kind must be {:?}",
            kind.as_str()
        )));
    }
    keys[1..]
        .iter()
        .map(|k| match obj.get(*k) {
            Some(v) => v.as_u64().ok_or_else(|| {
                Rejection::PolicyInvalid(format!("{k} must be a non-negative integer"))
            }),
            None => Err(Rejection::PolicyInvalid(format!("missing {k}"))),
        })
        .collect()
}

pub(crate) fn parse_grant(doc: &Value) -> Result<GrantPolicy, Rejection> {
    match fields(PolicyKind::Grant, doc)?.as_slice() {
        [onboarding_cu, free_tier_cap_cu, operator_limit_cu] => Ok(GrantPolicy {
            onboarding_cu: *onboarding_cu,
            free_tier_cap_cu: *free_tier_cap_cu,
            operator_limit_cu: *operator_limit_cu,
        }),
        _ => Err(Rejection::PolicyInvalid("malformed grant policy".into())),
    }
}

pub(crate) fn parse_price(doc: &Value) -> Result<PricePolicy, Rejection> {
    match fields(PolicyKind::Price, doc)?.as_slice() {
        [cu_in, cu_out, cu_call, gas, cap] => Ok(PricePolicy {
            cu_per_input_token: *cu_in,
            cu_per_output_token: *cu_out,
            cu_per_tool_call: *cu_call,
            hold_gas_cu: *gas,
            hold_cap_cu: *cap,
        }),
        _ => Err(Rejection::PolicyInvalid("malformed price policy".into())),
    }
}

/// The `policy.adjusted` payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Adjusted {
    pub(crate) policy_id: Uuid,
    pub(crate) version: u32,
    pub(crate) policy_kind: PolicyKind,
    #[serde(with = "fold::sha")]
    pub(crate) doc_sha: Sha256Ref,
    pub(crate) doc: Value,
    pub(crate) accepted_by: String,
    pub(crate) rationale: String,
    pub(crate) active_from: DateTime<Utc>,
}

/// Accept a new version of a policy. Human actors only, with a rationale (§11.4).
#[derive(Debug, Clone)]
pub struct Accept {
    policy_id: Uuid,
    kind: PolicyKind,
    doc: Value,
    rationale: String,
    active_from: DateTime<Utc>,
    expected: Expected,
}

impl Accept {
    /// Version 1 of a new policy unless [`Accept::expected`] says otherwise.
    pub fn new(
        policy_id: Uuid,
        kind: PolicyKind,
        doc: Value,
        rationale: impl Into<String>,
        active_from: DateTime<Utc>,
    ) -> Accept {
        Accept {
            policy_id,
            kind,
            doc,
            rationale: rationale.into(),
            active_from: fold::micros(active_from),
            expected: Expected::NoStream,
        }
    }

    pub fn expected(mut self, expected: Expected) -> Accept {
        self.expected = expected;
        self
    }

    fn stream_id(&self) -> String {
        format!("{KIND}:{}", self.policy_id.hyphenated())
    }
}

impl CommandSpec for Accept {
    const TYPE: &'static str = "policy.accept";
    type Output = PolicyRef;
    type Route = ();
    type Facts = Vec<StoredEvent>;

    fn validate(&self, ctx: &CommandCtx) -> Result<(), Rejection> {
        if ctx.actor.kind != ActorKind::Human {
            return Err(Rejection::NotHuman {
                actor_kind: ctx.actor.kind,
            });
        }
        if self.rationale.trim().is_empty() {
            return Err(Rejection::ReasonRequired);
        }
        match self.kind {
            PolicyKind::Grant => parse_grant(&self.doc).map(|_| ()),
            PolicyKind::Price => parse_price(&self.doc).map(|_| ()),
        }?;
        sha256_jcs(&self.doc)
            .map(|_| ())
            .map_err(|e| Rejection::PolicyInvalid(e.to_string()))
    }

    fn idempotency_key(&self) -> Option<&str> {
        None
    }

    fn request(&self) -> Value {
        json!({
            "policy_id": self.policy_id,
            "kind": self.kind.as_str(),
            "doc": self.doc,
            "rationale": self.rationale,
            "active_from": fold::rfc3339(self.active_from),
            "expected": self.expected.head(),
        })
    }

    fn route<'a>(
        &'a self,
        _conn: &'a mut PgConnection,
    ) -> BoxFuture<'a, Result<(), DispatchError>> {
        fold::ready(())
    }

    fn write_set(&self, _route: &()) -> Vec<String> {
        vec![self.stream_id()]
    }

    fn read_set(&self, _route: &()) -> Vec<String> {
        Vec::new()
    }

    fn fold<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        _route: &'a (),
        _ctx: &'a CommandCtx,
    ) -> BoxFuture<'a, Result<Vec<StoredEvent>, DispatchError>> {
        Box::pin(async move {
            let id = self.stream_id();
            Ok(fold::load_streams(conn, std::slice::from_ref(&id))
                .await?
                .remove(&id)
                .unwrap_or_default())
        })
    }

    fn decide(
        &self,
        heads: &Heads,
        facts: &Vec<StoredEvent>,
        ctx: &CommandCtx,
    ) -> Result<Vec<StreamAppend>, DispatchError> {
        let stream_id = self.stream_id();
        // A policy keeps its kind for life.
        if let Some(first) = facts.first() {
            let prior: Adjusted = first.decode()?;
            if prior.policy_kind != self.kind {
                return Err(DispatchError::Rejected(Rejection::PolicyKindMismatch {
                    expected: prior.policy_kind,
                    found: self.kind,
                }));
            }
        }
        let version = u32::try_from(heads.head(&stream_id) + 1)
            .map_err(|_| DispatchError::Integrity("policy version out of range".into()))?;
        let doc_sha = sha256_jcs(&self.doc)
            .map_err(|e| DispatchError::Rejected(Rejection::PolicyInvalid(e.to_string())))?;
        let payload = Adjusted {
            policy_id: self.policy_id,
            version,
            policy_kind: self.kind,
            doc_sha,
            doc: self.doc.clone(),
            accepted_by: ctx.actor.id.clone(),
            rationale: self.rationale.clone(),
            active_from: self.active_from,
        };
        let event = NewEvent::new(&POLICY_ADJUSTED, to_value(&payload)?)
            .map_err(DispatchError::Rejected)?;
        Ok(vec![StreamAppend {
            stream_id,
            kind: KIND,
            expected: self.expected,
            events: vec![event],
        }])
    }

    fn output(
        &self,
        recorded: &[Recorded],
        _facts: &Vec<StoredEvent>,
    ) -> Result<PolicyRef, DispatchError> {
        match recorded {
            [r] => {
                let a: Adjusted = decode(&r.payload)?;
                Ok(PolicyRef {
                    policy_id: a.policy_id,
                    version: a.version,
                    doc_sha: a.doc_sha,
                })
            }
            _ => Err(DispatchError::Integrity(
                "policy.accept expected one recorded event".into(),
            )),
        }
    }
}

pub(crate) fn to_value<T: Serialize>(v: &T) -> Result<Value, DispatchError> {
    serde_json::to_value(v).map_err(|e| DispatchError::Integrity(format!("payload: {e}")))
}

pub(crate) fn decode<T: serde::de::DeserializeOwned>(v: &Value) -> Result<T, DispatchError> {
    serde_json::from_value(v.clone()).map_err(|e| DispatchError::Integrity(format!("payload: {e}")))
}

/// The tenant column of a policy accepted under `tenant`: NULL for the platform default.
pub(crate) fn stored_tenant(tenant: &str) -> Option<&str> {
    (tenant != PLATFORM_TENANT).then_some(tenant)
}

/// The active policy of `kind` for `tenant` at `at` (§11.2), or `None`.
pub async fn active(
    read: &ReadDb,
    kind: PolicyKind,
    tenant: &str,
    at: DateTime<Utc>,
) -> Result<Option<PolicyRef>, ReadError> {
    let row = read
        .fetch_optional(
            read_query!(
                "SELECT policy_id, version, doc_sha FROM ledger.policy_versions
                  WHERE policy_kind = $1 AND active_from <= $3
                    AND tenant_id IS NOT DISTINCT FROM (
                      CASE WHEN EXISTS (SELECT 1 FROM ledger.policy_versions
                                         WHERE policy_kind = $1 AND tenant_id = $2)
                           THEN $2 ELSE NULL END)
                  ORDER BY event_pos DESC LIMIT 1"
            )
            .bind(kind.as_str().to_string())
            .bind(tenant.to_string())
            .bind(at),
        )
        .await?;
    row.map(|r| policy_ref(&r).map_err(ReadError::Db))
        .transpose()
}

fn policy_ref(r: &sqlx::postgres::PgRow) -> Result<PolicyRef, sqlx::Error> {
    let version: i32 = r.try_get("version")?;
    let sha: String = r.try_get("doc_sha")?;
    Ok(PolicyRef {
        policy_id: r.try_get("policy_id")?,
        version: u32::try_from(version).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        doc_sha: Sha256Ref::parse(&sha).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
    })
}

/// The price policy `r` cites, if it exists, is a price policy and has that `doc_sha`.
pub async fn load_price(read: &ReadDb, r: &PolicyRef) -> Result<Option<PricePolicy>, ReadError> {
    let Ok(version) = i32::try_from(r.version) else {
        return Ok(None);
    };
    let row = read
        .fetch_optional(
            read_query!(
                "SELECT policy_kind, doc, doc_sha FROM ledger.policy_versions
                  WHERE policy_id = $1 AND version = $2"
            )
            .bind(r.policy_id)
            .bind(version),
        )
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let kind: String = row.try_get("policy_kind")?;
    let sha: String = row.try_get("doc_sha")?;
    let doc: Json<Value> = row.try_get("doc")?;
    if kind != PolicyKind::Price.as_str() || sha != r.doc_sha.as_str() {
        return Ok(None);
    }
    Ok(parse_price(&doc.0).ok())
}

/// A cited policy as stored, read in the append transaction.
#[derive(Debug, Clone)]
pub(crate) struct Cited {
    row: Option<StoredPolicy>,
    active: Option<(Uuid, u32)>,
}

#[derive(Debug, Clone)]
struct StoredPolicy {
    kind: String,
    tenant_id: Option<String>,
    doc: Value,
    doc_sha: String,
}

/// Reads what [`check`] needs about `r`: its row and the active version of `kind` for
/// `tenant` at `at`, with the same resolution as [`active`].
pub(crate) async fn cite(
    conn: &mut PgConnection,
    r: &PolicyRef,
    kind: PolicyKind,
    tenant: &str,
    at: DateTime<Utc>,
) -> Result<Cited, DispatchError> {
    // A version beyond i32 can't be stored, so it cites nothing.
    let version = i32::try_from(r.version).unwrap_or(-1);
    let row = sqlx::query(
        "SELECT policy_kind, tenant_id, doc, doc_sha FROM ledger.policy_versions
          WHERE policy_id = $1 AND version = $2",
    )
    .bind(r.policy_id)
    .bind(version)
    .fetch_optional(&mut *conn)
    .await?;
    let row = match row {
        Some(x) => Some(StoredPolicy {
            kind: x.try_get("policy_kind")?,
            tenant_id: x.try_get("tenant_id")?,
            doc: x.try_get::<Json<Value>, _>("doc")?.0,
            doc_sha: x.try_get("doc_sha")?,
        }),
        None => None,
    };
    let active = sqlx::query(
        "SELECT policy_id, version FROM ledger.policy_versions
          WHERE policy_kind = $1 AND active_from <= $3
            AND tenant_id IS NOT DISTINCT FROM (
              CASE WHEN EXISTS (SELECT 1 FROM ledger.policy_versions
                                 WHERE policy_kind = $1 AND tenant_id = $2)
                   THEN $2 ELSE NULL END)
          ORDER BY event_pos DESC LIMIT 1",
    )
    .bind(kind.as_str())
    .bind(tenant)
    .bind(at)
    .fetch_optional(&mut *conn)
    .await?;
    let active = match active {
        Some(a) => {
            let v: i32 = a.try_get("version")?;
            let v = u32::try_from(v)
                .map_err(|_| DispatchError::Integrity(format!("policy version {v}")))?;
            Some((a.try_get("policy_id")?, v))
        }
        None => None,
    };
    Ok(Cited { row, active })
}

/// The refusals of ADR-0008 §4 "Resolution", in order, returning the cited document.
fn check<'c>(
    r: &PolicyRef,
    kind: PolicyKind,
    cited: &'c Cited,
    tenant: &str,
) -> Result<&'c Value, Rejection> {
    let not_found = || Rejection::PolicyNotFound {
        policy: r.to_string(),
    };
    let row = cited.row.as_ref().ok_or_else(not_found)?;
    if row.doc_sha != r.doc_sha.as_str() {
        return Err(not_found());
    }
    if row.tenant_id.as_deref().is_some_and(|t| t != tenant) {
        return Err(not_found());
    }
    let found = PolicyKind::parse(&row.kind).ok_or_else(not_found)?;
    if found != kind {
        return Err(Rejection::PolicyKindMismatch {
            expected: kind,
            found,
        });
    }
    if cited.active != Some((r.policy_id, r.version)) {
        return Err(Rejection::PolicyNotActive {
            policy: r.to_string(),
        });
    }
    Ok(&row.doc)
}

pub(crate) fn check_grant(
    r: &PolicyRef,
    cited: &Cited,
    tenant: &str,
) -> Result<GrantPolicy, Rejection> {
    parse_grant(check(r, PolicyKind::Grant, cited, tenant)?)
}

pub(crate) fn check_price(
    r: &PolicyRef,
    cited: &Cited,
    tenant: &str,
) -> Result<PricePolicy, Rejection> {
    parse_price(check(r, PolicyKind::Price, cited, tenant)?)
}

/// Parses `<policy_id>@<version>`.
pub(crate) fn parse_version_ref(s: &str) -> Option<(Uuid, u32)> {
    let (id, v) = s.split_once('@')?;
    Some((Uuid::parse_str(id).ok()?, v.parse().ok()?))
}

/// The price document of a version cited by an open episode, read in the append
/// transaction. Its kind and sha were checked when the episode opened.
pub(crate) async fn price_at(
    conn: &mut PgConnection,
    id: Uuid,
    version: u32,
) -> Result<Option<PricePolicy>, DispatchError> {
    let doc: Option<Json<Value>> = sqlx::query_scalar(
        "SELECT doc FROM ledger.policy_versions
          WHERE policy_id = $1 AND version = $2 AND policy_kind = 'price'",
    )
    .bind(id)
    .bind(i32::try_from(version).unwrap_or(-1))
    .fetch_optional(&mut *conn)
    .await?;
    doc.map(|d| parse_price(&d.0).map_err(DispatchError::Rejected))
        .transpose()
}

/// The stream id `ref` cites, for read sets.
pub(crate) fn ref_stream(r: &PolicyRef) -> Result<String, Rejection> {
    stream_id(KIND, &r.policy_id.hyphenated().to_string())
}
