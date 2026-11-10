# Instructions for coding agents

You are working on the ABW core. Read `docs/PLAN.md` first, then the documents in its "Sources of truth" table, in rank order. Where sources disagree, the higher rank wins. Code is evidence of current behaviour, never of intended behaviour.

## Hard rules

1. **Only `abw-ledger` writes.** No other crate holds a write-capable database handle or depends on `sqlx` with Postgres. Trust state changes only by appending events through a command. CRUD state changes only through `abw_ledger::crud`.
2. **History is never rewritten.** No `UPDATE` or `DELETE` on events, artifacts, provenance or policy; projections change only in the transaction that appends their event.
3. **A record commits with its effect.** No write-behind, detached task or fire-and-forget for decisions, charges, versions or verdicts.
4. **A control refuses or it doesn't exist.** A check that only records is typed `Observation` and records `Undetermined` with a reason class.
5. **Evidence is not verdict; opinion is not provenance.** Judge output is `interpretive` and never a source.
6. **No value moves between owners inside ABW.** See `docs/design/economy.md`. There are no wallets, transfers or Stripe.
7. **One word, one meaning.** Presence, Liveness, Truth, Grounding and Binding name only Lattice dimensions.
8. **No `unwrap`, `expect` or discarded `Result`s** (workspace lints deny them). Errors are typed.
9. **Hashing is canonical:** JCS (RFC 8785) and `sha256:<hex>` only, from `abw-canon`.
10. **No pattern, crate or dependency the design doesn't name.** If you think one is needed, stop and write an ADR proposal in `docs/decisions/`.

## How work is done

- **Tests before code.** A test PR adds tests tagged `// @req <ID>` (IDs from `docs/requirements/register.toml`). Tests that compile but can't pass yet carry `#[ignore = "pending <milestone>: <reason>"]`. Tests against an API that doesn't exist yet stay out of the build until the API lands (`#![cfg(any())]` at the top of the file, removed in the product PR).
- **Product PRs don't touch verification assets** (`tests/`, `crates/*/tests/`, `docs/requirements/`, `xtask/`, `.github/`). They may only delete `#[ignore = "pending ...` lines (and the `#![cfg(any())]` gate) from tests they make pass. CI enforces this (`cargo xtask pr-scope`).
- **Never weaken, skip, delete or special-case a check to make it pass.** If a check looks wrong, open an issue titled `CHECK-DISPUTE: <id>` and stop work on that item.
- **Every PR names the register IDs it serves** and pastes `cargo xtask trace` output. You never declare anything done; done is CI green on `cargo xtask trace` at the milestone.
- **Cite what you claim.** Comments and docs that describe behaviour cite `path:line`, or say they describe intent. Tag factual claims in PRs [V] verified or [I] inferred.
- **Ambiguity is a question, not a guess.** Don't resolve it by copying what the legacy code does.

## Commands

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask trace            # run tests and check the register
cargo xtask trace --list     # show the matrix without running tests
cargo xtask pr-scope --base origin/main
```
