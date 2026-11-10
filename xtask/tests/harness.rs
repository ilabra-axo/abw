//! Tests for the harness itself. Each check is shown rejecting a violating input and
//! accepting a conforming one; a check never seen to reject is not a check.

use std::collections::BTreeMap;
use xtask::register::Register;
use xtask::results::{self, Outcome};
use xtask::scope;
use xtask::tags;
use xtask::trace::{self, Status};

const REG: &str = r#"
[meta]
current = "M1"
milestones = ["M0", "M1", "M2"]

[[req]]
id = "A-1"
title = "due and tested"
text = "t"
source = "s"
milestone = "M0"

[[req]]
id = "A-2"
title = "due, untested"
text = "t"
source = "s"
milestone = "M1"

[[req]]
id = "A-3"
title = "not yet due"
text = "t"
source = "s"
milestone = "M2"
"#;

fn reg() -> Register {
    match Register::parse(REG) {
        Ok(r) => r,
        Err(e) => panic!("fixture register should parse: {e}"),
    }
}

fn outcomes(pairs: &[(&str, Outcome)]) -> BTreeMap<String, Vec<Outcome>> {
    pairs
        .iter()
        .map(|(n, o)| (n.to_string(), vec![*o]))
        .collect()
}

fn status_of(report: &trace::Report, id: &str) -> Status {
    match report.rows.iter().find(|r| r.id == id) {
        Some(r) => r.status,
        None => panic!("no row for {id}"),
    }
}

// Single-line so the repository's own tag scan does not read these fixture tags.
const TESTS_SRC: &str = "// @req A-1\n#[test]\nfn proves_a1() {}\n\n// @req A-3\n#[tokio::test]\nasync fn proves_a3_early() {}\n";

// @req GOV-01
#[test]
fn trace_fails_when_a_due_requirement_has_no_test() {
    let scan = tags::scan([("tests/x.rs", TESTS_SRC)]);
    let res = outcomes(&[
        ("proves_a1", Outcome::Passed),
        ("proves_a3_early", Outcome::Passed),
    ]);
    let report = trace::evaluate(&reg(), &scan, Some(&res));
    assert_eq!(status_of(&report, "A-1"), Status::Met);
    assert_eq!(status_of(&report, "A-2"), Status::Untested);
    assert!(!report.ok(), "A-2 is due and untested, so trace must fail");
}

// @req GOV-01
#[test]
fn trace_passes_when_every_due_requirement_is_met() {
    let src = format!("{TESTS_SRC}\n// @req A-2\n#[test]\nfn proves_a2() {{}}\n");
    let scan = tags::scan([("tests/x.rs", src.as_str())]);
    let res = outcomes(&[
        ("proves_a1", Outcome::Passed),
        ("proves_a2", Outcome::Passed),
        ("proves_a3_early", Outcome::Passed),
    ]);
    let report = trace::evaluate(&reg(), &scan, Some(&res));
    assert!(report.ok(), "{:?}", report.blocking());
    assert_eq!(
        status_of(&report, "A-3"),
        Status::Met,
        "met early is shown as met"
    );
}

// @req GOV-01
#[test]
fn trace_fails_when_a_linked_test_fails_or_did_not_run() {
    let scan = tags::scan([("tests/x.rs", TESTS_SRC)]);
    let failed = outcomes(&[
        ("proves_a1", Outcome::Failed),
        ("proves_a3_early", Outcome::Passed),
    ]);
    assert_eq!(
        status_of(&trace::evaluate(&reg(), &scan, Some(&failed)), "A-1"),
        Status::Failing
    );

    let missing = outcomes(&[("proves_a3_early", Outcome::Passed)]);
    assert_eq!(
        status_of(&trace::evaluate(&reg(), &scan, Some(&missing)), "A-1"),
        Status::Failing
    );

    // A not-yet-due test that does not run yet (e.g. compiled out until its API exists) is planned.
    let not_built = outcomes(&[("proves_a1", Outcome::Passed)]);
    let report = trace::evaluate(&reg(), &scan, Some(&not_built));
    assert_eq!(status_of(&report, "A-3"), Status::Planned);
    assert!(!report.blocking().iter().any(|r| r.id == "A-3"));

    let early_fail = outcomes(&[
        ("proves_a1", Outcome::Passed),
        ("proves_a3_early", Outcome::Failed),
    ]);
    let report = trace::evaluate(&reg(), &scan, Some(&early_fail));
    assert_eq!(status_of(&report, "A-3"), Status::Failing);
    assert!(
        report.blocking().iter().any(|r| r.id == "A-3"),
        "a failing test blocks even before it is due"
    );
}

// @req GOV-02
#[test]
fn unknown_requirement_tag_is_a_problem() {
    let src = "// @req NOPE-9\n#[test]\nfn tagged_wrong() {}\n";
    let scan = tags::scan([("tests/y.rs", src)]);
    let report = trace::evaluate(
        &reg(),
        &scan,
        Some(&outcomes(&[("tagged_wrong", Outcome::Passed)])),
    );
    assert!(
        report.problems.iter().any(|p| p.contains("NOPE-9")),
        "{:?}",
        report.problems
    );
    assert!(!report.ok());
}

