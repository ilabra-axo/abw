# ABW Refactor-in-Place Plan

Nov 10, 2026 · @ivan Labra

ABW is refactored in place, in its existing repository, CI and Railway/Neon environment, until the code implements the Logical Architecture and the Verification Lattice as designed. The credit economy is rebuilt on an event-sourced ledger, and ACN becomes the only path by which value enters or leaves ABW.

## Purpose and core decision

The low-level design audit found that v1's failures cluster at boundaries: transactions, authorization, write paths, gate recording, and claims that a check happened. The algorithmic kernels (Monte Carlo, Sobol, Shapley, TEC settling, BayesOps) are not implicated. So the rebuild targets the spine, not the organs.

Three decisions frame everything below:

- **Refactor in place, not a greenfield rewrite.** The repo, toolchain, migrations runner, deployment and secrets stay. New substrate is added beside old code, and paths migrate onto it one aggregate at a time until old write paths reach zero.
- **Enforcement before code.** Audits in August found the right problems and the metrics got worse anyway. Only the one finding turned into a CI check (`tests/write_accounting_coverage.rs`) held. Every requirement in this plan therefore exists first as a check that blocks merges.
- **Zero agent trust, including Claude.** Code is generated; nothing generated is believed until an independent check, written by a different session or person, says so. Demos are not evidence.

Done means: every invariant in the register is `enforced`, every ratchet is at its target, the Lattice adversarial suite passes, and each tenant passes its own acceptance suite. Nothing is "done" on an agent's say-so.

## Sources of truth

When documents disagree, the higher row wins; code never wins over a design document, it is measured against it.

| Rank | Document | Governs |
| --- | --- | --- |
| 1 | This plan + `invariants/registry.toml` | What must hold, and how it is checked |
| 2 | `docs/architecture/ABW Verification Lattice — Specification v0.1.md` (commit it; it is untracked) | What a gate checks; verdict states; judge; routing |
| 3 | `ABW_LOW_LEVEL_DESIGN.md`, §9 gap register and §10 target state | Known defects (G01–G33) and the minimal substrate |
| 4 | `docs/architecture/ABW — Logical Architecture in Capability Terms.md` | Current requirements (not yet reviewed against this plan) |
| 5 | ABW Logical Architecture paper (June, amended Nov 2026) | Primitives, five loops, signal classes, MoE |
| 6 | ACN spec fork in the repo | Commitments, Cashu settlement, binding API |
| 7 | Everything else in `docs/` | Context only; Appendix D of the LLD lists known drift |

The paper is amended on four points before Phase 0 closes:

1. Lattice verdicts gate entry into Loop 1 consolidation. LLM-judged episodes no longer consolidate ungated.
2. Hard-verified ground truth carries its own provenance (source, artifact hash, recorder) and passes Presence and Grounding.
3. The VSM table gains System 3\*: the Lattice at runtime, independent verification in development.
4. A typed `derivation` event (inputs, kernel version, seed, output) makes computed values groundable.

## The enforcement mechanism

The mechanism is an invariant harness in the repository, run by `cargo xtask invariants` in CI, that blocks merges. It has five parts, and each one answers a specific v1 failure.

**1. Invariant registry** (`invariants/registry.toml`). One entry per invariant:

```toml
[[invariant]]
id = "INV-MONEY-01"
statement = "Credits change only through balanced ledger events"
source = ["G01", "G04", "plan §ACN"]
check = "static"            # static | deps | schema | test | runtime
command = "xtask check no-wallet-writes"
mutant = "invariants/mutants/INV-MONEY-01.patch"
status = "enforced"         # proposed | enforced | retired
```

Check kinds: `static` (AST or grep lints over the workspace), `deps` (crate dependency bans via `cargo metadata`), `schema` (queries against a migrated scratch database: triggers, FKs, constraints), `test` (named acceptance or adversarial tests), `runtime` (base-rate alarms on live gates).

