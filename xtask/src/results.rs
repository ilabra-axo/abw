//! Parses libtest's human output (`test path::name ... ok`).

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Passed,
    Failed,
    Ignored,
}

/// Maps the last path segment of each test to its outcomes. A name can appear more than
/// once (e.g. unit and integration targets); every occurrence is kept.
pub fn parse(output: &str) -> BTreeMap<String, Vec<Outcome>> {
    let mut map: BTreeMap<String, Vec<Outcome>> = BTreeMap::new();
    for line in output.lines() {
        let Some(rest) = line.strip_prefix("test ") else {
            continue;
        };
        let Some((path, status)) = rest.rsplit_once(" ... ") else {
            continue;
        };
        let outcome = match status.trim() {
            "ok" => Outcome::Passed,
            "FAILED" => Outcome::Failed,
            s if s.starts_with("ignored") => Outcome::Ignored,
            _ => continue,
        };
        let path = path.trim();
        let name = path.rsplit("::").next().unwrap_or(path);
        map.entry(name.to_string()).or_default().push(outcome);
    }
    map
}
