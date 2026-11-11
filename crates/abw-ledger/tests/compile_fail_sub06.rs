//! SUB-06: no ledger transaction spans a non-ledger await (ADR-0008 §1 item 5; restores
//! ADR-0005 item 1; target-system-design §2.3, §3.9 layer 4).
//!
//! The guarantee has two halves. Inside `abw-ledger`, every `decide` is a plain synchronous
//! fn (`cargo xtask check`, check `sync-decide`). Outside it, nothing can write a decision
//! and nothing can hold a transaction: `Command` is sealed, and `Ledger`, `ReadDb` and `Crud`
//! hand out no transaction, connection or pool. A caller can therefore await an LLM, tool or
//! HTTP call only between dispatches, never inside one.
//!
//! Each case is a pair of tiny crates that depend only on `abw-ledger`, as `abw-agent` and
//! `abw-http` do, differing in one line. The conforming twin must compile and the violating
//! twin must fail with an error on that line (the T1 harness in `compile_fail.rs`).

use std::path::{Path, PathBuf};
use std::process::Command;

/// A caller holding the ledger handles, as the episode executor and HTTP handlers do.
const TEMPLATE_CALLER: &str = r#"#![allow(dead_code, unused_variables, unused_must_use, clippy::all)]
use abw_ledger::{Ledger, ReadDb};

async fn caller(ledger: &Ledger, db: ReadDb) {
    let r = @LINE@
}

fn main() {}
"#;

/// An item next to the caller: where an external decision would have to live.
const TEMPLATE_ITEM: &str = r#"#![allow(dead_code, unused_variables, clippy::all)]
struct Settle;

@LINE@

fn main() {}
"#;

/// (case, template, conforming line, violating line): the M1 surface, present today.
const CASES_NOW: &[(&str, &str, &str, &str)] = &[
    (
        "async_decide_outside_the_ledger",
        TEMPLATE_ITEM,
        "impl Settle { async fn decide(&self) {} }",
        "impl abw_ledger::Command for Settle { async fn decide(&self) {} }",
    ),
    (
        "sync_decide_outside_the_ledger",
        TEMPLATE_ITEM,
        "impl Settle { fn decide(&self) {} }",
        "impl abw_ledger::Command for Settle { fn decide(&self) {} }",
    ),
    (
        "ledger_has_no_begin",
        TEMPLATE_CALLER,
        "ledger.reader();",
        "ledger.begin();",
    ),
    (
        "ledger_has_no_acquire",
        TEMPLATE_CALLER,
        "ledger.reader();",
        "ledger.acquire();",
    ),
    (
        "ledger_has_no_transaction_scope",
        TEMPLATE_CALLER,
        "ledger.reader();",
        "ledger.transaction(|tx| async move { drop(tx) });",
    ),
    (
        "ledger_has_no_tx",
        TEMPLATE_CALLER,
        "ledger.reader();",
        "ledger.tx();",
    ),
    (
        "ledger_has_no_pool_accessor",
        TEMPLATE_CALLER,
        "ledger.reader();",
        "ledger.pool();",
    ),
    (
        "ledger_has_no_connection",
        TEMPLATE_CALLER,
        "ledger.reader();",
        "ledger.connection();",
    ),
    (
        "read_db_has_no_acquire",
        TEMPLATE_CALLER,
        "db.clone();",
        "db.acquire();",
    ),
    (
        "read_db_has_no_begin",
        TEMPLATE_CALLER,
        "db.clone();",
        "db.begin();",
    ),
    (
        "read_db_has_no_pool_accessor",
        TEMPLATE_CALLER,
        "db.clone();",
        "db.pool();",
    ),
];

/// The M2 surface: `abw_ledger::crud` (ADR-0008 §4) replaces design §14.5's `Crud::tx`.
const CASES_M2: &[(&str, &str, &str, &str)] = &[
    (
        "crud_has_no_tx",
        TEMPLATE_CALLER,
        "ledger.crud();",
        "ledger.crud().tx();",
    ),
    (
        "crud_has_no_begin",
        TEMPLATE_CALLER,
        "ledger.crud();",
        "ledger.crud().begin();",
    ),
    (
        "crud_has_no_transaction_scope",
        TEMPLATE_CALLER,
        "ledger.crud();",
        "ledger.crud().transaction(|tx| async move { drop(tx) });",
    ),
    (
        "crud_has_no_acquire",
        TEMPLATE_CALLER,
        "ledger.crud();",
        "ledger.crud().acquire();",
    ),
    (
        "crud_has_no_pool_accessor",
        TEMPLATE_CALLER,
        "ledger.crud();",
        "ledger.crud().pool();",
    ),
];

fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        panic!("{}: {e}", parent.display());
    }
    if let Err(e) = std::fs::write(path, text) {
        panic!("{}: {e}", path.display());
    }
}

/// Builds the fixtures in their own package directory, sharing the T1 harness's target
/// directory so `abw-ledger` is compiled once for both.
fn check(dir: &Path, bin: &str) -> (bool, String) {
    let target = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("abw-compile-fail")
        .join("target");
    let out = Command::new(env!("CARGO"))
        .current_dir(dir)
        .args([
            "check",
            "--quiet",
            "--message-format",
            "short",
            "--bin",
            bin,
        ])
        .env("CARGO_TARGET_DIR", target)
        .output();
    match out {
        Ok(o) => (
            o.status.success(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        ),
        Err(e) => panic!("running cargo check: {e}"),
    }
}

fn run_twins(package: &str, cases: &[(&str, &str, &str, &str)]) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(package);
    let ledger = env!("CARGO_MANIFEST_DIR");
    write(
        &dir.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"{package}\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n\
             [workspace]\n\n[dependencies]\nabw-ledger = {{ path = {ledger:?} }}\n"
        ),
    );
    let lock = Path::new(ledger).join("../../Cargo.lock");
    if lock.exists()
        && let Err(e) = std::fs::copy(&lock, dir.join("Cargo.lock"))
    {
        panic!("copy Cargo.lock: {e}");
    }
    for (case, template, good, bad) in cases {
        assert!(
            template.matches("@LINE@").count() == 1,
            "{case}: template needs one @LINE@"
        );
        let line = template
            .lines()
            .position(|l| l.contains("@LINE@"))
            .map_or(0, |i| i + 1);
        write(
            &dir.join(format!("src/bin/{case}_ok.rs")),
            &template.replace("@LINE@", good),
        );
        write(
            &dir.join(format!("src/bin/{case}_bad.rs")),
            &template.replace("@LINE@", bad),
        );
        let (ok, stderr) = check(&dir, &format!("{case}_ok"));
        assert!(ok, "{case}: the conforming twin must compile:\n{stderr}");
        let (ok, stderr) = check(&dir, &format!("{case}_bad"));
        assert!(
            !ok,
            "{case}: the violating twin compiled; a caller could hold a ledger transaction \
             across an await:\n  {bad}"
        );
        let at = format!("src/bin/{case}_bad.rs:{line}:");
        assert!(
            stderr.contains(&at),
            "{case}: expected an error at {at}, got:\n{stderr}"
        );
    }
}

// @req SUB-06
#[test]
fn no_decision_or_transaction_is_reachable_outside_the_ledger() {
    run_twins("abw-sub06-fixtures", CASES_NOW);
}

// @req SUB-06
#[test]
fn crud_hands_out_no_transaction() {
    run_twins("abw-sub06-crud-fixtures", CASES_M2);
}