**2. Ratchets** (`invariants/ratchets.toml`). Counted metrics with a baseline and a target. CI fails if any count rises; when it falls, the baseline is lowered in the same PR. Initial ratchets come from LLD §4.2: `UPDATE` sites outside the substrate (335 → 0), `let _ = sqlx::query` (119 → 0), handlers without the command dispatcher, `unwrap` on DB data, routes outside the tenancy filter.

**3. Mutants** (`invariants/mutants/`). Each invariant ships with a patch that violates it. A CI job applies each mutant and requires its check to fail. A check that has never caught its own mutant is not counted as enforced. This answers "a gate that is declared is not a gate."

**4. Separation of duties.** A PR may change product code or the verification assets (`invariants/`, `tests/acceptance/`, `tests/adversarial/`, `tests/oracles/`), never both. CI rejects mixed PRs, and CODEOWNERS requires a different reviewer for each side. Builder and Verifier are different agent sessions with no shared context; human sign-off on phase gates comes from someone other than the person who drove the build.

**5. Phase gates.** A phase closes only when `xtask gate <phase>` passes. It generates a report from the registry, ratchets and test results. A phase report written in prose by an agent is never the gate.

The harness is built so it can be lifted out later as an Executable Intent reference implementation; it should depend on nothing ABW-specific beyond the registry file.

## Initial invariant register

These 24 invariants seed `registry.toml`. The Verifier refines wording and adds the rest of G01–G33; the Builder may not edit them. "Phase" is when each must reach `enforced`.

| ID | Invariant | Source | Check | Phase |
| --- | --- | --- | --- | --- |
| INV-SEP-01 | No PR changes product code and verification assets together | v1 process failure | static (CI) | 0 |
| INV-HAR-01 | Every enforced invariant fails on its own mutant | Paper §2.5 note | test | 0 |
| INV-TEN-01 | Every read or write of tenant data is filtered by the caller's owner identity | G02 | static + adversarial test | 1 |
| INV-NET-01 | All outbound HTTP goes through the egress guard; private, loopback and link-local ranges are refused | G03 | deps + test | 1 |
| INV-ROB-01 | No `unwrap`/`expect`/byte-slicing on DB or external data | G05, G06 | static | 1 |
| INV-EPI-01 | Every episode reaches a terminal status on every exit path | G07 | test (fault injection) | 1 |
| INV-EVL-01 | Evaluator or provider errors yield `not_evaluated`, never pass | G11 | adversarial test | 1 |
| INV-MONEY-01 | Credits change only through double-entry ledger events in the dispatcher; every entry nets to zero | G01, G04 | static + deps | 3 |
| INV-SPEND-01 | Every tool-loop route reserves credits before work and settles after, in the dispatcher | G17 | test | 3 |
| INV-SUB-01 | Canonical state changes only via events appended with an expected-version CAS | G18, LLD §10.1 | static (ratchet to 0) | 3 |
| INV-SUB-02 | Events and artifacts reject UPDATE and DELETE at the database | G15 | schema | 3 |
| INV-SUB-03 | Side effects leave via an outbox written in the same transaction as their event | G08, LLD §4.1 | static + crash test | 3 |
| INV-SUB-04 | Every state-changing request enters through the typed command dispatcher | LLD §4.8 | static | 3 |
| INV-SCH-01 | Migrations run once, are checksummed, and a failure stops boot | G16 | test | 3 |
| INV-AGT-01 | An agent version is the canonical-JSON hash of the resolved card; seeding is a version event | G13, G14 | schema + test | 4 |
| INV-AGT-02 | Each agent's tools come from its card's allowlist via capability tokens | G24 | test | 4 |
| INV-PRV-01 | Every tool invocation is recorded with input and output hashes in the episode stream | Paper §2.2, G09 | test | 4 |
| INV-PRV-02 | Every kernel output used in a claim is a `derivation` event | Plan §Salvage | static + test | 4 |
| INV-LAT-01 | No claim is `pass` on Grounding unless a provenance walk reaches a tool or observation event | G09, Lattice | adversarial test | 5 |
| INV-LAT-02 | Judge output is typed `interpretive`, uses an independent model lineage, and is never walked as provenance | G12, Lattice | static + test | 5 |
| INV-LAT-03 | Verdict vector, route decision and their events commit in one transaction | Lattice, G08 | test | 5 |
| INV-LAT-04 | Every θ is a row in a versioned policy table; verdicts record `theta_version` | G23 | static (no const θ) | 5 |
| INV-LOOP-01 | Every loop gate is shown not absent, not fatal and not proxied | Paper §2.5 note | test + runtime | 6 |
| INV-LOOP-02 | Only verdict-qualified episodes enter consolidation | Plan amendment 1 | test | 6 |

