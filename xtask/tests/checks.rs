//! Static checks: each is shown rejecting a violating input and accepting a conforming one,
//! then run over the repository itself.

use std::path::Path;
use xtask::checks::{self, File, SpawnAllowlist};

fn file<'a>(path: &'a str, text: &'a str) -> File<'a> {
    File { path, text }
}

fn root() -> &'static Path {
    match Path::new(env!("CARGO_MANIFEST_DIR")).parent() {
        Some(p) => p,
        None => panic!("xtask has no parent directory"),
    }
}

fn repo_findings(check: &str) -> Vec<String> {
    let meta = match checks::cargo_metadata(root()) {
        Ok(m) => m,
        Err(e) => panic!("{e}"),
    };
    match checks::repo(root(), &meta) {
        Ok(f) => f
            .into_iter()
            .filter(|x| x.check == check)
            .map(|x| x.to_string())
            .collect(),
        Err(e) => panic!("{e}"),
    }
}

const META_OK: &str = r#"{"workspace_members":["a","b"],"packages":[
 {"name":"abw-ledger","id":"a","dependencies":[{"name":"sqlx","kind":null}]},
 {"name":"abw-http","id":"b","dependencies":[{"name":"axum","kind":null},{"name":"sqlx","kind":"dev"}]}]}"#;

const META_BAD: &str = r#"{"workspace_members":["a","b","c"],"packages":[
 {"name":"abw-ledger","id":"a","dependencies":[{"name":"sqlx","kind":null}]},
 {"name":"abw-http","id":"b","dependencies":[{"name":"sqlx-postgres","kind":null}]},
 {"name":"abw-agent","id":"c","dependencies":[{"name":"nostr-sdk","kind":"build"}]}]}"#;

// @req T1
#[test]
fn check_dependencies_reject_sqlx_outside_the_ledger() {
    let bad = match checks::dependencies(META_BAD) {
        Ok(f) => f,
        Err(e) => panic!("{e}"),
    };
    assert_eq!(bad.len(), 2, "{bad:?}");
    assert!(
        bad.iter()
            .any(|f| f.location.starts_with("abw-http") && f.message.contains("sqlx-postgres"))
    );
    assert!(
        bad.iter()
            .any(|f| f.location.starts_with("abw-agent") && f.message.contains("nostr-sdk"))
    );
    assert_eq!(checks::dependencies(META_OK), Ok(vec![]));
}

// @req T1
#[test]
fn check_write_sql_rejects_writes_outside_the_ledger() {
    let violating = r##"
        async fn h() {
            let q = "UPDATE ledger.streams SET head = 3 WHERE stream_id = $1";
            let r = r#"insert into crud.teams (id) values ($1)"#;
            let d = "DELETE FROM proj.probes";
        }
    "##;
    let found = checks::write_sql(&[file("crates/abw-http/src/h.rs", violating)]);
    assert_eq!(found.len(), 3, "{found:?}");
    assert_eq!(found[0].location, "crates/abw-http/src/h.rs:3");

    let conforming = r#"
        // UPDATE ledger.streams SET head = 1  -- a comment, not code
        fn h() {
            let q = "SELECT head FROM ledger.streams WHERE stream_id = $1";
            let msg = "update the plan, then truncate the log";
            let s = "allowance.granted";
        }
    "#;
    assert!(checks::write_sql(&[file("crates/abw-http/src/h.rs", conforming)]).is_empty());
    // The ledger itself, tests and non-crate paths are out of scope.
    assert!(checks::write_sql(&[file("crates/abw-ledger/src/append.rs", violating)]).is_empty());
    assert!(checks::write_sql(&[file("crates/abw-http/tests/t.rs", violating)]).is_empty());
}

// @req T1
#[test]
fn repo_has_no_restricted_dependencies() {
    assert_eq!(repo_findings("dependencies"), Vec::<String>::new());
}

