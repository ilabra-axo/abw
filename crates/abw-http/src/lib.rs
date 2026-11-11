//! The HTTP surface (target-system-design §13.1; T17; ADR-0008 §9).
//!
//! Handlers read only through [`ReadDb`] (`read_query!`) and would write only through
//! `Ledger::dispatch`; this crate has no sqlx dependency (T1). Every route but `/health`
//! needs a principal from `Authorization: Bearer <secret>`, looked up by the secret's
//! `sha256:` in `crud.api_keys` (a missing, unknown or revoked key is 401). Authorisation
//! is one function per object kind (§13.1); a caller who may not see an object gets the
//! same 404 as for an id that doesn't exist, never 403 ("403 would be an existence oracle").
//!
//! Visibility in M2 (ADR-0008 ambiguity 9): agents tenant-wide; episodes, their artifacts
//! and allowances only to their owner.

use abw_canon::{Sha256Ref, sha256_raw};
use abw_ledger::{Ledger, ReadDb, ReadError, Row, read_query};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde_json::{Value, json};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub ledger: Arc<Ledger>,
    pub db: ReadDb,
}

/// One route of [`router`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteInfo {
    pub method: &'static str,
    pub path: &'static str,
}

#[derive(Debug, Clone, Copy)]
enum Handler {
    Health,
    Agent,
    AgentVersions,
    AgentVersion,
    Episode,
    ToolCalls,
    Artifact,
    Allowance,
}

/// The route table [`router`] is built from (§13.1 "Coverage": the IDOR matrix reads it).
const ROUTES: &[(RouteInfo, Handler)] = &[
    (
        RouteInfo {
            method: "GET",
            path: "/health",
        },
        Handler::Health,
    ),
    (
        RouteInfo {
            method: "GET",
            path: "/api/v1/agents/{agent_id}",
        },
        Handler::Agent,
    ),
    (
        RouteInfo {
            method: "GET",
            path: "/api/v1/agents/{agent_id}/versions",
        },
        Handler::AgentVersions,
    ),
    (
        RouteInfo {
            method: "GET",
            path: "/api/v1/agents/{agent_id}/versions/{version_number}",
        },
        Handler::AgentVersion,
    ),
    (
        RouteInfo {
            method: "GET",
            path: "/api/v1/episodes/{episode_id}",
        },
        Handler::Episode,
    ),
    (
        RouteInfo {
            method: "GET",
            path: "/api/v1/episodes/{episode_id}/tool-calls",
        },
        Handler::ToolCalls,
    ),
    (
        RouteInfo {
            method: "GET",
            path: "/api/v1/episodes/{episode_id}/artifacts/{sha}",
        },
        Handler::Artifact,
    ),
    (
        RouteInfo {
            method: "GET",
            path: "/api/v1/allowances/{owner_id}",
        },
        Handler::Allowance,
    ),
];

pub fn route_table() -> Vec<RouteInfo> {
    ROUTES.iter().map(|(r, _)| *r).collect()
}

pub fn router(state: AppState) -> axum::Router {
    let mut r = axum::Router::new();
    for (info, h) in ROUTES {
        let method = match h {
            Handler::Health => get(health),
            Handler::Agent => get(agent),
            Handler::AgentVersions => get(agent_versions),
            Handler::AgentVersion => get(agent_version),
            Handler::Episode => get(episode),
            Handler::ToolCalls => get(tool_calls),
            Handler::Artifact => get(artifact),
            Handler::Allowance => get(allowance),
        };
        r = r.route(info.path, method);
    }
    r.with_state(state)
}

/// The errors a handler answers with. `NotFound` has one body for every route and cause.
#[derive(Debug)]
enum ApiError {
    Unauthorized,
    NotFound,
    Internal,
}

impl From<ReadError> for ApiError {
    fn from(_: ReadError) -> ApiError {
        ApiError::Internal
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        };
        (status, Json(json!({"error": code}))).into_response()
    }
}

/// Who is calling: the API key's tenant, owner and actor.
#[derive(Debug, Clone)]
struct Principal {
    tenant: String,
    owner_id: String,
    #[allow(dead_code)] // carried for commands, which arrive with write routes
    actor_id: String,
}