## Salvage policy

Keep the intellectual substance, replace the plumbing. Every component lands in exactly one of four bins, recorded in `invariants/salvage.toml` during Phase 0:

| Bin | What goes in it | Treatment |
| --- | --- | --- |
| Kernel | Pure computations: Monte Carlo (`crates/projections`), Sobol, exact Shapley, Brier and calibration, TEC/ECHO settling, DBSCAN clustering, BayesOps posterior fitting, PCA projector, probably the FPL parser and interpreter | Extract into pure crates, certify against independent oracles, keep |
| Knowledge as data | Agent cards, prompts, strategist definitions (cohere\_and\_coordinate, pipeline, MoE routing, debate), grounding field contracts, schemas, current θ values | Port as data; θ values become initial rows in the policy table |
| Pattern, re-implement | Embedding provenance, append-only + `DISTINCT ON`, tri-state `Decision`, gate model, episode lifecycle, memory store, consolidation orchestration, coordination cascade | Keep the design, rebuild on the substrate |
| Discard | Current credit/wallet/gas code (replaced by the new ledger), gate write-behind, ladder naming, handler-level write paths, `PgListener`-per-client SSE | Delete once its replacement passes |

**Purity rule (INV-KER-01).** Kernel crates may not depend on `sqlx`, `tokio` networking, `reqwest`, the clock or global state. All I/O and randomness are injected; RNGs are seeded explicitly. The `deps` check enforces this, so a kernel cannot drift back into boundary code.

**Certification.** The Verifier writes oracle tests in `tests/oracles/` before the kernel is moved. Characterisation tests against v1 output are used only to detect regressions during extraction, never to certify, because they also capture v1 bugs.

| Kernel | Independent oracle |
| --- | --- |
| Monte Carlo | Convergence to closed-form answers on analytic cases; bit-identical replay from seed; confidence-interval coverage over repeated runs |
| Sobol | Ishigami and Sobol g-function benchmarks with known first-order and total indices; indices sum within tolerance |
| Shapley | Efficiency, symmetry and null-player axioms; brute-force agreement for n ≤ 8; the existing residual and reconstruction gates |
| Brier / calibration | Hand-computed fixtures; proper-scoring checks; team score never copied to agents |
| TEC settling | Thagard's published ECHO examples; the measured case where Γ is 0.632 both when a correction is absorbed and when it is rejected |
| DBSCAN | Reference implementation on standard datasets; deterministic clustering for fixed input order |
| BayesOps | Conjugate cases with analytic posteriors; posterior predictive checks on synthetic data |
| FPL | Grammar round-trip; golden programs evaluated through the Monte Carlo oracle |

**Strategists.** cohere\_and\_coordinate's research-grounded design and prompts carry over as data. Its orchestration is re-verified piece by piece: whether observations reach member memory, whether anything reads the coordination brief, and the valence-homophily threshold that currently exists only in prompt text. Each gets an acceptance test before it is claimed to work.

## Economy: internal ledger, ACN at the boundary

ABW keeps its credit economy, rebuilt as an event-sourced double-entry ledger. Value enters and leaves only through ACN commitments settled in Cashu: you buy credits through ACN, spend them inside your own estate, and earn sats by selling or renting assets to other owners through ACN.

