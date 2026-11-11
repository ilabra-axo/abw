//! Static checks run by `cargo xtask check` and by the conformance tests.
//!
//! Each check is a pure function over file contents (or `cargo metadata` JSON), so its own
//! tests can show it rejecting a violating input and accepting a conforming one.

use crate::lex::{self, Kind};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub check: &'static str,
    pub location: String,
    pub message: String,
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.check, self.location, self.message)
    }
}

/// A source file, path relative to the repository root.
pub struct File<'a> {
    pub path: &'a str,
    pub text: &'a str,
}

/// The only crate that may hold a write-capable database handle (T1).
pub const LEDGER_DIR: &str = "crates/abw-ledger/";
/// The only crate that may reference Lattice dimension names (T16).
pub const LATTICE_DIR: &str = "crates/abw-lattice/";
/// The outbound guard (T18, ADR-0008 item 8): `egress.rs` or `egress/` in abw-agent. The only
/// product code that may construct an outbound HTTP client or open an outbound socket.
pub const EGRESS_PREFIX: &str = "crates/abw-agent/src/egress";

/// Crates that only one workspace member may depend on (normal and build dependencies).
/// A prefix ending in `*` matches any crate name starting with it.
pub const DEPENDENCY_RULES: &[(&str, &str, &str)] = &[
    (
        "sqlx",
        "abw-ledger",
        "T1: only abw-ledger holds a database handle",
    ),
    (
        "sqlx-*",
        "abw-ledger",
        "T1: only abw-ledger holds a database handle",
    ),
    (
        "tokio-postgres",
        "abw-ledger",
        "T1: only abw-ledger holds a database handle",
    ),
    (
        "postgres",
        "abw-ledger",
        "T1: only abw-ledger holds a database handle",
    ),
    ("nostr*", "abw-acn", "ECO-9: only abw-acn depends on Nostr"),
    ("cashu*", "abw-acn", "ECO-9: only abw-acn depends on Cashu"),
    ("cdk*", "abw-acn", "ECO-9: only abw-acn depends on Cashu"),
    (
        "acn-*",
        "abw-acn",
        "ECO-9: only abw-acn depends on ACN crates",
    ),
    (
        "reqwest",
        "abw-agent",
        "T18: outbound HTTP goes only through abw-agent's egress guard",
    ),
    (
        "ureq",
        "abw-agent",
        "T18: outbound HTTP goes only through abw-agent's egress guard",
    ),
    (
        "isahc",
        "abw-agent",
        "T18: outbound HTTP goes only through abw-agent's egress guard",
    ),
    (
        "surf",
        "abw-agent",
        "T18: outbound HTTP goes only through abw-agent's egress guard",
    ),
    (
        "attohttpc",
        "abw-agent",
        "T18: outbound HTTP goes only through abw-agent's egress guard",
    ),
];

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    id: String,
    dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
struct Dependency {
    name: String,
    #[serde(default)]
    kind: Option<String>,
}

fn matches_rule(pattern: &str, name: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => pattern == name,
    }
}

/// T1, ECO-9: restricted crates are depended on only by their one permitted member.
/// Dev-dependencies are exempt: tests may hold an admin connection.
pub fn dependencies(metadata_json: &str) -> Result<Vec<Finding>, String> {
    let meta: Metadata =
        serde_json::from_str(metadata_json).map_err(|e| format!("cargo metadata: {e}"))?;
    let mut out = Vec::new();
    for pkg in meta
        .packages
        .iter()
        .filter(|p| meta.workspace_members.contains(&p.id))
    {
        for dep in &pkg.dependencies {
            if dep.kind.as_deref() == Some("dev") {
                continue;
            }
            for (pattern, allowed, why) in DEPENDENCY_RULES {
                if matches_rule(pattern, &dep.name) && pkg.name != *allowed {
                    out.push(Finding {
                        check: "dependencies",
                        location: format!("{}/Cargo.toml", pkg.name),
                        message: format!("depends on `{}`; {why}", dep.name),
                    });
                }
            }
        }
    }
    Ok(out)
}

fn is_product_source(path: &str) -> bool {
    path.starts_with("crates/") && path.contains("/src/") && path.ends_with(".rs")
}

