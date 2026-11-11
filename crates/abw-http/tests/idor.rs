//! T17: a read by opaque ID returns 404 unless the caller may see the object
//! (target-system-design §13.1, §15.1 `idor_matrix`; capabilities.md §14 "Isolate tenants";
//! ADR-0008 §9).
//!
//! The matrix is generated from `abw_http::route_table()`, the table the router is built from:
//! every GET route with a `{param}` is requested by a principal of another tenant and must
//! answer 404, byte-identical to the answer for an ID that doesn't exist. There are no
//! exemptions; adding one requires a written reason in `EXEMPT`.

#[path = "../../abw-ledger/tests/common/mod.rs"]
mod common;
#[path = "../../abw-ledger/tests/common/m2.rs"]
mod m2;

use abw_http::{AppState, route_table};
use abw_ledger::episode::{
    Close, CloseOutcome, EpisodeStatus, RecordTool, ToolOutcome, ToolRecord,
};
use abw_ledger::{Actor, Expected, Ledger, SingletonFence};
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use common::TestDb;
use m2::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

/// `(path, reason)`: GET routes with an id that a second principal may read. Empty, and any
/// entry needs a reason a reviewer can check.
const EXEMPT: &[(&str, &str)] = &[];

/// Tenant A's objects and the principals that look at them.
struct Fixture {
    db: TestDb,
    _fence: SingletonFence,
    state: AppState,
    /// Principal of owner-a in tenant A (owns everything below).
    key_a: String,
    /// Another owner in tenant A.
    key_a2: String,
    /// A principal of tenant B.
    key_b: String,
    /// Path parameter name -> tenant A's value.
    ids: BTreeMap<&'static str, String>,
}

async fn issue(ledger: &Ledger, tenant: &str, owner: &str) -> String {
    let key = ok(
        ledger
            .crud()
            .issue_api_key(tenant, owner, &Actor::human(owner), "idor test")
            .await,
        "issue api key",
    );
    assert!(
        key.secret.len() >= 32,
        "an API key needs at least 32 characters"
    );
    key.secret
}

async fn fixture() -> Fixture {
    let db = TestDb::migrated().await;
    let (ledger, read, fence) = db.ledger().await;
    let grant_a = accept_grant(&ledger, TENANT_A, 500, 0, 0).await;
    let price_a = accept_price(&ledger, TENANT_A, 1, 200).await;
    let grant_b = accept_grant(&ledger, TENANT_B, 500, 0, 0).await;
    open_allowance(&ledger, TENANT_A, "owner-a", &grant_a).await;
    open_allowance(&ledger, TENANT_A, "owner-a2", &grant_a).await;
    open_allowance(&ledger, TENANT_B, "owner-b", &grant_b).await;
    let v = put_agent(&ledger, TENANT_A, "agent-a", &["echo"]).await;

    let eid = Uuid::new_v4();
    open_episode(
        &ledger,
        TENANT_A,
        open_request(
            eid,
            "owner-a",
            "agent-a",
            50,
            &price_a,
            &grant_a,
            at(2026, 11, 3, 9, 0),
        ),
    )
    .await;
    let rec = ok(
        ledger
            .dispatch(
                RecordTool::new(
                    eid,
                    Expected::Exact(1),
                    ToolRecord {
                        iteration: 0,
                        call_index: 0,
                        tool_name: "echo".into(),
                        tool_version: "1".into(),
                        allowed: true,
                        input: json!({"q": "secret question"}),
                        output: ToolOutcome::Ok(json!({"echo": "secret answer"})),
                        duration_ms: 1,
                    },
                ),
                &platform(TENANT_A),
            )
            .await,
        "record tool",
    )
    .output;
    ok(
        ledger
            .dispatch(
                Close::new(
                    eid,
                    Expected::Exact(2),
                    CloseOutcome {
                        status: EpisodeStatus::Success,
                        reason_class: None,
                        raw_output: Some("done".into()),
                        usage: usage(10, 0, 1),
                    },
                ),
                &platform(TENANT_A),
            )
            .await,
        "close",
    );

    let key_a = issue(&ledger, TENANT_A, "owner-a").await;
    let key_a2 = issue(&ledger, TENANT_A, "owner-a2").await;
    let key_b = issue(&ledger, TENANT_B, "owner-b").await;
    let ids = BTreeMap::from([
        ("agent_id", "agent-a".to_string()),
        ("version_number", v.version_number.to_string()),
        ("episode_id", eid.to_string()),
        ("sha", rec.output_sha.to_string()),
        ("owner_id", "owner-a".to_string()),
    ]);
    Fixture {
        db,
        _fence: fence,
        state: AppState {
            ledger: Arc::new(ledger),
            db: read,
        },
        key_a,
        key_a2,
        key_b,
        ids,
    }
}