The credit system is ABW's System 3 resource bargain: budgets fund workspaces, agents hire agents, and `select_agent` ranks candidates partly by cost. Deleting it would remove a regulatory loop. What failed in v1 is its implementation (G01 minting, G04 non-atomic charging), and the ledger fixes both by construction.

| Flow | Rail | Unit | Mechanism |
| --- | --- | --- | --- |
| Buy credits | ACN | sats in, credits out | Purchase commitment to the tenant operator; the operator issues credits against a Cashu redemption |
| Spend inside your own estate | Internal ledger | credits | Transfers between your own workspaces, agents and apps; LLM and tool spend reserved and settled by the dispatcher |
| Sell, rent or license an asset to another owner | ACN | sats | Commitment with conditions of satisfaction; Lattice verdict as delivery evidence; payment to the owner's key |
| Hire another owner's agent inside ABW | ACN | sats | As a rental; compute follows the listing's hosting model |

**Ledger rules.**

- Double-entry and event-sourced on the substrate; balances are projections, never written directly.
- Every entry nets to zero.
- Issuance happens only by the tenant operator, only from a purchase event carrying a Cashu redemption reference.
- Credits never cross ownership boundaries. A transfer between different owners is rejected; value between owners always goes through ACN.
- Credits are not redeemable. The way out is earning sats through commitments for something delivered.

**Hosting model, declared per listing.**

- **Renter-hosted:** the asset runs in the renter's workspace; the renter pays compute in their own credits; the ACN price is the owner's licence or outcome fee.
- **Owner-hosted:** the asset runs in the owner's estate as a service; the owner pays compute; the ACN price covers compute plus margin.

**What happens to v1 economy code.** Phase 2 maps every issuance, transfer, sink and budget and freezes it. Phase 3 rebuilds it on the ledger: `credits::charge`, wallets and `credit_ledger` are replaced; App budget deposits become transfers from the owner's account; execution fees and gas become dispatcher reservations. Demo balances are archived, not migrated.

**ACN binding rules, from decisions already made.**

- Every ABW agent gets a keypair; each tenant has one operator key. Private keys live in an encrypted key store, never in plaintext columns or logs.
- Commitments inside one owner's agent team stay off ACN. The coordinating, contract-holding agent delivers team output.
- Internal ABW acts are never relayed. Only allowlisted event types leave, and only via the outbox.
- The requester declares satisfaction (Flores). The Lattice verdict vector is evidence attached to a delivery; it never declares satisfaction for the requester.
- Conditions of satisfaction, including any LLM-judge rubric and threshold, are visible to the performer, with explicit disqualifiers.

**Event mapping.** ABW events carry ACN event ids; ACN deliveries carry ABW artifact hashes and the verdict vector's hash.

| ACN step | ABW event | Evidence attached |
| --- | --- | --- |
| Offer / counter-offer with CoS | `commitment.proposed` | CoS template and rubric hash |
| Accept | `commitment.bound` | Both keys' signatures; deadline |
| Deliver | `commitment.delivered` | Artifact hashes, verdict vector hash |
| Declare satisfaction | `commitment.satisfied` or `commitment.disputed` | Requester signature |
| Settle | `commitment.settled` | Cashu redemption reference; for a credit purchase, the `ledger.issued` event |

**Option, decided in the ACN spec fork, not here:** ACN Core could make settlement a pluggable binding, so cross-owner hires inside ABW use the same commitment state machine with the ledger as settlement. Until then, cross-owner value inside ABW uses the Cashu binding.