// @req T1
#[test]
fn repo_has_no_sql_writes_outside_the_ledger() {
    assert_eq!(repo_findings("write-sql"), Vec::<String>::new());
}

fn allow(toml_text: &str) -> SpawnAllowlist {
    match toml::from_str(toml_text) {
        Ok(a) => a,
        Err(e) => panic!("{e}"),
    }
}

// @req T5
#[test]
fn check_spawn_rejects_unlisted_task_spawns() {
    let src = "fn f() {\n    tokio::spawn(async move { charge().await });\n    let h = set.spawn (job());\n    tokio::task::spawn_blocking(|| hash());\n}\n";
    let none = allow("");
    let found = checks::spawn_sites(&[file("crates/abw-agent/src/run.rs", src)], &none);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0]
            .message
            .starts_with("2 spawn site(s), 0 allowlisted"),
        "{}",
        found[0].message
    );

    let listed = allow(
        "[[site]]\npath = \"crates/abw-agent/src/run.rs\"\ncount = 2\nreason = \"worker loop\"\nclass = \"non_trust\"\n",
    );
    assert!(checks::spawn_sites(&[file("crates/abw-agent/src/run.rs", src)], &listed).is_empty());

    let trust = allow(
        "[[site]]\npath = \"crates/abw-agent/src/run.rs\"\ncount = 2\nreason = \"settle later\"\nclass = \"trust\"\n",
    );
    assert!(!checks::spawn_sites(&[file("crates/abw-agent/src/run.rs", src)], &trust).is_empty());

    let comment_only = "// tokio::spawn(x)\nfn f() { let s = \"spawn(\"; spawn_blocking(g); }\n";
    assert!(
        checks::spawn_sites(&[file("crates/abw-agent/src/run.rs", comment_only)], &none).is_empty()
    );
}

// @req T5
#[test]
fn repo_spawns_are_allowlisted() {
    assert_eq!(repo_findings("spawn"), Vec::<String>::new());
}

// @req T16
#[test]
fn check_vocabulary_rejects_dimension_names_outside_the_lattice() {
    let src = "struct PortBinding;\nfn grounding_score() {}\nconst S: &str = \"Truth check\";\n// presence in a comment is fine\n";
    let found = checks::vocabulary(&[file("crates/abw-agent/src/x.rs", src)]);
    let words: Vec<&str> = found
        .iter()
        .map(|f| f.message.split('`').nth(1).unwrap_or(""))
        .collect();
    assert_eq!(words, vec!["binding", "grounding", "truth"], "{found:?}");

    let sql = "CREATE TABLE proj.x (liveness_ok bool); -- binding in a comment\n";
    assert_eq!(
        checks::vocabulary(&[file("crates/abw-ledger/migrations/0001.sql", sql)]).len(),
        1
    );

    let ok = "fn bind(q: Q) -> Q { q.bind(1) }\nstruct Present;\n";
    assert!(checks::vocabulary(&[file("crates/abw-agent/src/x.rs", ok)]).is_empty());
    assert!(checks::vocabulary(&[file("crates/abw-lattice/src/lib.rs", src)]).is_empty());
}

// @req T16
#[test]
fn repo_reserves_dimension_names_for_the_lattice() {
    assert_eq!(repo_findings("vocabulary"), Vec::<String>::new());
}

// ADR-0004: the probe fixture stays a fixture. Not a register requirement; kept as a guard.
#[test]
fn check_probe_isolation() {
    let bad = "use abw_ledger::probe::Append;\nfn f() { abw_ledger :: probe::Append::new(); }\n";
    assert_eq!(
        checks::probe_isolation(&[file("crates/abw-agent/src/x.rs", bad)]).len(),
        2
    );
    assert!(checks::probe_isolation(&[file("crates/abw-ledger/src/probe.rs", bad)]).is_empty());
    assert!(checks::probe_isolation(&[file("crates/abw-ledger/tests/append.rs", bad)]).is_empty());
    assert_eq!(repo_findings("probe"), Vec::<String>::new());
}

