//! Joins the register, the tagged tests and the test results into a verdict per requirement.

use crate::register::Register;
use crate::results::Outcome;
use crate::tags::Scan;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    /// Every linked test ran and passed, and at least one exists.
    Met,
    /// Not yet due; shown for visibility.
    Planned,
    /// Due, and no linked test exists.
    Untested,
    /// Due, and every linked test is pending (`#[ignore]`).
    Pending,
    /// Due, and a linked test failed, was ignored, or did not run.
    Failing,
}

#[derive(Debug)]
pub struct Row {
    pub id: String,
    pub title: String,
    pub milestone: String,
    pub due: bool,
    pub status: Status,
    pub tests: Vec<(String, &'static str)>,
}

#[derive(Debug)]
pub struct Report {
    pub rows: Vec<Row>,
    pub problems: Vec<String>,
    pub current: String,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.problems.is_empty() && self.blocking().is_empty()
    }

    pub fn blocking(&self) -> Vec<&Row> {
        self.rows
            .iter()
            .filter(|r| r.status == Status::Failing || (r.due && r.status != Status::Met))
            .collect()
    }
}

/// `results` is `None` when tests were not run (`--list`): due requirements then report
/// their tests' presence only and the report is never ok.
pub fn evaluate(
    reg: &Register,
    scan: &Scan,
    results: Option<&BTreeMap<String, Vec<Outcome>>>,
) -> Report {
    let mut problems = scan.problems.clone();
    for t in &scan.tests {
        for id in &t.ids {
            if !reg.contains(id) {
                problems.push(format!(
                    "{}:{}: test `{}` is tagged with unknown requirement `{id}`",
                    t.file, t.line, t.name
                ));
            }
        }
    }

    let mut rows = Vec::new();
    for req in &reg.reqs {
        let due = reg.is_due(req);
        let linked: Vec<_> = scan
            .tests
            .iter()
            .filter(|t| t.ids.iter().any(|i| i == &req.id))
            .collect();
        let mut tests = Vec::new();
        let mut any_bad = false;
        let mut any_missing = false;
        let mut all_pending = !linked.is_empty();
        for t in &linked {
            let label = if t.ignored {
                "pending"
            } else {
                all_pending = false;
                match results.map(|r| r.get(&t.name)) {
                    None => "not run",
                    Some(None) => {
                        any_missing = true;
                        "did not run"
                    }
                    Some(Some(outs)) if outs.iter().all(|o| *o == Outcome::Passed) => "passed",
                    Some(Some(outs)) if outs.contains(&Outcome::Failed) => {
                        any_bad = true;
                        "failed"
                    }
                    Some(Some(_)) => {
                        any_bad = true;
                        "ignored"
                    }
                }
            };
            tests.push((format!("{} ({}:{})", t.name, t.file, t.line), label));
        }
        // A failing active test fails the trace even before its requirement is due; a
        // requirement met early is shown as Met.
        let status = if any_bad || (due && any_missing) {
            Status::Failing
        } else if any_missing {
            Status::Planned
        } else if linked.is_empty() {
            if due {
                Status::Untested
            } else {
                Status::Planned
            }
        } else if all_pending {
            if due {
                Status::Pending
            } else {
                Status::Planned
            }
        } else if results.is_none() {
            if due {
                Status::Failing
            } else {
                Status::Planned
            }
        } else {
            Status::Met
        };
        rows.push(Row {
            id: req.id.clone(),
            title: req.title.clone(),
            milestone: req.milestone.clone(),
            due,
            status,
            tests,
        });
    }
    Report {
        rows,
        problems,
        current: reg.meta.current.clone(),
    }
}

pub fn markdown(report: &Report, reg: &Register) -> String {
    let mut s = format!(
        "## Requirement trace (current milestone {})\n\n",
        report.current
    );
    let mut counts: BTreeMap<String, [usize; 2]> = BTreeMap::new();
    for r in &report.rows {
        let c = counts.entry(r.milestone.clone()).or_default();
        c[1] += 1;
        if r.status == Status::Met {
            c[0] += 1;
        }
    }
    s.push_str("| Milestone | Met | Total |\n| --- | --- | --- |\n");
    for m in &reg.meta.milestones {
        if let Some([met, total]) = counts.get(m) {
            s.push_str(&format!("| {m} | {met} | {total} |\n"));
        }
    }
    s.push_str("\n| ID | Milestone | Status | Title | Tests |\n| --- | --- | --- | --- | --- |\n");
    for r in &report.rows {
        let tests = if r.tests.is_empty() {
            "—".to_string()
        } else {
            r.tests
                .iter()
                .map(|(t, l)| format!("`{t}` {l}"))
                .collect::<Vec<_>>()
                .join("<br>")
        };
        s.push_str(&format!(
            "| {} | {} | {:?} | {} | {} |\n",
            r.id, r.milestone, r.status, r.title, tests
        ));
    }
    if !report.problems.is_empty() {
        s.push_str("\n### Problems\n\n");
        for p in &report.problems {
            s.push_str(&format!("- {p}\n"));
        }
    }
    s
}