**Added invariants:** INV-MONEY-02 (issuance requires the operator key and a redemption reference), INV-MONEY-03 (no ledger transfer across owners), INV-MONEY-04 (balances written only by the ledger projector), and INV-ACN-01 (only the binding crate depends on Nostr, Cashu or key-store crates), INV-ACN-02 (satisfaction events require the requester's key), INV-ACN-03 (only allowlisted event types reach the relay outbox), INV-ACN-04 (no private key material in DB columns, logs or error strings).

One-way, non-redeemable platform credits generally fall under lighter rules than redeemable stored value, but whether ABW qualifies in the EU and Switzerland needs counsel's confirmation before credits are sold.

## Phased plan

Eight phases (0–7), each closed only by `xtask gate <n>`. Phases run in order; kernel salvage runs alongside Phases 1–3 because it touches no shared state.

### Phase 0 — Harness, inventory, freeze

- Feature freeze. Take every demo endpoint off the public internet or behind auth.
- Build the harness: registry, ratchets, mutants, mixed-PR check, CODEOWNERS, `xtask gate`. Record ratchet baselines at HEAD.
- Commit the Lattice spec. Apply the four paper amendments. Reconcile the "Capability Terms" document against this plan and log conflicts.
- Inventory: fill `salvage.toml` with every crate and major module (bin, purity, existing test evidence).
- **Exit:** INV-SEP-01 and INV-HAR-01 enforced; each registry entry has a check and a mutant, even if `proposed`; inventory complete and reviewed by a human.

### Phase 1 — Containment

Cheap fixes that stop known damage while the larger work proceeds: G02, G03, G05, G06, G07, G11.

- **Exit:** INV-TEN-01, INV-NET-01, INV-ROB-01, INV-EPI-01 and INV-EVL-01 enforced, each with its adversarial test (second-tenant read, metadata-endpoint fetch, multi-byte truncation, error mid-loop, provider outage).

### Phase 2 — Map and freeze the economy

- The Builder maps every issuance, transfer, sink and budget in v1, with file:line: purchases, App budgets (`initial_budget`, `extra_budget`, the batch path), execution fees, gas, workspace budgets, hires, rewards and the claim-multiplier hook.
- The Verifier writes the ledger's acceptance and adversarial tests from the economy section: conservation, unbacked issuance, cross-owner transfer, crash between reserve and settle, double spend under concurrency.
- Freeze: disable App budget deposits now (G01). Add a ratchet on call sites of v1 credit functions that may only fall.
- **Exit:** flow map reviewed by a human; ledger test suite merged and failing (no implementation yet); the G01 path disabled with its adversarial test passing; the credit call-site ratchet in place.

### Phase 3 — Substrate

- Add `streams`, `events` (CAS on head, `payload_sha` over JCS), `outbox` and `artifacts` per LLD §10.1, with UPDATE/DELETE-blocking triggers and `ON DELETE RESTRICT`.
- Add the typed command dispatcher with tenancy and credit reservation inside it. Fix the migrations runner (G16).
- Migrate aggregates one at a time behind the dispatcher: workspaces and outputs, then episodes, then the remaining tables. Each migration lowers the `UPDATE` ratchet.
- Replace gate write-behind with in-transaction events (G08). Build the credit ledger as its own aggregate and move every v1 credit call site onto it.
- **Exit:** INV-SUB-01 to 04, INV-SCH-01, INV-MONEY-01 to 04 and INV-SPEND-01 enforced, with v1 credit code deleted; kill -9 at every write point violates no invariant; projections replayed from events are bit-identical to live state; the `UPDATE` ratchet is at 0 outside projectors.

### Phase 4 — Agent, episode, provenance, ACN binding

- Agent versions as hashes of the resolved card; seeding as a version event; per-card tool allowlists with capability tokens.
- Episodes as streams: `claim.submitted`, `tool_invocation` with I/O hashes, `derivation`, `episode.closed`.
- Wire certified kernels in through `derivation` events.
- Build `acn-binding-abw` on the outbox.
- **Exit:** INV-AGT-01/02, INV-PRV-01/02, INV-KER-01 and INV-ACN-01 to 04 enforced; for any episode, the configuration, tool I/O and kernel derivations are provable by hash; one end-to-end ACN commitment round-trips on a test relay and test mint; a credit purchase issues credits only against a Cashu redemption; an agent rental settles in sats to the owner's key.

### Phase 5 — Lattice vertical slice

- Verdict vector value type; five dimensions bound to the physical checks of LLD §10.2; four verdict states.
- Versioned policy table for θ; interpretive judge on an independent model lineage, with graded output delimited, never interpolated.
- Routing actions (Proceed, Sign-off, Correct, Endorse, Park) committed with `lattice.evaluated`.
- **Exit:** INV-LAT-01 to 04 enforced, and the adversarial suite passes: a planted fabrication gives `unsupported`; a stale citation fails Liveness; a provider outage gives `not_evaluated`; a judge-injection attempt leaves the verdict unchanged; a derived value with a valid `derivation` chain passes Grounding.

### Phase 6 — Loops, most verifiable first

Order: Loop 5 (calibration, with observation provenance), Loop 2 (human correction, discriminating on the correction's own post-settling activation, Γ as floor, `Undetermined` as non-pass), Loop 1 (consolidation of verdict-qualified episodes only), Loop 4 (Shapley-driven proposals, never auto-applied), Loop 3 (TEC workspace coherence and the cohere\_and\_coordinate cascade).

- **Exit per loop:** INV-LOOP-01 shown for its gate (fires, admits valid input, rejects a crafted input that satisfies the proxy but not the property); runtime base-rate alarms live; INV-LOOP-02 enforced when Loop 1 lands.

### Phase 7 — Tenants

See the next section. A tenant ships only on its own acceptance suite.

## Tenant requalification

Tenants are re-qualified, not rewritten: the new spine, plus certified kernels, plus ported cards and prompts. They come in order of how checkable their claims are, so each one also exercises a Lattice dimension.

| Order | Tenant | Lattice dimension it proves | Keeps from v1 | Rebuilt |
| --- | --- | --- | --- | --- |
| 1 | Fermi Console | Binding: forecasts with condition, deadline, Brier | Monte Carlo, Sobol, Shapley, FPL, BayesOps, MoE routing cards | Forecast storage, attribution, quota, Polymarket ingestion provenance |
| 2 | SimOps | Hard-verified ground truth; `derivation` for projections | Dynamics models, BayesOps, projection scoring | Batch observation ingestion with provenance |
| 3 | DPP Studio | Presence and Grounding end to end | Digital-twin model, carbon calculations as kernels | Carbon-factor lineage to episodes, signed and hashed publication (G20) |
| 4 | Rabble | External-source provenance (GBIF) | Game logic, creature ecology | Species-data ingestion as observations |
| 5 | Pulse, Arborea, architect lens, cow-tipping game | Substrate only: tenancy, quota, sensor-data provenance | Experience and AR client code | Server-side state on the substrate |

Per tenant, in this order:

1. A one-page claim spec: what the tenant asserts, what the ground truth is, and which claim types enter the Lattice versus are routed past it by policy.
2. Acceptance and adversarial tests, written by the Verifier from the claim spec, merged before any port.
3. The port, by the Builder, with v1 as a behavioural reference and its kernels and data as sources.
4. `xtask gate tenant-<name>` passes. Demos built afterwards are demos, not evidence.

DPP Studio gets the strictest suite because its outputs carry external compliance weight.

## Agent prompts

Three prompts: a shared preamble, then a Builder and a Verifier role. Run the two roles as separate sessions that never share context. Export this plan to `docs/refactor/ABW_REFACTOR_PLAN.md` and commit it before the first session; the prompts point there.

**Shared preamble** (prepend to both roles):

```markdown
You are working in the ABW repository on a refactor-in-place defined by
docs/refactor/ABW_REFACTOR_PLAN.md. Read it fully before anything else, then
the documents in its "Sources of truth" table, in rank order. Where sources
disagree, the higher rank wins. Code is evidence of current behaviour, never
of intended behaviour.

Operating rules:
- Keep the existing toolchain, CI, migrations runner and Railway/Neon
  deployment. Change environment config only when a phase requires it, and
  say so explicitly in the PR.
- One invariant, one aggregate, or one kernel per PR. Small diffs.
- Never introduce a pattern, crate or abstraction the plan does not name. If
  you think one is needed, stop and write a proposal instead of code.
- Every factual claim in a PR description or report cites file:line or the
  exact command and its output, tagged [V] verified or [I] inferred.
- You never declare anything done. Done is `cargo xtask gate <phase>`
  passing in CI. Paste its output; do not paraphrase it.
- Never weaken, skip, delete or special-case a check to make it pass. If a
  check looks wrong, open an issue titled "CHECK-DISPUTE: <id>" and stop work
  on that item.
- If the plan is ambiguous, ask. Do not resolve ambiguity by reading the old
  code and copying what it does.
```

**Builder role:**

```markdown
Role: BUILDER. You change product code only.

You may not create or edit anything under invariants/, tests/acceptance/,
tests/adversarial/ or tests/oracles/. CI rejects PRs that mix product code
with those paths. The Verifier owns them; you work to make them pass.

For each task:
1. Name the registry ids and ratchets your PR targets.
2. Run `cargo xtask invariants` before and after; include both outputs.
3. A ratchet that rises fails your PR. When one falls, lower its baseline
   in invariants/ratchets.toml in a separate Verifier-approved PR.
4. When porting from v1: kernels are moved and wired through `derivation`
   events only after their oracle tests exist and pass; cards, prompts and
   θ values are ported as data; everything else is reimplemented on the
   substrate per the salvage bins.

Phase 0 task: produce invariants/salvage.toml. For every workspace crate and
major module record: path, purpose (one line), bin
(kernel | data | pattern | discard), purity evidence (its dependencies from
`cargo metadata`), existing test evidence (test names, whether they ran),
and open questions. Do not change code in Phase 0.
```

**Verifier role:**

```markdown
Role: VERIFIER. You write checks; you never change product code.

You write invariant checks, ratchets, mutants, acceptance tests,
adversarial tests and kernel oracle tests from the plan and the design
documents, not from the implementation. Read product code only to locate
where a check must attach, never to decide what correct behaviour is.

Rules:
- Every check ships with a mutant patch that violates it. A check is
  `enforced` only after CI shows it failing on its mutant and passing on
  correct code. Record both runs.
- Prefer independent oracles: analytic results, published benchmarks,
  axioms, brute force on small cases. Characterisation against v1 output
  detects regressions only; it never certifies.
- Write adversarial cases for every invariant: another tenant, a crash
  point, a provider outage, a planted fabrication, a stale citation, an
  injection attempt.
- Report spec ambiguities as issues. Do not resolve them yourself.

Phase 0 task: build the harness in the plan's "enforcement mechanism"
section (registry, ratchets with HEAD baselines, mutants, mixed-PR check,
CODEOWNERS, `cargo xtask invariants` and `cargo xtask gate`), seed
registry.toml from the plan's register and G01–G33, and get INV-SEP-01 and
INV-HAR-01 to `enforced`. Then review the Builder's salvage.toml
independently and list every disagreement.
```

## Open decisions for Ivan

These block specific phases; none blocks Phase 0.

- [ ] **Human gate signer.** Who signs phase gates, other than the person driving the build? Needed by the end of Phase 0.
- [ ] **Cashu custody.** Which mint, who operates it, and how each tenant's operator wallet is held. Needed before Phase 4.
- [ ] **Credit pricing and treatment.** Credit price in sats, default workspace budgets, and the default hosting model for listings; counsel to confirm the credits' regulatory treatment before any sale. Needed for Phase 3.
- [ ] **Judge lineage.** Which provider and model serve as the independent interpretive judge. Needed for Phase 5.
- [ ] **Lattice scope per tenant.** Which claim types get a verdict vector and which are routed past by policy. Needed per tenant in Phase 7.
- [ ] **Label supply.** Who adjudicates Loop 2 corrections and false-pass reviews, and at what rate, so residual-risk estimates have data. Needed for Phase 6.
- [ ] **Single replica.** Confirm ABW stays on one instance until Phase 3 closes; G21 (per-process state) is deferred until then.
- [ ] **Capability Terms reconciliation.** Accept or reject the conflicts the Phase 0 review finds between that document and this plan.
