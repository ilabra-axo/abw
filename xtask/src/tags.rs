//! Finds tests tagged with requirement IDs.
//!
//! A tag is a line comment `// @req ID[, ID ...]` placed above a test function. It binds to
//! the next `fn` after a `#[test]`-style attribute (`#[test]`, `#[tokio::test]`,
//! `#[sqlx::test]`, ...). Attributes and doc comments may sit between the tag and the `fn`.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedTest {
    /// Function name; must be unique among tagged tests so results can be matched.
    pub name: String,
    pub file: String,
    pub line: usize,
    pub ids: Vec<String>,
    /// The test carries `#[ignore]`: it is pending and is never evidence.
    pub ignored: bool,
}

#[derive(Debug, Default)]
pub struct Scan {
    pub tests: Vec<TaggedTest>,
    pub problems: Vec<String>,
}

/// Scans `(path, contents)` pairs.
pub fn scan<'a>(files: impl IntoIterator<Item = (&'a str, &'a str)>) -> Scan {
    let mut out = Scan::default();
    for (path, text) in files {
        scan_file(path, text, &mut out);
    }
    let mut by_name: BTreeMap<&str, Vec<&TaggedTest>> = BTreeMap::new();
    for t in &out.tests {
        by_name.entry(t.name.as_str()).or_default().push(t);
    }
    let dupes: Vec<String> = by_name
        .iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(name, v)| {
            let sites: Vec<String> = v.iter().map(|t| format!("{}:{}", t.file, t.line)).collect();
            format!(
                "tagged test name `{name}` is not unique: {}",
                sites.join(", ")
            )
        })
        .collect();
    out.problems.extend(dupes);
    out
}

fn scan_file(path: &str, text: &str, out: &mut Scan) {
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let Some(ids) = parse_tag(lines[i]) else {
            i += 1;
            continue;
        };
        let tag_line = i + 1;
        if ids.is_empty() {
            out.problems
                .push(format!("{path}:{tag_line}: `@req` with no IDs"));
            i += 1;
            continue;
        }
        let mut saw_test_attr = false;
        let mut ignored = false;
        let mut j = i + 1;
        let mut bound = false;
        while j < lines.len() {
            let l = lines[j].trim();
            if l.starts_with("#[") {
                let attr = l.trim_start_matches("#[");
                if attr.starts_with("test]") || attr.contains("::test") {
                    saw_test_attr = true;
                }
                if attr.starts_with("ignore") {
                    ignored = true;
                }
            } else if l.starts_with("//") || l.is_empty() {
                if parse_tag(l).is_some() {
                    break;
                }
            } else if let Some(name) = fn_name(l) {
                if saw_test_attr {
                    out.tests.push(TaggedTest {
                        name: name.to_string(),
                        file: path.to_string(),
                        line: tag_line,
                        ids: ids.clone(),
                        ignored,
                    });
                    bound = true;
                }
                break;
            } else {
                break;
            }
            j += 1;
        }
        if !bound {
            out.problems.push(format!(
                "{path}:{tag_line}: `@req` is not followed by a test function"
            ));
        }
        i = j.max(i + 1);
    }
}

fn parse_tag(line: &str) -> Option<Vec<String>> {
    let rest = line
        .trim()
        .strip_prefix("//")?
        .trim()
        .strip_prefix("@req")?;
    if !(rest.is_empty() || rest.starts_with(char::is_whitespace)) {
        return None;
    }
    Some(
        rest.split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
    )
}

fn fn_name(line: &str) -> Option<&str> {
    let mut rest = line;
    for prefix in ["pub(crate) ", "pub ", "async ", "unsafe "] {
        rest = rest.strip_prefix(prefix).unwrap_or(rest);
    }
    let rest = rest.strip_prefix("async ").unwrap_or(rest);
    let rest = rest.strip_prefix("fn ")?;
    let end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
    Some(&rest[..end])
}
