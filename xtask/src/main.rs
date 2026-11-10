//! `cargo xtask <command>`
//!
//! - `trace [--list] [--summary FILE]` runs the workspace tests, joins them to
//!   `docs/requirements/register.toml`, prints the matrix and fails if any requirement due
//!   at the current milestone is unmet. `--list` skips running tests.
//! - `pr-scope --base REF` fails if the diff against REF mixes product code and
//!   verification assets (GOV-04).
//! - `check` runs the static checks in `xtask::checks` over the repository.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use xtask::{checks, register::Register, results, scope, tags, trace};

const REGISTER: &str = "docs/requirements/register.toml";
const SCAN_ROOTS: &[&str] = &["tests", "crates", "xtask"];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("trace") => cmd_trace(&args[1..]),
        Some("pr-scope") => cmd_pr_scope(&args[1..]),
        Some("check") => cmd_check(),
        _ => Err(
            "usage: cargo xtask <trace [--list] [--summary FILE] | pr-scope --base REF | check>"
                .into(),
        ),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn root() -> Result<PathBuf, String> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| "run through `cargo xtask`")?;
    Path::new(&manifest)
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask has no parent directory".to_string())
}

fn flag_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            if path
                .file_name()
                .is_some_and(|n| n == "target" || n == "fixtures")
            {
                continue;
            }
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn cmd_trace(args: &[String]) -> Result<(), String> {
    let root = root()?;
    let reg_text =
        fs::read_to_string(root.join(REGISTER)).map_err(|e| format!("{REGISTER}: {e}"))?;
    let reg = Register::parse(&reg_text)?;

    let mut paths = Vec::new();
    for r in SCAN_ROOTS {
        rust_files(&root.join(r), &mut paths)?;
    }
    let mut files = Vec::new();
    for p in &paths {
        let text = fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        let rel = p.strip_prefix(&root).unwrap_or(p).display().to_string();
        files.push((rel, text));
    }
    let scan = tags::scan(files.iter().map(|(p, t)| (p.as_str(), t.as_str())));

    let list_only = args.iter().any(|a| a == "--list");
    let parsed;
    let results = if list_only {
        None
    } else {
        let out = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .current_dir(&root)
            .args([
                "test",
                "--workspace",
                "--all-targets",
                "--no-fail-fast",
                "--",
                "--include-ignored",
            ])
            .env("ABW_TRACE", "1")
            .output()
            .map_err(|e| format!("running cargo test: {e}"))?;
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        parsed = mark_ignored(results::parse(&combined), &scan);
        Some(&parsed)
    };

    let report = trace::evaluate(&reg, &scan, results);
    let md = trace::markdown(&report, &reg);
    println!("{md}");
    if let Some(path) = flag_value(args, "--summary") {
        let mut existing = fs::read_to_string(path).unwrap_or_default();
        existing.push_str(&md);
        fs::write(path, existing).map_err(|e| format!("{path}: {e}"))?;
    }

    if list_only {
        return Ok(());
    }
    if report.ok() {
        println!("trace: ok");
        Ok(())
    } else {
        let mut msg = String::from("trace: FAILED\n");
        for p in &report.problems {
            msg.push_str(&format!("  problem: {p}\n"));
        }
        for r in report.blocking() {
            msg.push_str(&format!(
                "  {} ({}) is {:?}: {}\n",
                r.id, r.milestone, r.status, r.title
            ));
        }
        Err(msg)
    }
}

/// Tests run with `--include-ignored` so pending tests show whether they already pass, but a
/// pending test is never evidence (GOV-05): its outcome is recorded as Ignored.
fn mark_ignored(
    mut map: std::collections::BTreeMap<String, Vec<results::Outcome>>,
    scan: &tags::Scan,
) -> std::collections::BTreeMap<String, Vec<results::Outcome>> {
    for t in scan.tests.iter().filter(|t| t.ignored) {
        if let Some(v) = map.get_mut(&t.name) {
            v.iter_mut().for_each(|o| *o = results::Outcome::Ignored);
        }
    }
    map
}

fn cmd_check() -> Result<(), String> {
    let root = root()?;
    let meta = checks::cargo_metadata(&root)?;
    let findings = checks::repo(&root, &meta)?;
    if findings.is_empty() {
        println!("check: ok");
        Ok(())
    } else {
        let lines: Vec<String> = findings.iter().map(ToString::to_string).collect();
        Err(format!(
            "check: {} finding(s)\n  {}",
            findings.len(),
            lines.join("\n  ")
        ))
    }
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn cmd_pr_scope(args: &[String]) -> Result<(), String> {
    let root = root()?;
    let base = flag_value(args, "--base").ok_or("pr-scope needs --base REF")?;
    let range = format!("{base}...HEAD");
    let names = git(&root, &["diff", "--name-only", &range])?;
    let changed: Vec<&str> = names.lines().filter(|l| !l.is_empty()).collect();
    let mut diffs = Vec::new();
    for p in changed
        .iter()
        .filter(|p| scope::classify(p) == scope::Kind::Verification)
    {
        diffs.push((
            p.to_string(),
            git(&root, &["diff", "--unified=0", &range, "--", p])?,
        ));
    }
    let diff_refs: Vec<(&str, &str)> = diffs
        .iter()
        .map(|(p, d)| (p.as_str(), d.as_str()))
        .collect();
    let msg = scope::check(&changed, &diff_refs)?;
    println!("{msg}");
    Ok(())
}
