# Core delta: applying the target system design to a new codebase

Nov 10, 2026 · Status: **normative** (rank 4 together with `target-system-design.pdf`)

The target system design (`target-system-design.pdf`, 180 pages; text extraction in `target-system-design.txt` for search) was written to migrate the legacy `fermi` repository. This core is a new codebase. This document says, section by section, what applies. Where it says **adopted**, the design is normative as written. Where it says **replaced** or **not applicable**, it isn't.

## Rules for reading the target design here

1. `[K] path:line` citations point at the legacy repository (tag-free; source commit `a3cea516` of `fermi`). In this repo they mean "behavioural reference" or "port from", never "keep this file". A kept component is ported into a new crate, with tests, and cited by its new path.
2. "Phase 0/1/2/3" in the design maps to this repo's milestones: Phase 0 → M0 and M2 (T16–T18), Phase 1 → M1, Phase 2 → M3, Phase 3 → M2 (money) and M6 (DPP, all writers). The register's `milestone` field is authoritative.
3. Names. The design's `fermi-http` crate is `abw-http`; the root `fermi` crate doesn't exist. Everything else keeps the design's names (`abw-ledger`, `ReadDb`, `LedgerPool`, `ProjTx`, `read_query!`, `execute_crud!`).
4. §16 open decisions. The **recommendation** of every OD-nn is adopted as the default (ADR-0002), except where this document says otherwise. Changing one needs an ADR.

## Section map

| § | Topic | Status here | Notes |
| --- | --- | --- | --- |
| 0 | Reading guide | Adopted with the ranking in `PLAN.md` §3 | The Lattice spec stays rank 1 |
| 1.1 | Context: trust vs CRUD domain | Adopted | Matches `capabilities.md` "two kinds of state" |
| 1.2 | Invariants T1–T20 | Adopted, **except T9** | T9 is replaced by ECO-1..9 (`economy.md`) |
| 1.3 | Where each invariant becomes true | Replaced by register milestones | |
| 2.1 | Processes, roles `api` / `worker` | Adopted | Binary is `abw-server`; one process with `ABW_ROLE=api+worker` until horizontal scale (OD-01) |
| 2.2 | Tokio runtime, blocking work, no detached trust effects | Adopted | The spawn allowlist test lives in `tests/conformance/` |
| 2.3 | Connection pools `ReadDb` / `LedgerPool` / migration | Adopted | |
| 2.4 | Schemas `ledger`, `prov`, `proj`, `crud` and roles | Adopted, with one change | The CRUD domain uses schema `crud` instead of `public`, so nothing lives in `public` by accident |
| 2.5 | Git and object storage | Adopted when needed | Workspace git arrives with workspaces (M3); object storage when an artifact exceeds 256 KiB |
| 2.6 | Single replica, boot fence | Adopted | The advisory-lock fence ships in M1 |
| 2.7 | Background workers | Adopted per milestone | Workers arrive with the aggregate they serve |
| 3 | The ledger crate | **Adopted in full** | M1. DDL from §3.1 and Appendix D is the starting schema |
| 4 | Agent, versions, tool allowlist, episode pipeline, crash points | Adopted | M2. §4.4 "kept from the executor" becomes "ported from" |
| 5 | Claims and the Verification Lattice | Adopted | M3. The Lattice is the pure crate `abw-lattice`; `evaluate_v1` per OD-29 |
| 6 | Provenance (ΞPROV) | Adopted | M3 |
| 7 | Evidence-based grounding | Adopted | §7.5 (the bridge from today's content proxy, G09) and §7.6 (measure → shadow → enforce) don't apply: grounding is enforced from the first claim |
| 8 | Gates as events | Adopted | M3 |
| 9 | Money | **Replaced** by `economy.md` | No wallets, issuer, Stripe, royalties, sponsorship, debt or reconciliation. OD-24, OD-25, OD-34 and OD-36 lapse |
| 10.1 | Fermi vertical slice | Adopted | M3. `forecast_agent_claims` becomes claim events from the start, not a kept table |
| 10.2 | DPP / carbon | Adopted | M6 |
| 11 | Loops and policy | Adopted | Policy table and T20 in M3; loops in M5 |
| 12.1–12.4 | Tools, evaluators and feeds, apps and MCP, A2A | Adopted per milestone | MCP and A2A in M6 |
| 12.5 | Outbound SSRF guard | Adopted | M2 |
| 12.6 | Contract versioning (G33) | Adopted | Event types are versioned from M1 |
| 13 | Security model | Adopted | Tenancy (T17) and egress (T18) in M2; RLS as defence in depth when CRUD lands (OD-30) |
| 14 | Migration from current state | **Not applicable** | No legacy tables, backfills or strangler sequence. §14.5 CRUD-domain rules **are** adopted |
| 14.6 | CI ratchets | **Not applicable** | Every count starts at zero and is a hard check instead: no `UPDATE`/`DELETE` outside `abw-ledger`, no `let _ =` on a write, no `unwrap`/`expect` (workspace lints) |
| 15 | Conformance test catalogue | **Adopted in full** | Each test gets `// @req T<n>` and lives in `tests/conformance/` or the owning crate's `tests/`. "Fails today because" columns don't apply |
| 16 | Open decisions | Recommendations adopted (ADR-0002) | Lapsed: OD-24, OD-25, OD-34, OD-36 (money). OD-22 and OD-23 don't apply (no legacy cards to backfill; port_trust doesn't exist) |
| App. A, B | Gap closure map, SPEC 36 divergence | Reference only | |
| App. C | Rust → C++ mental model | Reference only | |
| App. D | DDL validation record | Adopted as the test plan for M1's DDL | Cases 35–36 (issuer wallet) lapse with §9 |

## Seams the design leaves open

`capabilities.md` ("Open: where the halves meet") lists five seams. Each gets an ADR before the milestone that needs it:

| Seam | Needed by | Default until decided |
| --- | --- | --- |
| Fidelity vs calibration; Shapley credit | M3 | Both are kept and scored separately: per-claim Brier (T19) for fidelity, exact Shapley credit (PROT-03) for calibration |
| Memory and fitted parameters as trust state | M5 | Accepted BayesOps fits and Loop 1 rule changes are events with provenance; raw memory stays CRUD |
| Coherence's two roles | M5 | The coherence gate is a gate decision; Γ(C) is a recorded observation; cascade writes are events |
| Composition changes | M5 | Owner-accepted composition changes are events (`composition.changed`) |
| Contracts vs claims | M3 | An episode contract produces a claim; it isn't one |
