//! GOV-04: a PR changes verification assets or product code, never both.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Tests, the register, the harness and CI.
    Verification,
    /// Crate sources and migrations.
    Product,
    /// Docs, manifests, lockfile, top-level files.
    Neutral,
}

pub fn classify(path: &str) -> Kind {
    let p = path.trim_start_matches("./");
    let verification = p.starts_with("tests/")
        || p.starts_with("xtask/")
        || p.starts_with("docs/requirements/")
        || p.starts_with(".github/")
        || p == "deny.toml"
        || p == "clippy.toml"
        || (p.starts_with("crates/") && p.contains("/tests/"))
        || (p.starts_with("crates/") && p.contains("/benches/"));
    if verification {
        return Kind::Verification;
    }
    let product = p.starts_with("migrations/")
        || (p.starts_with("crates/")
            && (p.contains("/src/") || p.contains("/migrations/") || p.ends_with("/build.rs")));
    if product {
        Kind::Product
    } else {
        Kind::Neutral
    }
}

/// `diffs` holds `(path, unified diff with zero context)` for each verification file in a
/// mixed PR. Such a file is allowed only if every change is the removal of an
/// `#[ignore = "pending ..."]` line or of the `#![cfg(any())]` gate that keeps a test file
/// out of the build until its API exists (AGENTS.md; ADR-0008 item 12). That is how a
/// product PR turns pending tests on.
pub fn check(changed: &[&str], diffs: &[(&str, &str)]) -> Result<String, String> {
    let product: Vec<&str> = changed
        .iter()
        .copied()
        .filter(|p| classify(p) == Kind::Product)
        .collect();
    let verification: Vec<&str> = changed
        .iter()
        .copied()
        .filter(|p| classify(p) == Kind::Verification)
        .collect();
    if product.is_empty() || verification.is_empty() {
        let kind = if !product.is_empty() {
            "product"
        } else if !verification.is_empty() {
            "verification"
        } else {
            "neutral"
        };
        return Ok(format!("pr-scope: {kind} PR ({} files)", changed.len()));
    }
    let mut offending = Vec::new();
    for v in &verification {
        let diff = diffs.iter().find(|(p, _)| p == v).map(|(_, d)| *d);
        if !diff.is_some_and(only_unignores) {
            offending.push(*v);
        }
    }
    if offending.is_empty() {
        Ok(format!(
            "pr-scope: product PR enabling pending tests in {} file(s)",
            verification.len()
        ))
    } else {
        Err(format!(
            "pr-scope: this PR changes product code and verification assets together.\n\
             Split it: tests first (merged failing or pending), then the implementation.\n\
             product: {}\nverification: {}",
            product.join(", "),
            offending.join(", ")
        ))
    }
}

fn only_unignores(diff: &str) -> bool {
    let mut removed_any = false;
    for line in diff.lines() {
        if line.starts_with("+++") || line.starts_with("---") || line.starts_with("@@") {
            continue;
        }
        if line.starts_with("diff ") || line.starts_with("index ") {
            continue;
        }
        if let Some(added) = line.strip_prefix('+') {
            if !added.trim().is_empty() {
                return false;
            }
        } else if let Some(removed) = line.strip_prefix('-') {
            let r = removed.trim();
            if r.starts_with("#[ignore = \"pending") || r == "#![cfg(any())]" {
                removed_any = true;
            } else if !r.is_empty() {
                return false;
            }
        }
    }
    removed_any
}