/// `path` with every `{param}` replaced from `ids`; panics on a parameter with no fixture.
fn fill(path: &str, ids: &BTreeMap<&'static str, String>) -> String {
    let mut out = String::new();
    let mut rest = path;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let close = match rest[open..].find('}') {
            Some(c) => open + c,
            None => panic!("unbalanced route {path}"),
        };
        let name = rest[open + 1..close].trim_start_matches('*');
        match ids.get(name) {
            Some(v) => out.push_str(v),
            None => panic!(
                "route {path} has parameter `{name}` with no fixture: add one to the IDOR matrix \
                 (or an EXEMPT entry with a written reason)"
            ),
        }
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
}

/// Every `{param}` replaced by a well-formed value that names nothing.
fn fill_absent(path: &str, ids: &BTreeMap<&'static str, String>) -> String {
    let absent: BTreeMap<&'static str, String> = ids
        .keys()
        .map(|k| {
            let v = match *k {
                "episode_id" => Uuid::new_v4().to_string(),
                "version_number" => "9999".to_string(),
                "sha" => format!("sha256:{}", "0".repeat(64)),
                _ => format!("absent-{}", Uuid::new_v4().simple()),
            };
            (*k, v)
        })
        .collect();
    fill(path, &absent)
}

async fn get(state: &AppState, uri: &str, key: Option<&str>) -> (StatusCode, Vec<u8>) {
    let mut req = Request::builder().method("GET").uri(uri);
    if let Some(k) = key {
        req = req.header("authorization", format!("Bearer {k}"));
    }
    let req = match req.body(Body::empty()) {
        Ok(r) => r,
        Err(e) => panic!("{uri}: {e}"),
    };
    let resp = match abw_http::router(state.clone()).oneshot(req).await {
        Ok(r) => r,
        Err(e) => panic!("{uri}: {e}"),
    };
    let status = resp.status();
    let body = match to_bytes(resp.into_body(), 1 << 20).await {
        Ok(b) => b.to_vec(),
        Err(e) => panic!("{uri}: {e}"),
    };
    (status, body)
}

fn id_routes() -> Vec<&'static str> {
    route_table()
        .into_iter()
        .filter(|r| r.method == "GET" && r.path.contains('{'))
        .map(|r| r.path)
        .collect()
}

fn json_of(body: &[u8]) -> Value {
    match serde_json::from_slice(body) {
        Ok(v) => v,
        Err(e) => panic!("not JSON ({e}): {}", String::from_utf8_lossy(body)),
    }
}

// @req T17, CAP-14.04
#[tokio::test]
async fn every_get_route_with_an_id_returns_404_to_another_tenant() {
    let f = fixture().await;
    let routes = id_routes();
    for want in [
        "/api/v1/agents/{agent_id}",
        "/api/v1/agents/{agent_id}/versions",
        "/api/v1/agents/{agent_id}/versions/{version_number}",
        "/api/v1/episodes/{episode_id}",
        "/api/v1/episodes/{episode_id}/tool-calls",
        "/api/v1/episodes/{episode_id}/artifacts/{sha}",
        "/api/v1/allowances/{owner_id}",
    ] {
        assert!(
            routes.contains(&want),
            "route table lacks {want}: {routes:?}"
        );
    }
    let mut checked = 0;
    for path in routes {
        if let Some((_, reason)) = EXEMPT.iter().find(|(p, _)| *p == path) {
            assert!(
                !reason.trim().is_empty(),
                "{path} is exempt without a reason"
            );
            continue;
        }
        let uri = fill(path, &f.ids);
        let (own, _) = get(&f.state, &uri, Some(&f.key_a)).await;
        assert_eq!(
            own,
            StatusCode::OK,
            "{uri}: the owner must see it, or the matrix proves nothing"
        );
        let (status, body) = get(&f.state, &uri, Some(&f.key_b)).await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "{uri} leaked to another tenant"
        );
        assert_ne!(status, StatusCode::FORBIDDEN);
        // Not even the body may tell "exists, not yours" from "doesn't exist".
        let (absent_status, absent_body) =
            get(&f.state, &fill_absent(path, &f.ids), Some(&f.key_b)).await;
        assert_eq!(absent_status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(body, absent_body, "{uri}: the 404 is an existence oracle");
        checked += 1;
    }
    assert!(checked >= 7);
    f.db.finish().await;
}

// @req T17, CAP-14.04
#[tokio::test]
async fn owner_scoped_objects_return_404_to_another_owner_of_the_same_tenant() {
    let f = fixture().await;
    for path in id_routes() {
        let uri = fill(path, &f.ids);
        let (status, _) = get(&f.state, &uri, Some(&f.key_a2)).await;
        let owner_scoped =
            path.starts_with("/api/v1/episodes/") || path.starts_with("/api/v1/allowances/");
        if owner_scoped {
            assert_eq!(
                status,
                StatusCode::NOT_FOUND,
                "{uri} leaked to another owner"
            );
        } else {
            assert_eq!(
                status,
                StatusCode::OK,
                "{uri}: agents are visible tenant-wide"
            );
        }
    }
    // Each owner reads their own allowance.
    let (status, body) = get(&f.state, "/api/v1/allowances/owner-a2", Some(&f.key_a2)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_of(&body).get("available"), Some(&json!(500)));
    f.db.finish().await;
}