async fn principal(db: &ReadDb, headers: &HeaderMap) -> Result<Principal, ApiError> {
    let value = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;
    let (scheme, secret) = value.split_once(' ').ok_or(ApiError::Unauthorized)?;
    let secret = secret.trim();
    if !scheme.eq_ignore_ascii_case("bearer") || secret.is_empty() {
        return Err(ApiError::Unauthorized);
    }
    let key_sha = sha256_raw(secret.as_bytes());
    let row = db
        .fetch_optional(
            read_query!(
                "SELECT tenant_id, owner_id, actor_id FROM crud.api_keys
                  WHERE key_sha = $1 AND revoked_at IS NULL"
            )
            .bind(key_sha.to_string()),
        )
        .await?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Principal {
        tenant: row.try_get("tenant_id").map_err(db_err)?,
        owner_id: row.try_get("owner_id").map_err(db_err)?,
        actor_id: row.try_get("actor_id").map_err(db_err)?,
    })
}

fn db_err<E>(_: E) -> ApiError {
    ApiError::Internal
}

// ---- authorisation, one function per object kind (§13.1) -------------------------------

/// An agent is visible to its tenant.
async fn authorize_agent(db: &ReadDb, p: &Principal, agent_id: &str) -> Result<Value, ApiError> {
    let row = db
        .fetch_optional(
            read_query!(
                "SELECT agent_id, version_number, card_sha, retired FROM proj.agent_heads
                  WHERE agent_id = $1 AND tenant_id = $2"
            )
            .bind(agent_id.to_string())
            .bind(p.tenant.clone()),
        )
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(json!({
        "agent_id": row.try_get::<String, _>("agent_id").map_err(db_err)?,
        "version_number": row.try_get::<i32, _>("version_number").map_err(db_err)?,
        "card_sha": row.try_get::<String, _>("card_sha").map_err(db_err)?,
        "retired": row.try_get::<bool, _>("retired").map_err(db_err)?,
    }))
}

/// An episode is visible to its owner in its tenant. Returns its row as JSON and stream.
async fn authorize_episode(
    db: &ReadDb,
    p: &Principal,
    episode_id: &str,
) -> Result<(Value, String), ApiError> {
    let id = Uuid::parse_str(episode_id).map_err(|_| ApiError::NotFound)?;
    let row = db
        .fetch_optional(
            read_query!(
                "SELECT episode_id, stream_id, agent_id, status, reason_class, hold_cu, consumed_cu
                   FROM proj.episodes
                  WHERE episode_id = $1 AND tenant_id = $2 AND owner_id = $3"
            )
            .bind(id)
            .bind(p.tenant.clone())
            .bind(p.owner_id.clone()),
        )
        .await?
        .ok_or(ApiError::NotFound)?;
    let body = json!({
        "episode_id": row.try_get::<Uuid, _>("episode_id").map_err(db_err)?,
        "agent_id": row.try_get::<String, _>("agent_id").map_err(db_err)?,
        "status": row.try_get::<String, _>("status").map_err(db_err)?,
        "reason_class": row.try_get::<Option<String>, _>("reason_class").map_err(db_err)?,
        "hold_cu": row.try_get::<i64, _>("hold_cu").map_err(db_err)?,
        "consumed_cu": row.try_get::<Option<i64>, _>("consumed_cu").map_err(db_err)?,
    });
    Ok((body, row.try_get("stream_id").map_err(db_err)?))
}

/// An allowance is visible to its owner in its tenant.
async fn authorize_allowance(
    db: &ReadDb,
    p: &Principal,
    owner_id: &str,
) -> Result<Value, ApiError> {
    if owner_id != p.owner_id {
        return Err(ApiError::NotFound);
    }
    let row = db
        .fetch_optional(
            read_query!(
                "SELECT owner_id, free_tier, granted, purchased, held, available
                   FROM proj.allowances WHERE owner_id = $1 AND tenant_id = $2"
            )
            .bind(owner_id.to_string())
            .bind(p.tenant.clone()),
        )
        .await?
        .ok_or(ApiError::NotFound)?;
    let n = |c: &str| row.try_get::<i64, _>(c).map_err(db_err);
    Ok(json!({
        "owner_id": row.try_get::<String, _>("owner_id").map_err(db_err)?,
        "free_tier": n("free_tier")?,
        "granted": n("granted")?,
        "purchased": n("purchased")?,
        "held": n("held")?,
        "available": n("available")?,
    }))
}

// ---- handlers ---------------------------------------------------------------------------

async fn health() -> &'static str {
    "ok"
}

async fn agent(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let p = principal(&s.db, &headers).await?;
    Ok(Json(authorize_agent(&s.db, &p, &agent_id).await?))
}

