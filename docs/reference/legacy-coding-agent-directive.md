# ABW — Coding Agent Directive: Lattice as Target

Nov 6, 2026 · @ivan Labra

## 1. Authority: which document wins

The ABW Verification Lattice Specification v0.1 is the design. `ABW_LOW_LEVEL_DESIGN.md` (the LLD) describes what the code does today. When you plan work, the spec decides direction; when you describe current behaviour, the LLD's rule still holds: code wins over prose.

This directive overrides the LLD wherever the LLD reads as a backlog. The gap register (LLD §9) is a list of findings, not a work plan. Section 4 below assigns each finding a disposition. Work issues only in the order and form given here.

The core problem this directive corrects: over the past year, changes passed their tests while the architecture drifted from the design. The drift has five causes, and every rule below targets one of them.

- **The code became the spec.** New code copied the dominant local pattern (direct `UPDATE` through `sqlx`), so the pattern compounded: 303 sites in August 2026, 335 at HEAD.
- **Name collision hid divergence.** The code's ladder (`src/ladder.rs`) reuses the Lattice's five dimension names for different questions. Only Grounding overlaps in meaning.
- **Controls were built as records.** `command_registry.rs` declares commands but routes nothing; `port_trust` is advisory; `schema_validate` records instead of refusing; `InputBinding` records `Refused` while refusing nothing.
- **Findings became metrics.** The August audit was answered with counters (`write_accounting.rs`) rather than integrity.
- **Comments and docs asserted intended behaviour as fact.** Examples: the `credits.rs` PgBouncer atomicity comment; `GROUNDING_SERVICE.md` on tool checks. Later sessions trusted them.

## 2. Stop now, then report

Before picking up the next issue, pause and produce a disposition report. Do not merge open branches until the report is reviewed.

The report lists every open issue and branch you are working, each with:

- the LLD gap ID it addresses (or "none"),
- its disposition from section 4 (Do now / Re-scope / Phase N / Defer / Freeze),
- whether the change adds any direct `UPDATE`, `let _ =` on a write, `const` declaration table, counter, or advisory check,
- whether it touches any file under the five Lattice dimension names.

Stop work immediately on anything that falls into these freeze categories:

1. Extending the ladder or any `*_trust` contract with new `const` tables or classifiers.
2. Adding commands to `command_registry.rs` as declarations without routing.
3. Adding `write_accounting` sinks or other counters as the fix for a finding.
4. Building outbox, queue or event infrastructure outside the ledger crate described in section 5.
5. Any new advisory or record-only check presented as a control.
6. Any doc or comment that describes behaviour without a `path:line` citation.

## 3. Hard rules for every change

These are invariants. A change that cannot satisfy one is not merged; you stop and ask (section 6).

1. **No new direct writes to canonical tables.** Do not add `UPDATE`, `DELETE` or bare `INSERT` on canonical state outside the ledger crate once it exists. Until then, the count of `UPDATE <ident>` in production Rust may not rise above 335 and must fall with every phase.
2. **No discarded write results.** No `let _ =` on any `sqlx` call or any `Result` from a write. Fix the existing 119 as you touch their files.
3. **No `unwrap` or `expect` on data read from the database.**
4. **A control refuses or it does not exist.** If a check is called a gate, contract, binding or validation, it must block the effect it guards. A check that only records is named and typed as an observation, and records `Undetermined` with a reason class, never `Refused` or `Pass`.
5. **If enforcing a control breaks things, report it.** Example: `port_trust` would refuse 47 of 102 cards. Do not downgrade the control to advisory to keep tests green. List what fails and stop.
6. **One word, one meaning.** Presence, Liveness, Truth, Grounding and Binding refer only to the Lattice spec's claim-level dimensions. The current ladder rungs get renamed in Phase 0.
7. **Evidence is not verdict.** Code that records evidence (tool invocations, artifact hashes, provenance edges) is separate from code that judges it. No function both stamps provenance and amends the claim.
8. **Interpretive is never provenance.** LLM-judge output is typed `interpretive` and can never appear as a source in a provenance or Grounding walk.
9. **Every behavioural claim is cited.** Comments and docs that say what code does carry a `path:line` citation, or are marked as intent. Use the LLD's \[V\] / \[I\] / \[D\] legend.
10. **Atomic with its effect.** Any record of a decision, charge, or version is written in the same transaction as the effect it describes. Write-behind, detached tasks and fire-and-forget are not acceptable for these records.
11. **Hashing is canonical.** Use JCS for canonical JSON and one prefixed hash format (G22) for all new hashes.

## 4. Gap register disposition