const META_HTTP_BAD: &str = r#"{"workspace_members":["a","b","c"],"packages":[
 {"name":"abw-agent","id":"a","dependencies":[{"name":"reqwest","kind":null}]},
 {"name":"abw-http","id":"b","dependencies":[{"name":"reqwest","kind":null},{"name":"ureq","kind":"build"}]},
 {"name":"abw-lattice","id":"c","dependencies":[{"name":"reqwest","kind":"dev"}]}]}"#;

// @req T18, CAP-14.05
#[test]
fn check_dependencies_keep_http_clients_in_the_agent_crate() {
    let found = match checks::dependencies(META_HTTP_BAD) {
        Ok(f) => f,
        Err(e) => panic!("{e}"),
    };
    let mut locs: Vec<String> = found
        .iter()
        .map(|f| format!("{} {}", f.location, f.message))
        .collect();
    locs.sort();
    assert_eq!(found.len(), 2, "{locs:?}");
    assert!(
        found
            .iter()
            .all(|f| f.location.starts_with("abw-http") && f.message.contains("T18")),
        "{locs:?}"
    );
    // abw-agent may depend on reqwest (its egress module builds the clients); dev-deps are exempt.
    assert!(
        !found
            .iter()
            .any(|f| f.location.starts_with("abw-agent") || f.location.starts_with("abw-lattice"))
    );
}

// @req T18, CAP-14.05
#[test]
fn check_egress_rejects_outbound_clients_outside_the_guard() {
    let violating = r#"
use reqwest::Client;
use std::net::TcpStream;

fn a() -> reqwest::Client { reqwest::Client::new() }
fn b() { let c = reqwest :: Client :: builder().build(); }
fn c() { let c = Client::builder().build(); }
async fn d() { let r = reqwest::get("https://x").await; }
fn e() { let s = TcpStream::connect("10.0.0.1:80"); }
fn f() { let r = ureq::get("http://x").call(); }
fn g() { let c = hyper_util::client::legacy::Client::builder(x); }
"#;
    let found = checks::egress_clients(&[file("crates/abw-http/src/fetch.rs", violating)]);
    let lines: Vec<String> = found.iter().map(|f| f.location.clone()).collect();
    assert_eq!(
        lines,
        vec![
            "crates/abw-http/src/fetch.rs:5",
            "crates/abw-http/src/fetch.rs:6",
            "crates/abw-http/src/fetch.rs:7",
            "crates/abw-http/src/fetch.rs:8",
            "crates/abw-http/src/fetch.rs:9",
            "crates/abw-http/src/fetch.rs:10",
            "crates/abw-http/src/fetch.rs:11",
        ],
        "{found:?}"
    );
    assert!(found.iter().all(|f| f.check == "egress"));

    // The same code inside the guard module is the one permitted place.
    for guard in [
        "crates/abw-agent/src/egress.rs",
        "crates/abw-agent/src/egress/client.rs",
    ] {
        assert!(
            checks::egress_clients(&[file(guard, violating)]).is_empty(),
            "{guard}"
        );
    }
    // Tests are not product code.
    assert!(
        checks::egress_clients(&[file("crates/abw-agent/tests/egress.rs", violating)]).is_empty()
    );
}

// @req T18, CAP-14.05
#[test]
fn check_egress_accepts_clients_obtained_from_the_guard() {
    let conforming = r#"
// reqwest::Client::new() in a comment is not code
use abw_agent::egress;

struct Anthropic { client: reqwest::Client }

fn new() -> Result<Anthropic, egress::NetGuardError> {
    let client = egress::fixed_host_client(std::time::Duration::from_secs(60))?;
    let msg = "call reqwest::Client::new() only in egress";
    let other = sqlx_like::Client::new(1);
    Ok(Anthropic { client })
}
"#;
    assert_eq!(
        checks::egress_clients(&[file("crates/abw-agent/src/llm/anthropic.rs", conforming)]),
        vec![]
    );
    // Without `use reqwest`, a type named Client elsewhere is not an HTTP client.
    let unrelated = "fn f() { let c = redis::Client::new(); let d = Client::builder(); }\n";
    assert!(
        checks::egress_clients(&[file("crates/abw-agent/src/memory.rs", unrelated)]).is_empty()
    );
}