async fn agent_versions(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(agent_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let p = principal(&s.db, &headers).await?;
    authorize_agent(&s.db, &p, &agent_id).await?;
    let rows =
        s.db.fetch_all(
            read_query!(
                "SELECT version_number, card_sha, reason FROM proj.agent_versions
                  WHERE agent_id = $1 AND tenant_id = $2 ORDER BY version_number"
            )
            .bind(agent_id)
            .bind(p.tenant),
        )
        .await?;
    let mut versions = Vec::with_capacity(rows.len());
    for r in rows {
        versions.push(json!({
            "version_number": r.try_get::<i32, _>("version_number").map_err(db_err)?,
            "card_sha": r.try_get::<String, _>("card_sha").map_err(db_err)?,
            "reason": r.try_get::<String, _>("reason").map_err(db_err)?,
        }));
    }
    Ok(Json(json!({"versions": versions})))
}

async fn agent_version(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((agent_id, version)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let p = principal(&s.db, &headers).await?;
    authorize_agent(&s.db, &p, &agent_id).await?;
    let version: i32 = version.parse().map_err(|_| ApiError::NotFound)?;
    let row =
        s.db.fetch_optional(
            read_query!(
                "SELECT card_sha FROM proj.agent_versions
                  WHERE agent_id = $1 AND tenant_id = $2 AND version_number = $3"
            )
            .bind(agent_id.clone())
            .bind(p.tenant)
            .bind(version),
        )
        .await?
        .ok_or(ApiError::NotFound)?;
    let sha: String = row.try_get("card_sha").map_err(db_err)?;
    let sha = Sha256Ref::parse(&sha).map_err(db_err)?;
    let bytes = s.db.resolve(&sha).await?.ok_or(ApiError::Internal)?;
    let card: Value = serde_json::from_slice(&bytes).map_err(db_err)?;
    Ok(Json(json!({
        "agent_id": agent_id,
        "version_number": version,
        "card_sha": sha.as_str(),
        "card": card,
    })))
}

async fn episode(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(episode_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let p = principal(&s.db, &headers).await?;
    Ok(Json(authorize_episode(&s.db, &p, &episode_id).await?.0))
}

async fn tool_calls(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(episode_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let p = principal(&s.db, &headers).await?;
    let (_, stream) = authorize_episode(&s.db, &p, &episode_id).await?;
    let rows =
        s.db.fetch_all(
            read_query!(
                "SELECT seq, payload FROM ledger.events
                  WHERE stream_id = $1 AND event_type = 'tool.invoked' ORDER BY seq"
            )
            .bind(stream),
        )
        .await?;
    let mut calls = Vec::with_capacity(rows.len());
    for r in rows {
        let payload: Value = r.try_get("payload").map_err(db_err)?;
        calls.push(json!({
            "seq": r.try_get::<i64, _>("seq").map_err(db_err)?,
            "tool_name": payload.get("tool_name"),
            "allowed": payload.get("allowed"),
            "input_sha": payload.get("input_sha"),
            "output_sha": payload.get("output_sha"),
        }));
    }
    Ok(Json(json!({"tool_calls": calls})))
}

/// An artifact is served only through an episode that cites it (§13.1): its card, its
/// input, its raw output or a tool call's input or output.
async fn artifact(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path((episode_id, sha)): Path<(String, String)>,
) -> Result<Response, ApiError> {
    let p = principal(&s.db, &headers).await?;
    let (_, stream) = authorize_episode(&s.db, &p, &episode_id).await?;
    let sha = Sha256Ref::parse(&sha).map_err(|_| ApiError::NotFound)?;
    let cited =
        s.db.fetch_optional(
            read_query!(
                "SELECT a.media_type FROM ledger.artifacts a
                  WHERE a.sha = $2 AND EXISTS (
                    SELECT 1 FROM ledger.events e
                     WHERE e.stream_id = $1
                       AND $2 IN (e.payload->>'card_sha', e.payload->>'input_sha',
                                  e.payload->>'output_sha', e.payload->>'raw_output_sha'))"
            )
            .bind(stream)
            .bind(sha.to_string()),
        )
        .await?
        .ok_or(ApiError::NotFound)?;
    let media: String = cited.try_get("media_type").map_err(db_err)?;
    let bytes = s.db.resolve(&sha).await?.ok_or(ApiError::NotFound)?;
    let content_type = HeaderValue::from_str(&media)
        .unwrap_or(HeaderValue::from_static("application/octet-stream"));
    Ok(([(header::CONTENT_TYPE, content_type)], bytes).into_response())
}

async fn allowance(
    State(s): State<AppState>,
    headers: HeaderMap,
    Path(owner_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let p = principal(&s.db, &headers).await?;
    Ok(Json(authorize_allowance(&s.db, &p, &owner_id).await?))
}
