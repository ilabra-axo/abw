//! PROT-09: the curated agent cards are carried over as data and import as versioned agents
//! (PLAN.md §9; docs/harvest/agents.md; docs/harvest/DECISIONS.md D1, D2, D5;
//! target-system-design §4.2-§4.3; ADR-0008 §6).
//!
//! The expected set is derived from the harvest, not listed here: every inventory row whose
//! disposition is `port` or `port:tenant-*`, plus `kask_advisor` (D3), minus the drops in D1.

#[path = "../../abw-ledger/tests/common/mod.rs"]
mod common;
#[path = "../../abw-ledger/tests/common/m2.rs"]
mod m2;

use abw_agent::card::{self, EXCLUDED_FIELDS};
use abw_agent::import;
use abw_ledger::{Actor, CommandCtx, agent};
use common::TestDb;
use m2::*;
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn agents_dir() -> PathBuf {
    repo().join("agents")
}

fn read(path: &Path) -> String {
    match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => panic!("{}: {e}", path.display()),
    }
}

/// `port` and `port:tenant-*` slugs from the inventory table of docs/harvest/agents.md.
fn harvest_ported() -> BTreeSet<String> {
    let text = read(&repo().join("docs/harvest/agents.md"));
    let mut out = BTreeSet::new();
    let mut rows = 0;
    for line in text.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // | # | `slug` | type | tenant | tools | services | contract | prompt | disposition | note |
        if cells.len() < 11 || cells[1].parse::<u32>().is_err() {
            continue;
        }
        rows += 1;
        let slug = cells[2].trim_matches('`').to_string();
        let disposition = cells[9];
        if disposition == "port" || disposition.starts_with("port:tenant-") {
            out.insert(slug);
        }
    }
    assert_eq!(
        rows, 104,
        "the harvest inventory lists all 104 curated cards"
    );
    out
}

/// The slugs in the D1 line of docs/harvest/DECISIONS.md (every backticked name on it).
fn d1_drops() -> BTreeSet<String> {
    let text = read(&repo().join("docs/harvest/DECISIONS.md"));
    let line = match text.lines().find(|l| l.contains("**D1.")) {
        Some(l) => l,
        None => panic!("DECISIONS.md has no D1 line"),
    };
    let drops: BTreeSet<String> = line
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect();
    assert_eq!(drops.len(), 7, "D1 drops seven cards: {drops:?}");
    drops
}

fn expected_ports() -> BTreeSet<String> {
    let mut want = harvest_ported();
    want.insert("kask_advisor".to_string());
    for d in d1_drops() {
        want.remove(&d);
    }
    want
}

fn present() -> BTreeSet<String> {
    match import::read_dir(&agents_dir()) {
        Ok(cards) => cards.into_iter().map(|(slug, _)| slug).collect(),
        Err(e) => panic!("read {}: {e:?}", agents_dir().display()),
    }
}

/// Resolution per ADR-0008 §6, written out independently of the product.
fn resolve_by_hand(raw: &Value) -> Value {
    let mut obj: Map<String, Value> = match raw.as_object() {
        Some(o) => o.clone(),
        None => panic!("card is not an object"),
    };
    for f in EXCLUDED_FIELDS {
        obj.remove(*f);
    }
    obj.insert("$excluded".into(), json!(EXCLUDED_FIELDS));
    if obj.get("system_prompt").is_none_or(Value::is_null) {
        obj.insert("system_prompt".into(), json!(""));
    }
    let caps = obj.entry("capabilities").or_insert_with(|| json!({}));
    if let Some(caps) = caps.as_object_mut()
        && !caps.contains_key("tools_allowed")
    {
        let mut names: Vec<Value> = Vec::new();
        for t in caps
            .get("mcp_tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            if let Some(n) = t.get("name")
                && !names.contains(n)
            {
                names.push(n.clone());
            }
        }
        caps.insert("tools_allowed".into(), Value::Array(names));
    }
    Value::Object(obj)
}

// @req PROT-09
#[test]
fn the_ported_card_set_matches_the_harvest() {
    let want = expected_ports();
    assert_eq!(
        want.len(),
        98,
        "97 port rows plus kask_advisor; D1 drops none of them"
    );
    let have = present();
    let missing: Vec<&String> = want.difference(&have).collect();
    let extra: Vec<&String> = have.difference(&want).collect();
    assert!(missing.is_empty(), "cards not ported: {missing:?}");
    assert!(
        extra.is_empty(),
        "cards present without a port disposition: {extra:?}"
    );
    for dropped in d1_drops() {
        assert!(
            !agents_dir().join(&dropped).exists(),
            "{dropped} was dropped by D1 but is present"
        );
    }
}