Every LLD finding gets one disposition. **Do now** means fix in the current code, because the fix is target-agnostic or the risk is live. **Re-scope** means do the fix in the form stated here, not the LLD's "Direction" column. **Phase N** means it is built as part of that phase, not patched beforehand. **Defer** means do not start it.

| ID | Sev | Disposition | Instruction |
| --- | --- | --- | --- |
| G01 | P0 | Do now | Every credit a spawn moves is a transfer in one transaction: the template budget from the platform sponsor wallet while the caller is within a per-caller quota and the sponsor can cover it; otherwise from the caller, only with the caller's explicit acceptance. `extra_budget` from the caller; cap `extra_budget`. Never mint. *(Amended 2026-11-06 by the owner, decision recorded on PR #132; was "Debit the caller in the same transaction as the deposit".)* |
| G02 | P0 | Do now | Filter by caller and agent slug; return 404 otherwise. |
| G03 | P0 | Do now | Resolve, deny private, loopback and link-local ranges, pin the IP. |
| G04 | P0 | Do now | One transaction per charge; reserve before work, settle after. Moves into the ledger in Phase 3. |
| G05 | P1 | Do now | Add `Running` to the enum; remove the `unwrap` (rule 3). |
| G06 | P1 | Do now | Truncate at a char boundary. |
| G07 | P1 | Do now | Close with `Failure` on every exit path via a drop guard. |
| G08 | P1 | Re-scope | Interim only: per-row fallback on a poison batch. No new outbox. Gate decisions become events in Phase 3. |
| G09 | P1 | Do now | Join sourced values against `context.tool_invocations`; demote unmatched values to `model_inference`. This is the bridge to the Grounding walk. |
| G10 | P1 | Do now | Persist the first `GateOutcome`; replay it, never re-run with an empty `WorldModel`. |
| G11 | P1 | Do now | Pre-filter `Err` becomes `Undetermined` and blocks scoring. |
| G12 | P1 | Do now | Delimit and escape graded output; enforce judge model independence; add the `Interpretive` type (rule 8). |
| G13 | P1 | Do now | Seed only on insert. |
| G14 | P1 | Re-scope | Version the resolved card: JCS hash as identity, UNIQUE, same transaction as the edit. Becomes a stream in Phase 2. |
| G15 | P1 | Phase 1 | Triggers, `ON DELETE RESTRICT` and separate roles are built with the write boundary. |
| G16 | P1 | Do now | Fail boot on migration failure; skip on recorded success plus checksum; remove duplicate DDL. |
| G17 | P1 | Do now | Classify LLM spend by handler, not by path allowlist. |
| G18 | P2 | Phase 1 | Streams with expected-version append, in the ledger crate. Never as a patch. |
| G19 | P2 | Phase 1 | Content-addressed artifact store, in the ledger crate. |
| G20 | P2 | Phase 3 | DPP lineage built on the ledger. Hash the plaintext token now. |
| G21 | P2 | Defer | Single replica until Phase 3 completes. |
| G22 | P2 | Phase 1 | JCS plus one prefix, defined in the ledger crate. |
| G23 | P2 | Phase 2 | θ as versioned policy data in the Fermi slice. |
| G24 | P2 | Do now | Per-agent tool allowlist from the card. |
| G25 | P2 | Defer | Except HMAC signing and credential encryption: do now. |
| G26 | P2 | Do now | Pass attachments into the loop messages. |
| G27 | P3 | Do now | Code-exchange flow with CSPRNG state; intersect scopes. |
| G28 | P3 | Defer |  |
| G29 | P3 | Defer | Do not write these columns in the old model. |
| G30 | P3 | Do now | Record `Undetermined` with a reason class (rule 4). |
| G31 | P3 | Do now | Use the shared `pg_notify` broadcast. |
| G32 | P3 | Defer |  |
| G33 | P3 | Defer | Versioning is designed into the ledger's event types in Phase 1. |

Order within "Do now": G01–G04 first, then G05–G07, G11, G16, then the rest.

## 5. Phased plan

The migration is strangler-style: the new substrate is built clean, and existing code moves behind it one vertical slice at a time. Each phase ends at a gate; do not start the next phase until every acceptance criterion holds.

### Phase 0: make the design enforceable

Work:

