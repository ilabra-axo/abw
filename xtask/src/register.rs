//! The requirement register: `docs/requirements/register.toml`.

use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize)]
pub struct Register {
    pub meta: Meta,
    #[serde(rename = "req", default)]
    pub reqs: Vec<Req>,
}

#[derive(Debug, Deserialize)]
pub struct Meta {
    /// The milestone in progress. Requirements due at or before it must be met.
    pub current: String,
    /// Milestones in order.
    pub milestones: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Req {
    pub id: String,
    pub title: String,
    pub text: String,
    pub source: String,
    pub milestone: String,
    #[serde(default)]
    pub note: Option<String>,
}

impl Register {
    pub fn parse(text: &str) -> Result<Self, String> {
        let reg: Register = toml::from_str(text).map_err(|e| format!("register: {e}"))?;
        reg.validate()?;
        Ok(reg)
    }

    /// Structural rules (GOV-03). Returns every problem, not just the first.
    fn validate(&self) -> Result<(), String> {
        let mut problems = Vec::new();
        if !self.meta.milestones.contains(&self.meta.current) {
            problems.push(format!(
                "meta.current {:?} is not a declared milestone",
                self.meta.current
            ));
        }
        let mut seen = BTreeSet::new();
        for r in &self.reqs {
            if !seen.insert(r.id.as_str()) {
                problems.push(format!("duplicate id {}", r.id));
            }
            if !self.meta.milestones.contains(&r.milestone) {
                problems.push(format!("{}: unknown milestone {:?}", r.id, r.milestone));
            }
            for (field, value) in [
                ("title", &r.title),
                ("text", &r.text),
                ("source", &r.source),
            ] {
                if value.trim().is_empty() {
                    problems.push(format!("{}: empty {field}", r.id));
                }
            }
            if r.id.trim().is_empty() || r.id.contains(char::is_whitespace) {
                problems.push(format!("invalid id {:?}", r.id));
            }
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "register is malformed:\n  {}",
                problems.join("\n  ")
            ))
        }
    }

    /// Position of a milestone in the declared order.
    pub fn rank(&self, milestone: &str) -> Option<usize> {
        self.meta.milestones.iter().position(|m| m == milestone)
    }

    /// True when the requirement is due at or before the current milestone.
    pub fn is_due(&self, req: &Req) -> bool {
        match (self.rank(&req.milestone), self.rank(&self.meta.current)) {
            (Some(r), Some(c)) => r <= c,
            _ => true,
        }
    }

    pub fn contains(&self, id: &str) -> bool {
        self.reqs.iter().any(|r| r.id == id)
    }
}