// @req PROT-09, CAP-01.01
#[test]
fn every_ported_card_resolves_by_the_pinned_rules() {
    let cards = match import::read_dir(&agents_dir()) {
        Ok(c) => c,
        Err(e) => panic!("{e:?}"),
    };
    assert!(!cards.is_empty());
    for (slug, raw) in &cards {
        assert_eq!(
            raw.get("agent_id").and_then(Value::as_str),
            Some(slug.as_str())
        );
        let resolved = match card::resolve(raw) {
            Ok(r) => r,
            Err(e) => panic!("{slug}: {e:?}"),
        };
        let by_hand = resolve_by_hand(raw);
        assert_eq!(
            resolved.json(),
            &by_hand,
            "{slug}: resolution differs from ADR-0008 §6"
        );
        assert_eq!(resolved.agent_id(), slug);
        assert_eq!(
            resolved.card_sha(),
            ok(abw_canon::sha256_jcs(&by_hand), "jcs"),
            "{slug}: the version id is the JCS hash of the resolved card"
        );
        let prompt = by_hand
            .get("system_prompt")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert_eq!(
            resolved.prompt_sha(),
            abw_canon::sha256_raw(prompt.as_bytes())
        );
        for f in EXCLUDED_FIELDS {
            assert!(resolved.json().get(*f).is_none(), "{slug} keeps {f}");
        }
    }
}

// @req PROT-09, CAP-01.01
#[test]
fn runtime_fields_do_not_change_a_cards_identity_but_configuration_does() {
    assert_eq!(
        EXCLUDED_FIELDS,
        [
            "performance",
            "usage",
            "ontology_stats",
            "wallet",
            "declared_prompt_sha256"
        ]
    );
    let base = json!({
        "agent_id": "sample",
        "system_prompt": "You forecast.",
        "capabilities": {"mcp_tools": [{"name": "web_search"}, {"name": "execute_agent"}, {"name": "web_search"}]},
        "performance": {"episodes": 10},
        "wallet": {"balance": 5},
    });
    let a = match card::resolve(&base) {
        Ok(r) => r,
        Err(e) => panic!("{e:?}"),
    };
    assert_eq!(
        a.tools_allowed(),
        ["web_search", "execute_agent"],
        "backfilled, deduplicated"
    );
    let mut runtime = base.clone();
    runtime["performance"] = json!({"episodes": 9999});
    runtime["usage"] = json!({"calls": 1});
    runtime["ontology_stats"] = json!({"terms": 3});
    runtime["declared_prompt_sha256"] = json!("abc");
    match card::resolve(&runtime) {
        Ok(b) => assert_eq!(
            b.card_sha(),
            a.card_sha(),
            "runtime fields changed the identity"
        ),
        Err(e) => panic!("{e:?}"),
    }
    let mut prompt = base.clone();
    prompt["system_prompt"] = json!("You forecast carefully.");
    let mut tools = base.clone();
    tools["capabilities"]["tools_allowed"] = json!([]);
    for changed in [prompt, tools] {
        match card::resolve(&changed) {
            Ok(c) => assert_ne!(c.card_sha(), a.card_sha(), "{changed}"),
            Err(e) => panic!("{e:?}"),
        }
    }
    // An explicit allowlist wins over the backfill; a card with no prompt gets "".
    match card::resolve(
        &json!({"agent_id": "bare", "capabilities": {"tools_allowed": ["x"], "mcp_tools": [{"name": "y"}]}}),
    ) {
        Ok(c) => {
            assert_eq!(c.tools_allowed(), ["x"]);
            assert_eq!(c.json().get("system_prompt"), Some(&json!("")));
        }
        Err(e) => panic!("{e:?}"),
    }
    for bad in [
        json!([]),
        json!({"system_prompt": "no id"}),
        json!({"agent_id": "has space"}),
    ] {
        assert!(card::resolve(&bad).is_err(), "{bad} resolved");
    }
}

// @req PROT-09
#[test]
fn phantom_tools_are_stripped_from_ported_cards() {
    // Tools the harvest lists as never dispatchable (agents.md rows 1, 4, 11; DECISIONS.md D5).
    let cases: [(&str, &[&str]); 3] = [
        (
            "adaptogen_curator",
            &[
                "adaptogen_species_search",
                "adaptogen_species_detail",
                "adaptogen_compound_search",
                "adaptogen_evidence_query",
                "adaptogen_safety_check",
                "adaptogen_drug_interaction_check",
                "adaptogen_traditional_use_query",
                "adaptogen_indication_search",
                "adaptogen_genomic_markers",
                "adaptogen_population_variants",
                "adaptogen_compare_species",
                "adaptogen_medicine_system_browse",
            ],
        ),
        (
            "ar_avatar_renderer",
            &[
                "scene_planner",
                "interaction_designer",
                "avatar_profile_loader",
            ],
        ),
        (
            "bluesky_publisher",
            &[
                "create_post",
                "upload_blob",
                "resolve_handle",
                "get_post",
                "create_thread",
                "fetch_og_metadata",
            ],
        ),
    ];
    let cards: std::collections::BTreeMap<String, Value> = match import::read_dir(&agents_dir()) {
        Ok(c) => c.into_iter().collect(),
        Err(e) => panic!("{e:?}"),
    };
    for (slug, phantoms) in cases {
        let raw = match cards.get(slug) {
            Some(c) => c,
            None => panic!("{slug} is not ported"),
        };
        let resolved = match card::resolve(raw) {
            Ok(r) => r,
            Err(e) => panic!("{slug}: {e:?}"),
        };
        let allowed = resolved.tools_allowed();
        for p in phantoms {
            assert!(
                !allowed.iter().any(|t| t == p),
                "{slug} still allows phantom {p}"
            );
            assert!(
                !raw.to_string().contains(&format!("\"name\":\"{p}\"")),
                "{slug} still declares phantom {p}"
            );
        }
    }
}