// @req T18, CAP-14.05
#[test]
fn repo_constructs_outbound_clients_only_in_the_egress_guard() {
    assert_eq!(repo_findings("egress"), Vec::<String>::new());
}

// @req SUB-06
#[test]
fn check_sync_decide_rejects_async_decisions() {
    let violating = r#"
impl CommandSpec for Hold {
    async fn decide(&self, heads: &Heads) -> Result<Vec<StreamAppend>, DispatchError> {
        Ok(vec![])
    }
}
impl CommandSpec for Close {
    fn decide(&self, heads: &Heads) -> Result<Vec<StreamAppend>, DispatchError> {
        let price = self.llm.complete(req).await;
        Ok(vec![])
    }
}
trait Later {
    fn decide(&self) -> BoxFuture<'_, Result<(), DispatchError>>;
}
impl Other for Settle {
    pub(crate) fn decide(&self) -> impl std::future::Future<Output = ()> { async {} }
}
"#;
    let found = checks::sync_decide(&[file("crates/abw-ledger/src/episode.rs", violating)]);
    let locs: Vec<(String, String)> = found
        .iter()
        .map(|f| (f.location.clone(), f.message.clone()))
        .collect();
    let lines: Vec<&str> = found.iter().map(|f| f.location.as_str()).collect();
    assert_eq!(
        lines,
        vec![
            "crates/abw-ledger/src/episode.rs:3",
            "crates/abw-ledger/src/episode.rs:8",
            "crates/abw-ledger/src/episode.rs:14",
            "crates/abw-ledger/src/episode.rs:17",
        ],
        "{locs:?}"
    );
    assert!(found[0].message.contains("async"), "{locs:?}");
    assert!(found[1].message.contains(".await"), "{locs:?}");
    assert!(found[2].message.contains("Future"), "{locs:?}");
}

// @req SUB-06
#[test]
fn check_sync_decide_accepts_pure_decisions() {
    let conforming = r#"
impl CommandSpec for Hold {
    // async fn decide(&self) would be refused; this is a comment
    fn decide(&self, heads: &Heads, ctx: &CommandCtx) -> Result<Vec<StreamAppend>, DispatchError> {
        let note = "no .await here";
        let n = heads.head(&self.stream);
        Ok(vec![])
    }
    fn decided(&self) {}
}
async fn dispatch(ledger: &Ledger) { let x = ledger.dispatch(cmd, &ctx).await; }
fn undecided() {}
async fn decide_later() { work().await; }
"#;
    assert_eq!(
        checks::sync_decide(&[file("crates/abw-ledger/src/allowance.rs", conforming)]),
        vec![]
    );
}

// @req SUB-06
#[test]
fn repo_decisions_are_synchronous() {
    assert_eq!(repo_findings("sync-decide"), Vec::<String>::new());
}

#[test]
fn lexer_separates_code_strings_and_comments() {
    use xtask::lex::{Kind, segments};
    let src = "let q = '\\''; let a = 'x'; let b = '\"'; // c \"not a string\"\nlet s = r#\"raw \"quoted\" \"#; /* block /* nested */ */ let t = \"e\\\"s\";";
    let strs: Vec<String> = segments(src)
        .into_iter()
        .filter(|s| s.kind == Kind::Str)
        .map(|s| s.text)
        .collect();
    assert_eq!(
        strs,
        vec!["raw \"quoted\" ".to_string(), "e\\\"s".to_string()]
    );
}