// @req GOV-02
#[test]
fn tag_not_on_a_test_and_duplicate_names_are_problems() {
    let src = "// @req A-1\nfn helper() {}\n\n// @req A-1\n#[test]\nfn same() {}\n";
    let other = "// @req A-1\n#[test]\nfn same() {}\n";
    let scan = tags::scan([("tests/a.rs", src), ("tests/b.rs", other)]);
    assert!(
        scan.problems
            .iter()
            .any(|p| p.contains("not followed by a test")),
        "{:?}",
        scan.problems
    );
    assert!(
        scan.problems.iter().any(|p| p.contains("not unique")),
        "{:?}",
        scan.problems
    );
}

// @req GOV-02
#[test]
fn tags_bind_through_attributes_and_doc_comments() {
    let src = "// @req A-1, A-2\n/// doc\n#[sqlx::test]\n#[ignore = \"pending M1\"]\nasync fn pending_one() {}\n";
    let scan = tags::scan([("tests/z.rs", src)]);
    assert!(scan.problems.is_empty(), "{:?}", scan.problems);
    assert_eq!(scan.tests.len(), 1);
    assert_eq!(
        scan.tests[0].ids,
        vec!["A-1".to_string(), "A-2".to_string()]
    );
    assert!(scan.tests[0].ignored);
}

// @req GOV-03
#[test]
fn malformed_registers_are_rejected() {
    let dup = REG.replace("id = \"A-2\"", "id = \"A-1\"");
    let bad_ms = REG.replace("milestone = \"M2\"", "milestone = \"M9\"");
    let empty = REG.replace("title = \"not yet due\"", "title = \"  \"");
    let bad_current = REG.replace("current = \"M1\"", "current = \"Q3\"");
    for (name, text) in [
        ("duplicate", dup),
        ("milestone", bad_ms),
        ("empty", empty),
        ("current", bad_current),
    ] {
        assert!(
            Register::parse(&text).is_err(),
            "{name} register should be rejected"
        );
    }
    assert!(Register::parse(REG).is_ok());
}

// @req GOV-04
#[test]
fn pr_scope_rejects_mixed_changes() {
    let changed = ["crates/abw-ledger/src/append.rs", "tests/conformance/t2.rs"];
    let diff = "@@ -1 +1 @@\n-    assert!(x);\n+    assert!(true);\n";
    let err = scope::check(&changed, &[("tests/conformance/t2.rs", diff)]);
    assert!(err.is_err(), "mixed PR must be rejected");
}

// @req GOV-04
#[test]
fn pr_scope_accepts_pure_prs_and_unignoring_pending_tests() {
    assert!(scope::check(&["crates/abw-ledger/src/append.rs", "docs/PLAN.md"], &[]).is_ok());
    assert!(
        scope::check(
            &["tests/conformance/t2.rs", "docs/requirements/register.toml"],
            &[]
        )
        .is_ok()
    );

    let unignore = "@@ -4 +3,0 @@\n-#[ignore = \"pending M1: abw-ledger\"]\n";
    let changed = [
        "crates/abw-ledger/src/append.rs",
        "crates/abw-ledger/tests/t2.rs",
    ];
    assert!(scope::check(&changed, &[("crates/abw-ledger/tests/t2.rs", unignore)]).is_ok());

    let sneaky = "@@ -4 +3,0 @@\n-#[ignore = \"pending M1\"]\n-    assert_eq!(a, b);\n";
    assert!(scope::check(&changed, &[("crates/abw-ledger/tests/t2.rs", sneaky)]).is_err());
}

// @req GOV-04
#[test]
fn path_classification() {
    use scope::Kind::*;
    for (p, k) in [
        ("crates/abw-ledger/src/lib.rs", Product),
        ("migrations/0001_ledger.sql", Product),
        ("crates/abw-ledger/migrations/0001_ledger.sql", Product),
        ("crates/abw-ledger/Cargo.toml", Neutral),
        ("crates/abw-ledger/tests/append.rs", Verification),
        ("tests/conformance/t1.rs", Verification),
        ("xtask/src/main.rs", Verification),
        ("docs/requirements/register.toml", Verification),
        (".github/workflows/ci.yml", Verification),
        ("docs/design/economy.md", Neutral),
        ("Cargo.lock", Neutral),
    ] {
        assert_eq!(scope::classify(p), k, "{p}");
    }
}

// @req GOV-05
#[test]
fn pending_tests_never_count_as_evidence() {
    let src = "// @req A-1\n#[test]\n#[ignore = \"pending\"]\nfn later() {}\n";
    let scan = tags::scan([("tests/p.rs", src)]);
    let report = trace::evaluate(
        &reg(),
        &scan,
        Some(&outcomes(&[("later", Outcome::Ignored)])),
    );
    assert_eq!(status_of(&report, "A-1"), Status::Pending);
    assert!(!report.ok());
}

// @req GOV-05
#[test]
fn libtest_output_is_parsed() {
    let out = "running 3 tests\ntest a::b::one ... ok\ntest two ... FAILED\ntest c::three ... ignored, pending M1\n";
    let r = results::parse(out);
    assert_eq!(r.get("one"), Some(&vec![Outcome::Passed]));
    assert_eq!(r.get("two"), Some(&vec![Outcome::Failed]));
    assert_eq!(r.get("three"), Some(&vec![Outcome::Ignored]));
}
