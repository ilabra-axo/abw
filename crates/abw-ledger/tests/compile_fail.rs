//! T1, compile-time half: "a handler that tries to write directly fails to compile"
//! (target-system-design §3.9).
//!
//! Each case is a pair of tiny handler crates that depend only on `abw-ledger`, as a
//! handler crate does. The two differ in exactly one line. The conforming twin must compile
//! and the violating twin must fail with an error reported on that line. The twin rules
//! out a fixture that fails for an unrelated reason.

use std::path::{Path, PathBuf};
use std::process::Command;

const TEMPLATE_READ: &str = r#"#![allow(dead_code)]
use abw_ledger::{ReadDb, read_query};

async fn handler(db: ReadDb) {
    let q = @LINE@
    let rows = db.fetch_all(q).await;
    drop(rows);
}

fn main() {}
"#;

const TEMPLATE_HANDLE: &str = r#"#![allow(dead_code, unused_variables)]
use abw_ledger::{Ledger, ReadDb, read_query};

async fn handler(ledger: &Ledger, db: ReadDb) {
    let r = @LINE@
    let rows = db.fetch_all(read_query!("SELECT 1")).await;
    drop(rows);
}

fn main() {}
"#;

/// (case, template, conforming line, violating line)
const CASES: &[(&str, &str, &str, &str)] = &[
    (
        "update",
        TEMPLATE_READ,
        r#"read_query!("SELECT head FROM ledger.streams WHERE stream_id = 'probe:x'");"#,
        r#"read_query!("UPDATE ledger.streams SET head = head + 1 WHERE stream_id = 'probe:x'");"#,
    ),
    (
        "insert_lowercase",
        TEMPLATE_READ,
        r#"read_query!("select 1 from crud.notes");"#,
        r#"read_query!("insert into crud.notes (id) values (1)");"#,
    ),
    (
        "delete_in_cte",
        TEMPLATE_READ,
        r#"read_query!("WITH t AS (SELECT id FROM ledger.outbox) SELECT count(*) FROM t");"#,
        r#"read_query!("WITH t AS (DELETE FROM ledger.outbox RETURNING id) SELECT count(*) FROM t");"#,
    ),
    (
        "row_lock",
        TEMPLATE_READ,
        r#"read_query!("SELECT head FROM ledger.streams");"#,
        r#"read_query!("SELECT head FROM ledger.streams FOR UPDATE");"#,
    ),
    (
        "merge",
        TEMPLATE_READ,
        r#"read_query!("SELECT 1");"#,
        r#"read_query!("MERGE INTO proj.probes p USING (SELECT 1) s ON false WHEN NOT MATCHED THEN DO NOTHING");"#,
    ),
    (
        "read_handle_exposes_no_executor",
        TEMPLATE_HANDLE,
        r#"db.clone();"#,
        r#"db.as_executor();"#,
    ),
    (
        "ledger_pool_is_private",
        TEMPLATE_HANDLE,
        r#"ledger.reader();"#,
        r#"&ledger.pool;"#,
    ),
];

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("abw-compile-fail")
}

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

fn check(dir: &Path, bin: &str) -> (bool, String) {
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
        .env("CARGO_TARGET_DIR", dir.join("target"))
        .output();
    match out {
        Ok(o) => (
            o.status.success(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        ),
        Err(e) => panic!("running cargo check: {e}"),
    }
}

// @req T1
#[test]
fn handler_writes_do_not_compile() {
    let dir = fixture_dir();
    let ledger = env!("CARGO_MANIFEST_DIR");
    write(
        &dir.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"abw-handler-fixtures\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n\
             [workspace]\n\n[dependencies]\nabw-ledger = {{ path = {ledger:?} }}\n"
        ),
    );
    let lock = Path::new(ledger).join("../../Cargo.lock");
    if lock.exists()
        && let Err(e) = std::fs::copy(&lock, dir.join("Cargo.lock"))
    {
        panic!("copy Cargo.lock: {e}");
    }
    for (case, template, good, bad) in CASES {
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
            "{case}: the violating twin compiled; a handler could write:\n  {bad}"
        );
        let at = format!("src/bin/{case}_bad.rs:{line}:");
        assert!(
            stderr.contains(&at),
            "{case}: expected an error at {at}, got:\n{stderr}"
        );
    }
}