// @req PROT-09, CAP-01.01
#[tokio::test]
async fn every_ported_card_imports_as_a_versioned_agent() {
    let db = TestDb::migrated().await;
    let (ledger, read, _fence) = db.ledger().await;
    let ctx = CommandCtx::new("platform", Actor::platform("card-import"));
    let report = match import::import_dir(&ledger, &agents_dir(), &ctx).await {
        Ok(r) => r,
        Err(e) => panic!("import: {e:?}"),
    };
    let imported: BTreeSet<String> = report.agents.iter().map(|a| a.agent_id.clone()).collect();
    assert_eq!(imported, expected_ports());
    let raw: std::collections::BTreeMap<String, Value> = match import::read_dir(&agents_dir()) {
        Ok(c) => c.into_iter().collect(),
        Err(e) => panic!("{e:?}"),
    };
    for a in &report.agents {
        assert!(a.created, "{} was not created", a.agent_id);
        assert_eq!(a.version_number, 1, "{}", a.agent_id);
        let want = match raw.get(&a.agent_id).map(card::resolve) {
            Some(Ok(r)) => r,
            other => panic!("{}: {other:?}", a.agent_id),
        };
        assert_eq!(a.card_sha, want.card_sha(), "{}", a.agent_id);
        assert_eq!(
            a.card_sha,
            ok(abw_canon::sha256_jcs(want.json()), "jcs"),
            "{}: version id = JCS hash of the resolved card",
            a.agent_id
        );
        match agent::head(&read, "platform", &a.agent_id).await {
            Ok(Some(h)) => {
                assert_eq!(h.card_sha, a.card_sha, "{}", a.agent_id);
                // D2: archived-flagged dependencies are ported active; nothing imports retired.
                assert!(!h.retired, "{} imported retired", a.agent_id);
            }
            other => panic!("{}: head {other:?}", a.agent_id),
        }
        match read.resolve(&a.card_sha).await {
            Ok(Some(bytes)) => assert_eq!(bytes, ok(abw_canon::jcs(want.json()), "jcs")),
            other => panic!("{}: card artifact {other:?}", a.agent_id),
        }
    }
    for dropped in d1_drops() {
        match agent::head(&read, "platform", &dropped).await {
            Ok(None) => {}
            other => panic!("dropped card {dropped} was imported: {other:?}"),
        }
    }
    let seeds = db
        .scalar_i64(
            "SELECT count(*) FROM ledger.events
              WHERE event_type = 'agent.version_created' AND payload->>'reason' = 'seed'",
        )
        .await;
    assert_eq!(seeds, expected_ports().len() as i64);
    db.finish().await;
}

// @req PROT-09, CAP-01.01
#[tokio::test]
async fn reimporting_unchanged_cards_creates_no_versions() {
    let db = TestDb::migrated().await;
    let (ledger, _read, _fence) = db.ledger().await;
    let ctx = CommandCtx::new("platform", Actor::platform("card-import"));
    for pass in 0..2 {
        let report = match import::import_dir(&ledger, &agents_dir(), &ctx).await {
            Ok(r) => r,
            Err(e) => panic!("pass {pass}: {e:?}"),
        };
        assert_eq!(
            report.agents.iter().filter(|a| a.created).count(),
            if pass == 0 { expected_ports().len() } else { 0 },
            "pass {pass}"
        );
        assert!(report.agents.iter().all(|a| a.version_number == 1));
    }
    let versions = db
        .scalar_i64("SELECT count(*) FROM ledger.events WHERE event_type = 'agent.version_created'")
        .await;
    assert_eq!(versions, expected_ports().len() as i64);
    db.finish().await;
}

// @req PROT-09
#[test]
fn a_card_whose_id_differs_from_its_directory_is_refused() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("abw-card-import-mismatch");
    let card_dir = dir.join("right_name");
    if let Err(e) = std::fs::create_dir_all(&card_dir) {
        panic!("{e}");
    }
    let write = |id: &str| {
        let body = json!({"agent_id": id, "system_prompt": "x"}).to_string();
        if let Err(e) = std::fs::write(card_dir.join("agent_card.json"), body) {
            panic!("{e}");
        }
    };
    write("wrong_name");
    assert!(
        import::read_dir(&dir).is_err(),
        "a mislabelled card was read"
    );
    write("right_name");
    match import::read_dir(&dir) {
        Ok(cards) => assert_eq!(cards.len(), 1),
        Err(e) => panic!("{e:?}"),
    }
}