- Commit the Lattice spec under `docs/architecture/` as normative. Number its invariants.
- Add the numbered invariants and section 3 of this directive to the agent instruction file (CLAUDE.md or equivalent).
- Rename the ladder and its rungs (for example `platform_health` with rungs `schema_present`, `write_path_live`, `rollup_consistent`, `field_tool_sourceable`, `input_port_match`). Rename code, routes, UI labels and docs together.
- Add CI ratchets: commit current counts for `UPDATE <ident>`, `let _ = sqlx`, `let _ =` in `src/`, and `unwrap`/`expect` on DB decode. CI fails if any count rises.
- Enable `clippy::let_underscore_must_use` and mark write results `#[must_use]`.
- Fix all "Do now" items from section 4.

Acceptance:

- [ ] Spec tracked; invariants numbered and referenced from the agent instructions.
- [ ] `git grep` finds no use of the five dimension names outside Lattice code and the spec.
- [ ] Ratchet job runs in CI and fails on a deliberate test increase.
- [ ] G01–G04 fixed, each with a test that fails on the old code.

### Phase 1: the ledger crate (write boundary)

Build `abw-ledger` per LLD §10.1: `streams`, `events` (append with expected-version check), `artifacts` (content-addressed), JCS canonicalisation, one hash format, typed event envelopes with a schema version.

- The crate is the only code that holds a write-capable pool handle. `AppState` exposes a read-only handle to handlers.
- Database: `events` gets `BEFORE UPDATE OR DELETE` triggers that raise; foreign keys use `ON DELETE RESTRICT`; an owner role runs migrations, an app role has INSERT on `events` and nothing destructive (G15).
- Projections are updated in the same transaction as the append.

Acceptance:

- [ ] A handler that tries to write directly fails to compile.
- [ ] An `UPDATE` on `events` as the app role fails at the database.
- [ ] Concurrent appends with the same expected version: exactly one succeeds (test).
- [ ] Artifact round trip: store, resolve by hash, bytes identical.

### Phase 2: Fermi vertical slice, all five dimensions

Fermi goes first because Binding and Brier scoring are native to forecasts, `forecast_agent_claims` is already append-only, and outcomes resolve, so the calibration loop can be tested for real.

Event flow: `claim.submitted` → `lattice.evaluated` (one verdict vector) → `route.decided` (Proceed, Sign-off, Correct, Endorse, Park) → `outcome.observed`. Tool invocations are events. θ lives in a versioned `policy` table and `theta_version` is part of the vector (G23).

Write these conformance tests before the implementation:

- [ ] Replay: the verdict vector is reproduced exactly from (claim, artifacts, stream heads, provenance graph, θ version).
- [ ] An `interpretive` event is never reachable in a Grounding walk.
- [ ] Evidence-writing and verdict-writing code are separate modules; no function writes both.
- [ ] Every dimension returns one of `pass | fail | unsupported | not_evaluated`; a missing substrate yields `unsupported`, never `pass`.
- [ ] `lattice.evaluated` and `route.decided` commit in one transaction.

### Phase 3: DPP slice, then remaining writers

- DPP and carbon accounting on the ledger: Presence and Truth against stored artifacts, episode lineage, signed passports (G20).
- Money: charges as ledger events, balance derived from the ledger (completes G04).
- Gate decisions as events written with their effect (completes G08).
- Migrate remaining writers; old tables become projections.

Acceptance:

- [ ] A published passport can be traced to the episode, source commit and artifact hashes that produced it.
- [ ] Ratchet counts for direct `UPDATE` outside `abw-ledger` reach zero.
- [ ] Every question in LLD §7.4 is answerable at strength "Strong".

## 6. Per-PR protocol and stop-and-ask conditions

Every PR description contains these fields. A PR missing any of them is not ready for review.

| Field | Content |
| --- | --- |
| Gap / phase | LLD gap ID and its section 4 disposition, or the phase and acceptance item it advances |
| Spec reference | Lattice spec invariant number(s) touched, or "none" |
| Rules | Which section 3 rules the change touches and how it satisfies each |
| Ratchet delta | Before and after counts for every ratchet metric |
| Evidence | The test that fails on the old code and passes on the new |
| Claims made | Any new comment or doc stating behaviour, with its citation |

Stop and ask the human, without working around the problem, when:

- enforcing a control would break existing behaviour, tests, or cards (rule 5);
- the spec is silent or ambiguous on a decision the change requires;
- the cheapest fix would add a direct write, a counter, an advisory check, or a declaration-only table;
- a fix in the old model would duplicate infrastructure planned for the ledger crate;
- you find code or docs that contradict this directive or the spec.

When you stop, state the conflict in one paragraph, list the options, and name the one you recommend. Do not pick an option and proceed.

A separate reviewer pass checks each PR against the spec. It uses a different prompt from the implementer, has the spec in context, and answers one question: does this diff move the code toward or away from the stated invariants?