// @req T17, CAP-14.04
#[tokio::test]
async fn reads_return_the_owners_own_objects() {
    let f = fixture().await;
    let (s, body) = get(
        &f.state,
        &fill("/api/v1/agents/{agent_id}", &f.ids),
        Some(&f.key_a),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let agent = json_of(&body);
    assert_eq!(agent.get("agent_id"), Some(&json!("agent-a")));
    assert_eq!(agent.get("version_number"), Some(&json!(1)));
    assert_eq!(agent.get("retired"), Some(&json!(false)));

    let (s, body) = get(
        &f.state,
        &fill("/api/v1/episodes/{episode_id}", &f.ids),
        Some(&f.key_a),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let ep = json_of(&body);
    assert_eq!(ep.get("status"), Some(&json!("success")));
    assert_eq!(ep.get("consumed_cu"), Some(&json!(10)));

    let (s, body) = get(
        &f.state,
        &fill("/api/v1/episodes/{episode_id}/tool-calls", &f.ids),
        Some(&f.key_a),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let calls = json_of(&body);
    assert_eq!(
        calls.pointer("/tool_calls/0/output_sha"),
        Some(&json!(f.ids["sha"]))
    );

    let (s, bytes) = get(
        &f.state,
        &fill("/api/v1/episodes/{episode_id}/artifacts/{sha}", &f.ids),
        Some(&f.key_a),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(abw_canon::sha256_raw(&bytes).to_string(), f.ids["sha"]);

    let (s, body) = get(&f.state, "/api/v1/allowances/owner-a", Some(&f.key_a)).await;
    assert_eq!(s, StatusCode::OK);
    let al = json_of(&body);
    for (k, v) in [
        ("granted", 490),
        ("held", 0),
        ("available", 490),
        ("purchased", 0),
    ] {
        assert_eq!(al.get(k), Some(&json!(v)), "{k}");
    }
    f.db.finish().await;
}

// @req T17, CAP-14.04
#[tokio::test]
async fn an_artifact_is_readable_only_through_an_episode_that_cites_it() {
    let f = fixture().await;
    // A real artifact of the same owner's tenant, but not one this episode cites.
    let other = put_agent(&f.state.ledger, TENANT_A, "agent-other", &[]).await;
    let card_sha = other.card_sha.to_string();
    let uri = format!(
        "/api/v1/episodes/{}/artifacts/{card_sha}",
        f.ids["episode_id"]
    );
    let (status, _) = get(&f.state, &uri, Some(&f.key_a)).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "an uncited artifact was served"
    );
    f.db.finish().await;
}

// @req T17, CAP-14.04
#[tokio::test]
async fn a_missing_unknown_or_revoked_key_is_401() {
    let f = fixture().await;
    let uri = fill("/api/v1/agents/{agent_id}", &f.ids);
    assert_eq!(get(&f.state, &uri, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        get(
            &f.state,
            &uri,
            Some("abw_not_a_real_key_000000000000000000")
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let key = ok(
        f.state
            .ledger
            .crud()
            .issue_api_key(TENANT_A, "owner-a", &Actor::human("owner-a"), "to revoke")
            .await,
        "issue",
    );
    assert_eq!(
        get(&f.state, &uri, Some(&key.secret)).await.0,
        StatusCode::OK
    );
    ok(
        f.state.ledger.crud().revoke_api_key(key.key_id).await,
        "revoke",
    );
    assert_eq!(
        get(&f.state, &uri, Some(&key.secret)).await.0,
        StatusCode::UNAUTHORIZED
    );
    // /health is the one route without a principal.
    assert_eq!(get(&f.state, "/health", None).await.0, StatusCode::OK);
    f.db.finish().await;
}

// @req T17, CAP-14.04
#[tokio::test]
async fn api_keys_are_stored_only_as_hashes() {
    let f = fixture().await;
    let key = ok(
        f.state
            .ledger
            .crud()
            .issue_api_key(TENANT_A, "owner-a", &Actor::human("owner-a"), "hash check")
            .await,
        "issue",
    );
    let want = abw_canon::sha256_raw(key.secret.as_bytes()).to_string();
    let stored = match sqlx::query_scalar::<_, String>(
        "SELECT key_sha FROM crud.api_keys WHERE key_id = $1",
    )
    .bind(key.key_id)
    .fetch_one(&f.db.admin)
    .await
    {
        Ok(s) => s,
        Err(e) => panic!("{e}"),
    };
    assert_eq!(stored, want);
    let leaks = match sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM crud.api_keys k WHERE strpos(row_to_json(k)::text, $1) > 0",
    )
    .bind(&key.secret)
    .fetch_one(&f.db.admin)
    .await
    {
        Ok(n) => n,
        Err(e) => panic!("{e}"),
    };
    assert_eq!(leaks, 0, "the plaintext key is stored");
    f.db.finish().await;
}