/// Word sequences that make a string literal a write statement.
fn write_statement(words: &[String]) -> Option<String> {
    let w = |i: usize| words.get(i).map(String::as_str).unwrap_or("");
    for i in 0..words.len() {
        let found = match w(i) {
            "insert" | "merge" if w(i + 1) == "into" => true,
            "delete" if w(i + 1) == "from" => true,
            "update" => (i + 2..(i + 5).min(words.len())).any(|j| w(j) == "set"),
            "truncate" if w(i + 1) == "table" => true,
            "alter" | "drop" | "create" => matches!(
                w(i + 1),
                "table" | "schema" | "function" | "trigger" | "role" | "index"
            ),
            "grant" | "revoke" => matches!(
                w(i + 1),
                "select"
                    | "insert"
                    | "update"
                    | "delete"
                    | "truncate"
                    | "all"
                    | "usage"
                    | "execute"
            ),
            _ => false,
        };
        if found {
            return Some(words[i..(i + 4).min(words.len())].join(" "));
        }
    }
    None
}

/// T1, T3: no SQL write statement in product code outside `abw-ledger`.
pub fn write_sql(files: &[File<'_>]) -> Vec<Finding> {
    let mut out = Vec::new();
    for f in files
        .iter()
        .filter(|f| is_product_source(f.path) && !f.path.starts_with(LEDGER_DIR))
    {
        for seg in lex::segments(f.text)
            .into_iter()
            .filter(|s| s.kind == Kind::Str)
        {
            if let Some(stmt) = write_statement(&lex::words(&seg.text)) {
                out.push(Finding {
                    check: "write-sql",
                    location: format!("{}:{}", f.path, seg.line),
                    message: format!(
                        "SQL write `{stmt} ...` outside abw-ledger; append events through a command"
                    ),
                });
            }
        }
    }
    out
}

/// One permitted `spawn(` site: `path` may contain `count` of them, for `reason`.
#[derive(Debug, Deserialize)]
pub struct SpawnSite {
    pub path: String,
    pub count: usize,
    pub reason: String,
    pub class: String,
}

#[derive(Debug, Deserialize)]
pub struct SpawnAllowlist {
    #[serde(default, rename = "site")]
    pub sites: Vec<SpawnSite>,
}

/// T5: every task spawn in product code is allowlisted as `non_trust` with a reason.
pub fn spawn_sites(files: &[File<'_>], allow: &SpawnAllowlist) -> Vec<Finding> {
    let mut out = Vec::new();
    for site in &allow.sites {
        if site.class != "non_trust" || site.reason.trim().is_empty() {
            out.push(Finding {
                check: "spawn",
                location: format!("xtask/spawn-allowlist.toml ({})", site.path),
                message: "every allowlisted spawn must be class \"non_trust\" with a reason".into(),
            });
        }
    }
    for f in files.iter().filter(|f| is_product_source(f.path)) {
        let mut lines = Vec::new();
        for seg in lex::segments(f.text)
            .into_iter()
            .filter(|s| s.kind == Kind::Code)
        {
            let chars: Vec<char> = seg.text.chars().collect();
            let mut i = 0;
            while let Some(pos) = find(&chars, i, "spawn") {
                let before_ok =
                    pos == 0 || !(chars[pos - 1].is_alphanumeric() || chars[pos - 1] == '_');
                let mut j = pos + 5;
                while chars.get(j).is_some_and(|c| c.is_whitespace()) {
                    j += 1;
                }
                if before_ok && chars.get(j) == Some(&'(') {
                    lines.push(lex::line_at(&seg, pos));
                }
                i = pos + 5;
            }
        }
        let allowed = allow
            .sites
            .iter()
            .filter(|s| s.path == f.path && s.class == "non_trust")
            .map(|s| s.count)
            .sum::<usize>();
        if lines.len() > allowed {
            out.push(Finding {
                check: "spawn",
                location: format!("{}:{}", f.path, lines.first().copied().unwrap_or(0)),
                message: format!(
                    "{} spawn site(s), {} allowlisted (lines {:?}); a trust effect must not run in a detached task",
                    lines.len(),
                    allowed,
                    lines
                ),
            });
        }
    }
    out
}

fn find(chars: &[char], from: usize, needle: &str) -> Option<usize> {
    let n: Vec<char> = needle.chars().collect();
    (from..chars.len().saturating_sub(n.len() - 1)).find(|&i| chars[i..i + n.len()] == n[..])
}

pub const DIMENSIONS: &[&str] = &["presence", "liveness", "truth", "grounding", "binding"];

/// T16: the five Lattice dimension names appear in product code (identifiers and strings,
/// not comments) and SQL only inside `abw-lattice`.
pub fn vocabulary(files: &[File<'_>]) -> Vec<Finding> {
    let mut out = Vec::new();
    for f in files {
        let in_scope = (f.path.starts_with("crates/") && !f.path.contains("/tests/"))
            && (f.path.ends_with(".rs") || f.path.ends_with(".sql"))
            && !f.path.starts_with(LATTICE_DIR);
        if !in_scope {
            continue;
        }
        let segs = if f.path.ends_with(".sql") {
            sql_segments(f.text)
        } else {
            lex::segments(f.text)
        };
        for seg in segs.iter().filter(|s| s.kind != Kind::Comment) {
            for word in lex::words(&seg.text) {
                if DIMENSIONS.contains(&word.as_str()) {
                    out.push(Finding {
                        check: "vocabulary",
                        location: format!("{}:{}", f.path, seg.line),
                        message: format!(
                            "`{word}` names a Lattice dimension; use it only in abw-lattice"
                        ),
                    });
                }
            }
        }
    }
    out
}

/// SQL: `--` comments are comments, everything else counts.
fn sql_segments(text: &str) -> Vec<lex::Segment> {
    text.lines()
        .enumerate()
        .map(|(i, l)| {
            let code = l.split("--").next().unwrap_or("");
            lex::Segment {
                kind: Kind::Code,
                line: i + 1,
                text: code.to_string(),
            }
        })
        .collect()
}

/// `cargo metadata --format-version 1 --no-deps` for the workspace at `root`.
pub fn cargo_metadata(root: &std::path::Path) -> Result<String, String> {
    let out = std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .current_dir(root)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .map_err(|e| format!("cargo metadata: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "cargo metadata: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("cargo metadata: {e}"))
}

/// Every check over the repository at `root`. `metadata_json` is `cargo metadata
/// --format-version 1 --no-deps` for the workspace.
pub fn repo(root: &std::path::Path, metadata_json: &str) -> Result<Vec<Finding>, String> {
    let mut paths = Vec::new();
    collect(&root.join("crates"), &mut paths)?;
    let mut texts = Vec::new();
    for p in &paths {
        let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        let rel = p
            .strip_prefix(root)
            .unwrap_or(p)
            .to_string_lossy()
            .replace('\\', "/");
        texts.push((rel, text));
    }
    let files: Vec<File<'_>> = texts
        .iter()
        .map(|(path, text)| File { path, text })
        .collect();
    let allow_path = root.join("xtask/spawn-allowlist.toml");
    let allow: SpawnAllowlist = toml::from_str(
        &std::fs::read_to_string(&allow_path)
            .map_err(|e| format!("{}: {e}", allow_path.display()))?,
    )
    .map_err(|e| format!("spawn-allowlist.toml: {e}"))?;

    let mut out = dependencies(metadata_json)?;
    out.extend(write_sql(&files));
    out.extend(spawn_sites(&files, &allow));
    out.extend(vocabulary(&files));
    out.extend(probe_isolation(&files));
    out.extend(egress_clients(&files));
    out.extend(sync_decide(&files));
    Ok(out)
}

/// The text with every string literal and comment blanked to spaces, newlines kept, so that
/// byte offsets map to the same lines as the original.
pub fn code_only(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for seg in lex::segments(text) {
        match seg.kind {
            Kind::Code => out.push_str(&seg.text),
            Kind::Str | Kind::Comment => {
                // Keep the quotes' place so adjacent code tokens don't merge.
                out.push(' ');
                for c in seg.text.chars() {
                    out.push(if c == '\n' { '\n' } else { ' ' });
                }
                out.push(' ');
            }
        }
    }
    out
}

fn line_of(text: &str, byte: usize) -> usize {
    text[..byte.min(text.len())].matches('\n').count() + 1
}

/// Constructors of outbound HTTP clients and sockets, matched with whitespace removed.
const OUTBOUND_ALWAYS: &[&str] = &[
    "reqwest::Client::new",
    "reqwest::Client::builder",
    "reqwest::ClientBuilder",
    "reqwest::get(",
    "reqwest::blocking",
    "hyper::client",
    "hyper_util::client",
    "ureq::",
    "isahc::",
    "surf::",
    "attohttpc::",
    "curl::",
    "TcpStream::connect",
    "UdpSocket::connect",
];

/// Constructors that are reqwest's once the file imports reqwest.
const OUTBOUND_WITH_REQWEST: &[&str] = &["Client::new(", "Client::builder(", "ClientBuilder::new("];

/// T18 (ADR-0008 item 11): only the egress guard constructs outbound HTTP clients or opens
/// outbound sockets in product code.
pub fn egress_clients(files: &[File<'_>]) -> Vec<Finding> {
    let mut out = Vec::new();
    for f in files
        .iter()
        .filter(|f| is_product_source(f.path) && !f.path.starts_with(EGRESS_PREFIX))
    {
        let code = code_only(f.text);
        let uses_reqwest = code
            .lines()
            .map(|l| l.chars().filter(|c| !c.is_whitespace()).collect::<String>())
            .any(|l| l.starts_with("usereqwest") || l.starts_with("pubusereqwest"));
        for (n, line) in code.lines().enumerate() {
            let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
            let mut hits: Vec<&str> = OUTBOUND_ALWAYS
                .iter()
                .copied()
                .filter(|p| compact.contains(p))
                .collect();
            if uses_reqwest {
                hits.extend(
                    OUTBOUND_WITH_REQWEST
                        .iter()
                        .copied()
                        .filter(|p| compact.contains(p)),
                );
            }
            if !hits.is_empty() {
                out.push(Finding {
                    check: "egress",
                    location: format!("{}:{}", f.path, n + 1),
                    message: format!(
                        "`{}` outside the egress guard; outbound calls go through abw_agent::egress (T18)",
                        hits.join("`, `")
                    ),
                });
            }
        }
    }
    out
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// SUB-06 (ADR-0008 item 11): a `fn decide` is synchronous: not `async`, no `Future` in its
/// signature, no `.await` in its body.
pub fn sync_decide(files: &[File<'_>]) -> Vec<Finding> {
    let mut out = Vec::new();
    for f in files.iter().filter(|f| is_product_source(f.path)) {
        let code = code_only(f.text);
        let bytes = code.as_bytes();
        let mut from = 0;
        while let Some(rel) = code[from..].find("fn") {
            let at = from + rel;
            from = at + 2;
            let before_ok = at == 0 || !is_ident_char(code[..at].chars().last().unwrap_or(' '));
            let rest = &code[at + 2..];
            let trimmed = rest.trim_start();
            if !before_ok || trimmed.len() == rest.len() || !trimmed.starts_with("decide") {
                continue;
            }
            let after_name = &trimmed["decide".len()..];
            if after_name.starts_with(is_ident_char) {
                continue;
            }
            let line = line_of(&code, at);
            let mut push = |message: &str| {
                out.push(Finding {
                    check: "sync-decide",
                    location: format!("{}:{line}", f.path),
                    message: format!("{message}; decide must be a pure, synchronous fn (SUB-06)"),
                });
            };
            // The words before `fn` on the same item: `pub`, `pub(crate)`, `async`, `const` ...
            let line_start = code[..at].rfind(['\n', ';', '}', '{']).map_or(0, |i| i + 1);
            let prefix: Vec<String> = lex::words(&code[line_start..at]);
            if prefix.iter().any(|w| w == "async") {
                push("`async fn decide`");
            }
            // Signature: up to the body's `{` or a `;` (a trait declaration).
            let sig_end = code[at..].find(['{', ';']).map_or(code.len(), |i| at + i);
            let signature = &code[at..sig_end];
            if lex::words(signature).iter().any(|w| w == "future") {
                push("`decide` returns a Future");
            }
            if bytes.get(sig_end) == Some(&b'{') {
                let mut depth = 0usize;
                let mut end = code.len();
                for (i, c) in code[sig_end..].char_indices() {
                    match c {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                end = sig_end + i;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let body: String = code[sig_end..end]
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect();
                if body.contains(".await") {
                    push("`.await` inside `decide`");
                }
            }
        }
    }
    out
}

fn collect(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            collect(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs" || e == "sql") {
            out.push(path);
        }
    }
    Ok(())
}

/// ADR-0004: the conformance probe is used only by abw-ledger itself and by tests.
pub fn probe_isolation(files: &[File<'_>]) -> Vec<Finding> {
    let mut out = Vec::new();
    for f in files
        .iter()
        .filter(|f| is_product_source(f.path) && !f.path.starts_with(LEDGER_DIR))
    {
        for seg in lex::segments(f.text)
            .into_iter()
            .filter(|s| s.kind == Kind::Code)
        {
            for (n, line) in seg.text.lines().enumerate() {
                let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
                if !compact.contains("abw_ledger::probe") {
                    continue;
                }
                out.push(Finding {
                    check: "probe",
                    location: format!("{}:{}", f.path, seg.line + n),
                    message:
                        "abw_ledger::probe is a conformance fixture; domain code must not use it"
                            .into(),
                });
            }
        }
    }
    out
}
