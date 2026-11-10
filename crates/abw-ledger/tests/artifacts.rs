//! T2: artifacts are content-addressed. Appendix D cases 14, 15 and 28.

mod common;

use abw_canon::{sha256_jcs, sha256_raw};
use abw_ledger::probe::Append;
use abw_ledger::{Canon, Expected, artifact_bytes, artifact_json};
use common::{TestDb, assert_rejected, ctx};
use serde_json::json;
use sqlx::Row;

const V4_SHA: &str = "sha256:0d4251ddf30651714bd5d954aac1e8545862c1311d4b2858aa8bb49c469c712a";

// @req T2
#[tokio::test]
async fn artifact_round_trip_is_byte_identical() {
    let db = TestDb::migrated().await;
    let (ledger, read, _fence) = db.ledger().await;
    let bytes: Vec<u8> = (0u8..=255).cycle().take(4096).collect();
    let art = artifact_bytes(bytes.clone(), Canon::RawBytes, "application/octet-stream");
    let sha = art.sha().clone();
    assert_eq!(sha, sha256_raw(&bytes));
    if let Err(e) = ledger
        .dispatch(
            Append::new("a1", Expected::NoStream, json!({"cites": sha.as_str()})).artifact(art),
            &ctx(),
        )
        .await
    {
        panic!("{e:?}");
    }
    match read.resolve(&sha).await {
        Ok(Some(back)) => assert_eq!(back, bytes),
        other => panic!("resolve: {other:?}"),
    }
    match read.resolve(&sha256_raw(b"never stored")).await {
        Ok(None) => {}
        other => panic!("an unknown sha must resolve to None: {other:?}"),
    }
    db.finish().await;
}

// @req T2, T4
#[tokio::test]
async fn json_artifacts_are_stored_canonically() {
    let db = TestDb::migrated().await;
    let (ledger, read, _fence) = db.ledger().await;
    let doc = json!({"b": [1.0, 2], "a": "\u{e9}"});
    let art = match artifact_json(&doc, "application/json") {
        Ok(a) => a,
        Err(e) => panic!("{e:?}"),
    };
    let sha = art.sha().clone();
    assert_eq!(Some(sha.clone()), sha256_jcs(&doc).ok());
    if let Err(e) = ledger
        .dispatch(
            Append::new("a2", Expected::NoStream, json!({})).artifact(art),
            &ctx(),
        )
        .await
    {
        panic!("{e:?}");
    }
    match read.resolve(&sha).await {
        Ok(Some(back)) => assert_eq!(back, "{\"a\":\"\u{e9}\",\"b\":[1,2]}".as_bytes()),
        other => panic!("resolve: {other:?}"),
    }
    let canon: String =
        match sqlx::query("SELECT canonicalization FROM ledger.artifacts WHERE sha = $1")
            .bind(sha.as_str())
            .fetch_one(&db.admin)
            .await
            .and_then(|r| r.try_get(0))
        {
            Ok(c) => c,
            Err(e) => panic!("{e}"),
        };
    assert_eq!(canon, "jcs-rfc8785");
    db.finish().await;
}

// @req T2
#[tokio::test]
async fn storing_the_same_bytes_twice_is_idempotent() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    for (i, id) in ["d1", "d2"].into_iter().enumerate() {
        let art = artifact_bytes(b"hello, ABW\n".to_vec(), Canon::RawUtf8, "text/plain");
        if let Err(e) = ledger
            .dispatch(
                Append::new(id, Expected::NoStream, json!({"i": i})).artifact(art),
                &ctx(),
            )
            .await
        {
            panic!("{e:?}");
        }
    }
    assert_eq!(
        db.scalar_i64("SELECT count(*) FROM ledger.artifacts").await,
        1
    );
    db.finish().await;
}

// @req T2
#[tokio::test]
async fn database_recomputes_the_hash_of_inline_bytes() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let art = artifact_bytes(b"hello, ABW\n".to_vec(), Canon::RawUtf8, "text/plain");
    let out = match ledger
        .dispatch(
            Append::new("h1", Expected::NoStream, json!({})).artifact(art),
            &ctx(),
        )
        .await
    {
        Ok(d) => d.output,
        Err(e) => panic!("{e:?}"),
    };
    // Case 15 and 28: the stored sha equals V4 and Postgres's own sha256 over the bytes.
    let db_sha: String = match sqlx::query(
        "SELECT 'sha256:' || encode(sha256(bytes), 'hex') FROM ledger.artifacts WHERE sha = $1",
    )
    .bind(V4_SHA)
    .fetch_one(&db.admin)
    .await
    .and_then(|r| r.try_get(0))
    {
        Ok(s) => s,
        Err(e) => panic!("V4 artifact not stored under its sha: {e}"),
    };
    assert_eq!(db_sha, V4_SHA);
    // Case 14: bytes stored under a sha they do not hash to are rejected by a CHECK.
    let mut app = db.session(&db.app_url).await;
    let forged = format!(
        "INSERT INTO ledger.artifacts (sha, media_type, canonicalization, byte_len, bytes, created_pos)
         VALUES ('sha256:{}', 'text/plain', 'raw-utf8', 3, 'abc'::bytea, {})",
        "0".repeat(64),
        out.global_pos
    );
    assert_rejected(&mut app, &forged, "23514").await;
    db.finish().await;
}
