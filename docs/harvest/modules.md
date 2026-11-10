# Feature harvest — modules, loops, kernels

| | |
|---|---|
| **Source** | `/home/ilabra/fermi`, `git rev-parse --short HEAD` = **`a3cea516`** (2026-10-08 11:40 +0200, "The BOM total was the model's arithmetic…") |
| **Harvest date** | 2026-11-10 |
| **Method** | Read-only. I used `cargo metadata --no-deps` for members, targets and normal deps, then `wc -l`/`grep -c` for line counts, test counts (`#[test]` + `#[tokio::test]`) and impurity markers (`sqlx`/`PgPool`/`query(`, `reqwest`, `tokio::`/`async fn`, `Utc::now`/`SystemTime`, statics/`OnceLock`/`thread_rng`, `axum::`). I read the `//!` module headers, then followed the code at every gate named in the loop docs. I did not build or run tests. The `target/` dir is 307 GB and I did not touch it. |
| **Docs read** | `docs/architecture/FEEDBACK_LOOPS.md` (1915 l.), `docs/architecture/COMBINATORIAL_CREDIT_ASSIGNMENT.md` (452 l.), `docs/papers/abw_logical_architecture.md` (467 l.), plus `docs/HANDOFF_loops_and_gates.md` (469 l.), which the paper names as the authority on which gates actually hold. |
| **Notation** | `file:line` cites refer to the working tree. **[I]** = inference, not verified by reading the code path end to end. "T=" = test-attribute count. |

**Caveats**

1. **The working tree is dirty.** 18 tracked files are modified (+2599/−1423), including `agent-bestiary/memory/src/{store,types}.rs`, `src/handlers/{agents,observatory,economics,mod}.rs`, `src/api_server.rs` and the papers. There are also untracked `abw-invariant-compiler-kit/` and `docs/ABW — Coding Agent Directive Lattice as Target.md`. Line counts and cites are from the **working tree**, not from `a3cea516`. Line numbers in `observatory.rs`, `store.rs` and `api_server.rs` may drift by tens of lines from HEAD.
2. **Duplicate copies.** Git worktrees at `.wt/{docs-tooling,g26,verify}` and `.delta/worktrees/ae955xhq5jd8` hold full copies of the repo. I excluded them everywhere. Any repo-wide grep that doesn't exclude them double-counts.
3. **Test counts are attribute counts.** They don't tell you whether a test passes or whether it needs a DB. "Meaningful" is my judgment from the test names and bodies I sampled.
4. **Operational claims come from the docs.** Anything about production row counts ("0 of 3,581 episodes", "Γ mean 0.97") is quoted from the docs or from comments in the code. I could not verify it: no DB access, by design.
5. **Purity is per module.** Several "kernel" algorithms sit inside impure files (sqlx/async). In those cases the table marks the *pure function* worth extracting.

---

## Task A — crates and modules

### A.1 Workspace crates (28 members)

| path | purpose | lines (src) | bin | purity evidence | tests | note |
|---|---|---|---|---|---|---|
| `.` (`fermi`) | Root crate: FPL language + the platform server lib, plus 12 bins, 10 examples and 98 integration tests | 215,505 / 262 files | (split, see A.2) | deps include sqlx, reqwest, tokio full, axum, async-stripe, rust-mcp-sdk | 1,400 attrs in `src/` (lib + bin modules); 509 in `tests/` (98 files, many DB-gated `#[ignore]`) | Bins: `fermi` (REPL), `api-server`, `agent-mcp-server`, `initialize-workspace`, `contract-sketch`, `input-contract-sketch`, `brier-backtest-seed`, `embedding-anchors`, `backfill-embedding-provenance`, `migrate-embedding-dimensions`, `resim_wc`, `apply_wc_cascades` |
| `agent-bestiary/memory` | Episodic/semantic memory store, consolidation (dreaming) worker, DBSCAN, embeddings with provenance, LLM extraction, rule verification | 14,686 / 14 | **pattern** (store, consolidation, embedding provenance); **kernel** inside: `clustering.rs` (DBSCAN), `dyad.rs` | sqlx+pgvector (store.rs: 149 sqlx refs), reqwest (embeddings.rs, llm.rs), tokio | 94 (consolidation 19, store 7, llm 10, embeddings 8, rule_verification 6, locking 4; `tests/` reproducibility, llm_providers, seed) | `consolidation.rs:862` `rank_success_episodes_by_authority` and `:998` budget 30 are Loop 2's propagation fix. `embeddings.rs:50-133` `ProvenancedEmbedding` (model_id, model_version, dim; you can't write without it). `seed.rs` (1,173 l.) is test fixture data. Locally modified (caveat 1). |
| `agent-bestiary/evaluators` | `EvalModel` trait, `EvaluatorRegistry`, aggregator, LLM judge, `BrierEvaluator`, `ProjectionScoringEvaluator` | 2,229 / 13 | **pattern** (registry/tiers/aggregator); **kernel-small**: `projection_scoring.rs:135-136` score, `llm_json.rs` | no sqlx: DB is trait-injected (`projection_scoring.rs:14-17,62`) | 32 (llm_json 10, tests.rs 6, projection_scoring 6, aggregator 5) | A good pattern: hard verifiers take a `Lookup` trait, so the crate stays DB-free |
| `agent-bestiary/observability` | Plane C: `PersonaDriftMonitor`, `AnomalyDetector`, `EpisodeScorer`, `SocialInteractionTracker`, `TrendAnalyzer`, worker | 2,322 / 9 | **pattern**; **kernel-small**: `drift.rs:63` `is_anomalous`, `:197` cosine, `trend.rs` rolling stats | depends on memory (sqlx transitively); clock in social.rs (16) | 39 (social 16, drift 8, anomaly 5, tests 5, trend 4) | `DEFAULT_DRIFT_THRESHOLD=0.20` (`drift.rs:22`), `ROLLING_CONFLICT_WINDOW=3` (`anomaly.rs:23`), skips `persona_version<=1` (`worker.rs:240`). That skip is the root of the Loop 2 deadlock. |
| `agent-bestiary/coherence-gate` | Loop 2 gate: `InterventionEncoder`, `CoherenceGate` (TEC settling of a correction against the world model), `TwoWriteMemory` | 1,289 / 6 | **pattern** (gate model, two-write); **kernel**: `gate.rs:199` `check_against` is pure | `gate.rs` pure; `two_write.rs` uses `MemoryStore` (async) | 16 + helpers (`tests.rs`): `gamma_alone_cannot_tell_the_two_cases_apart`, `a_correction_to_a_load_bearing_belief_is_blocked_at_the_production_threshold`, `an_absent_world_model_is_undetermined_not_approved`. **Meaningful.** | See Task B Loop 2 |
| `agent-bestiary/evaluator-wildguard` | Pre-filter safety evaluator (regex harm patterns + optional LLM) | 367 / 1 | **pattern** + **data** (`HARM_PATTERNS`, `lib.rs:50`) | reqwest (LLM option) | 4 | Per HANDOFF §10.2, has never flagged live traffic |
| `agent-bestiary/evaluator-faithfulness` | Pre-filter grounding evaluator | 350 / 1 | **pattern** / **drop?** | reqwest | 5 | LLM-judged |
| `agent-bestiary/evaluator-sotopia` | Social-goals dimensional evaluator | 447 / 1 | **drop?** (LLM judge, no hard signal) | reqwest | 4 | — |
| `agent-bestiary/evaluator-lifelong` | Persona-consistency drift/retention evaluator | 245 / 1 | **pattern** | no reqwest; tokio | 5 (`too_few_episodes_is_inapplicable`, `stable_persona_scores_high`) | Deterministic |
| `agent-bestiary/evaluator-character` | Persona-fidelity LLM evaluator | 484 / 1 | **drop?** | reqwest | 4 | LLM-judged |
| `agent-bestiary/ontology` | Ontology snapshots, Mermaid generation, git-backed workspace history (libgit2) | 2,248 / 7 | **pattern** (versioned ontology + git provenance) | sqlx (snapshot.rs), git2, clock | 29 (lib) + `tests/git_smoke`, `workspace_git_history` | `create_snapshot` had a 0% success rate for its whole life: it decoded a NULL aggregate into `(i32,)` (FEEDBACK_LOOPS §Loop 1, break #16) |
| `agent-bestiary/consolidate` | Standalone dreaming CLI | 563 / 1 | **drop?**: duplicates the API dreaming path | sqlx, tokio, clap | 0 | Was the only caller of `create_snapshot` before 2026-08-15 |
| `agent-bestiary/projector` | 2D/3D PCA of agent embeddings (linfa) | 678 / 5 | **kernel** (thin; PCA itself is linfa) / **pattern** | depends on `MemoryStore` (async), clock (`projection.rs:62,106,214`) | **0** | Untested. Oracle: sklearn PCA |
| `…/coherence/crates/coherence-core` | TEC formal system C=⟨U,E,R⁺,R⁻,A,σ⟩, 7 principles, relations, relevance gating, incoherence taxonomy | 4,028 / 8 | **kernel** | pure except `Utc::now` stamps (`system.rs:180,200,593,675,771`, `types.rs:237…`) | 77 (principles 23, types 14, relations 13, system 10, incoherence 8, relevance 7). Meaningful. | `incoherence.rs:46` LOW=0.5, `:49` HIGH=0.6, `:56` OPTIMAL_TENSION=(0.45,0.75); `relevance.rs:69-85` thresholds; `system.rs:46-49` initial activations. Port as θ data. |
| `…/coherence-engine` | ECHO-style connectionist settling + principle scoring (σ(Pₖ), Γ) | 1,010 / 3 | **kernel** | pure (chrono/uuid only) | 15 (settling 8, scoring 7) | `settling.rs:35-43` decay 0.05, η 0.05, ε 0.001, max 200 cycles; rule `:174` A←clip((1−δ)A+ηΣwA); `scoring.rs:17` UPTAKE_THRESHOLD 0.10 |
| `…/coherence-observer` | Heuristic utterance classification and relation detection (regex) | 629 / 4 | **kernel** (heuristic) / **pattern** | no tokio use in src (0 refs); regex | 16 | Feeds settling from chat text. Quality is heuristic. |
| `…/coherence-api` | Standalone axum REST server for the coherence evaluator | 256 / 3 | **discard** / **drop?** | axum, tokio | 0 | Not used by api-server [I] |
| `…/coherence-protocols` | `CoherenceProvider` trait + MCP tool def | 182 / 3 | **drop?** | reqwest, tokio | 2 | — |
| `fermi-auth` | OAuth/OIDC, SIWE, JWT, API keys, RBAC, visibility, teams, secrets (AES-GCM), **credits** | 5,643 / 14 | **discard** (auth plumbing); `credits.rs` **replaced:acn**; `rbac.rs`/`visibility.rs` **pattern** | sqlx, reqwest, ethers, aes-gcm, argon2 | 43 (middleware 17, jwt 9, visibility 4, types 5, siwe 3, oidc 3, rbac 2) | `teams.rs::assign_default_strategist` is the Loop 3/4 onboarding fix (the "column nobody wrote") |
| `fermi-lsp` | tower-lsp server for FPL: completions, hover | 1,963 / 13 | **drop?** / tooling | depends on `fermi` (lexer), tokio | 1 | Stripped from the Docker build. `tree-sitter-fpl/grammar.js` (171 l.) is a separate grammar and works as an independent parser oracle. |
| `crates/fermi-console` | GPUI desktop forecasting cockpit (FPL composer, routing, negotiation, plots) | 75,585 / 40 | **tenant:fermi-console**. Kernel-small inside: `plot/density.rs:212` Silverman KDE, `calibration.rs:260` Wilson interval, `plot/scale.rs`. Pattern: `negotiate.rs`, `routing.rs`, `coverage.rs`. | gpui, reqwest, rusqlite, tokio | 418 (routing 40, negotiate 41, coverage 24, calibration 23, wire 19…) | `cockpit.rs` 29,524 l., `main.rs` 23,603 l. `routing.rs:117` hardcoded `domain_specialist` table, `:711` `DeclaredSpecialist`. It is the **only producer** of `route_reason` (`src/route_trust.rs:12-16`), and it's stripped from Docker. |
| `crates/fermi-activity` | Activity event model for the console inspector | 779 / 1 | **tenant:fermi-console** | pure (chrono, serde_json) | 18 | — |
| `crates/simops` | SOSA-aligned resource-efficiency engine: cascade v1/v2 (mass resolution), process models, predictor/optimizer, KPI, slot-match | 5,501 / 11 | **tenant:simops** (pure, kernel quality) | pure (one `Utc::now` stamp, `cascade_v2.rs:505`) | 59 (cascade_v2 15, process 14, cascade 8, kpi 6, slot_match 5, predictor 5) | `slot_match.rs` was ported from kask JS (`kask-sim-client.js:7896-7980`), so the JS original is an oracle. Phase 4 `PredictorEngine::Conditional` was never built. |
| `crates/posterior` | BayesOps Phase 1: `fit_marginal` (Beta/Normal/Lognormal/Triangular/auto), bootstrap CI, `DataQuality`, extractors, `Feed` registry | 2,716 / 9 | **kernel** (fits, bootstrap, DataQuality); `feeds.rs`/`extractors.rs` **pattern** | pure; `bootstrap.rs:46` `from_entropy` when unseeded; `lib.rs:214` clock stamp | 69 (extractors 17, lib 16, feeds 7, beta 6, bootstrap 6…) | `lib.rs:237` `DataQuality::classify`: n_eff≥20 Sufficient, ≥5 Sparse, else Insufficient |
| `crates/posterior-reg` | BayesOps Phase 2/3: conditional regression via HMC, diagnostics (R-hat/ESS), NLPD improvement ladder, what-if queries | 2,372 / 7 | **kernel** | tokio `spawn_blocking` for chains (`sampler.rs:14,68`); clock stamp `lib.rs:366` | 39 lib + 6 e2e (`recovers_known_linear_posterior`, `prob_exceeds_is_calibrated`, `compare_scenarios_identifies_winner`, `optimise_for_target_finds_higher_x`, …). Meaningful. | Only `LinearNormal` exists (`models/`). The sampler is **vanilla HMC, not NUTS** (`sampler.rs:5`), although FEEDBACK_LOOPS §4 says "NUTS". `whatif.rs:100,218,227` Saltelli pick-freeze, fixed seed `0x50B0_50B0` (`:144`). |
| `crates/projections` | Distributional projection engine: N runs of a deterministic executor over sampled inputs | 1,009 / 8 | **kernel** / **pattern** | pure | 9 | `simops-executor` feature bridges to simops |
| `crates/dynamics` | ODE dynamics models (RK4 via `ode_solvers`), coupled models, rheology | 4,801 / 14 | **kernel** | pure ("No LLM, no I/O"); clock stamps only in `coupled.rs:411`, `provenance.rs:36` | 67 (coupled 11, kombucha_f2 11, solid_liquid 11, rheology 8, lib 8…) | Models: bc_optimization, kombucha_fermentation, kombucha_f2_carbonation, linear_decay, pellicle_growth, solid_liquid_extraction |
| `crates/abw-cli` | `abw` CLI: login, app new/validate/deploy/publish/spawn, workspace, admin | 3,000 / 13 | **discard** (reimplement against the new API) | reqwest, tokio | **0** | — |
| `crates/abw-apps-core` | Pure App manifest builder + validator | 788 / 1 | **pattern** + **data** (manifest schema) | pure (serde only) | 11 | Shared by CLI and server |

### A.2 Root crate `fermi` — `src/` modules

#### A.2.1 FPL language core

| path | purpose | lines | bin | purity evidence | tests | note |
|---|---|---|---|---|---|---|
| `src/lexer.rs` | FPL tokenizer | 965 | **kernel** | pure | 11 (`test_keywords`, `test_probability`, `test_date`, `test_error_unterminated_string`, …) | Basic but real |
| `src/parser.rs` | FPL recursive-descent parser → AST, incl. `learnable`/`feeds_from` | 2,018 | **kernel** | pure | 11 (`test_parse_learnable_driver_with_feeds_from`, `test_parse_complete_forecast`, …) | Corpus: `forecasts/*.fpl` (42 + 29 archive), `templates/*.fpl`, `examples/*.fpl` |
| `src/ast.rs` | AST incl. Distribution enum (the BayesOps seam) and factor-model extensions | 898 | **kernel** / **data** | pure | 3 | `Distribution` is the Loop 5.B seam (FEEDBACK_LOOPS §4) |
| `src/semantic.rs` | Semantic analysis | 1,830 | **kernel** | pure | 22 | — |
| `src/symbol_table.rs` | Symbols, driver usage | 334 | **kernel** | pure | 3 | — |
| `src/types.rs` | FPL type system, coercion, op typing | 286 | **kernel** | pure | 5 | — |
| `src/evaluator.rs` | Expression evaluator | 696 | **kernel** | pure | 10 | — |
| `src/executor.rs` | Monte Carlo executor (incl. factor model), learnable-driver resolution | 1,305 | **kernel** | pure; RNG `from_entropy` by default (`:144,231`), seeded via `with_seed` (`:251`) | **3 only** (`test_executor_simple`, `test_executor_arithmetic`, `test_histogram`) | **BTreeMap driver maps are load-bearing for seed reproducibility** (`:286-310`). Most coverage is indirect, via attribution and corpus tests. |
| `src/distributions.rs` | Samplers: triangular, normal, lognormal, uniform, beta, categorical; stats | 455 | **kernel** | pure | 8 | **Triangular treats p5/p95 as min/max** (`:30-40`, "close approximation"). This is a semantic decision. |
| `src/sensitivity.rs` | Sobol first-order (nested MC) + total-order (Saltelli) for FPL | 1,042 | **kernel** | pure, but **unseeded** executors (`:88,377,453,617,725` `Executor::new`) | 6 property tests (`total_order_is_never_below_first_order`, `a_binary_driver_index_is_stable_across_runs`, …) | One of two Sobol implementations; the other is `posterior-reg/whatif.rs`. |
| `src/report/` | Markdown, Mermaid, chart and sparkline reports from `ExecutionResults` | 1,528 / 7 | **drop?** (pure rendering) | pure; one clock ref | 5 | — |
| `src/main.rs` | `fermi` REPL/CLI | 809 | **drop?** | pure I/O | 0 | — |

#### A.2.2 Attribution, calibration, loops (core of Task B)

| path | purpose | lines | bin | purity evidence | tests | note |
|---|---|---|---|---|---|---|
| `src/attribution/mod.rs` | Exact Shapley, pairwise interaction index, Brier, value fn, `stable_seed`, cluster bootstrap CI | 764 | **kernel** | pure | 18. Axioms (`efficiency_holds_on_a_game_with_interactions`, `dummy_player_receives_exactly_zero`, `symmetric_players…`, `additive_across_games`), `clustering_widens_the_interval_versus_ignoring_it`, `value_fn_evaluated_once_per_subset`. **Highly meaningful.** | See Task B §Shapley |
| `src/attribution/counterfactual.rs` | FPL counterfactual value function: 2ⁿ subset re-runs, Identity/Reference neutralisation | 650 | **kernel** | pure (uses executor) | 12 (`empty_subset_reproduces_base_rate` 0.0208, `agent_claiming_neutral_earns_nothing`, `cached_interactions_match_fresh_model_runs`, `unclaimed_declared_drivers_neutralise_instead_of_failing`) | — |
| `src/attribution/roster.rs` | Resolve `agents_used` statement names → agents (exact match, then longest `<agent>_` prefix) | 314 | **pattern** | pure | 9 | The heuristic exists because FPL statement names ≠ agent ids |
| `src/calibration.rs` | Loop 5.A `compute_agent_calibration`, `brier_skill`, `evidence_class`, gated Shapley contribution | 695 | **kernel-small** (`brier_skill` `:586`, `evidence_class` `:573`) + **pattern** | sqlx (7) in `compute_agent_calibration` | 7 | Shapley gates in SQL at `:324-325` |
| `src/intentions.rs` | Loop 3 Stage 0 conflict detection over declared intentions | 624 | **kernel-small** (`detect_conflicts` `:162`) | pure | 14 | — |
| `src/plan_solicitation.rs` | Loop 3 Stage 0 platform floor: asks members for plans | 808 | **pattern** | sqlx (9), async | 9 | `FRESHNESS_SECS=600` (`:86`), `MAX_PER_RUN=8` (`:99`) |
| `src/coordination_note.rs` | Loop 3→1 cascade: write a coordination episode into a member's memory | 264 | **pattern** | sqlx, clock | 2 | authority_weight 0.6 (`:190`) |
| `src/loop_model.rs` | The loops declared as chains of links with Trigger kinds | 1,144 | **pattern** + **data** | sqlx (2) | 9 | `Trigger::{Request,Scheduler,Upstream,Manual,Prompted,None}` (`:176-264`). Good model for "wired vs turning". |
| `src/loop_api.rs` | Assembles loop state views | 1,309 | **pattern** | pure | 14 | — |
| `src/projection_kind.rs` | Projection vs measurement predicate (Rust + SQL from the same consts) | 181 | **pattern** + **data** | pure | 5 | Fixed the "12,167 phantom" |
| `src/projection_commit.rs` | Commitment anchor: proves a prediction pre-dated its measurement | 200 | **pattern** | sqlx | 3 | Called at `handlers/observations.rs:617`, `agent_backend/simops_tools.rs:700` |
| `src/claim_outcome.rs` | What became of a run's quantified judgement | 344 | **pattern** | pure | 4 | — |
| `src/assertions.rs` | Quantified assertions from agent text; `Suggested p50` parser v2 | 1,689 | **pattern** + **kernel-small** (regex `:638-639`) | pure | 37 | v1 regex couldn't match `**1.15**` (HANDOFF §4.2) |
| `src/episodes.rs` | The single constructor AgentOutput → Episode | 682 | **pattern** (episode lifecycle) | pure; clock (1) | 11 | — |
| `src/episode_boundary.rs` | The one place a pulse becomes a row (reserve, contract, grade, route, gate ledger, queue) | 753 | **pattern** | sqlx, async | 3 | — |
| `src/provenance_oracle.rs` | Grades evidence grounding for memory (memory crate trait impl) | 402 | **pattern** | sqlx (4) | 9 | — |
| `src/grounding_anomaly.rs` | Grounding violation → Loop 2 anomaly (deadlock seed) | 189 | **pattern** | async spawn | 2 | Severity `L1` was rejected by a CHECK constraint (HANDOFF §10) |
| `src/anomaly_vocabulary.rs` | Declared anomaly kinds, severities, flag prefixes | 231 | **data** | pure | 7 | — |
| `src/native_evaluators.rs` | Evaluators of the platform's own machinery | 1,279 | **pattern** | sqlx (10) | 12 | — |
| `src/evaluator_api.rs` | What platform self-evaluations mean | 401 | **pattern** | pure | 6 | — |
| `src/route_trust.rs` | Server-recorded route selection (`CallerNamed`) | 207 | **pattern** | pure | 5 | `route:` tag was on 0/3,581 episodes (`:8-10`) |

#### A.2.3 Contracts, gates, verification ladder (mostly the "trust" substrate)

| path | purpose | lines | bin | purity evidence | tests | note |
|---|---|---|---|---|---|---|
| `src/grounding_trust.rs` | Rung 4: field contracts mapping output fields → tool sources; provenance vocabulary; `floor()` lattice; cross-checks | 8,979 | **data** (`FIELD_CONTRACTS` `:1963`, `PROV_*` `:65-136`, `NARRATIVE_LEAKS` `:1795`) + **pattern** (`floor` `:1424`, `EXTRACTION_CEILING` `:1401`); enforcement runtime **discard** | pure | 88 | **Needs owner decision** (largest file in src) |
| `src/card_contract.rs` | Card-declared contracts (third-party replacement for the const tables) | 908 | **pattern** | pure | 17 | — |
| `src/contract_sketch.rs` | Compile a typed output contract from a sketch | 2,860 | **pattern** | pure | 42 | Bins `contract-sketch`, `input-contract-sketch` |
| `src/tool_response_shapes.rs` | What each tool actually returns | 924 | **data** | pure | 7 | — |
| `src/schema_validate.rs` | Minimal, non-guessing JSON Schema validator | 517 | **kernel-small** / **pattern** | pure | 12 | Could be replaced by a library |
| `src/schema_conformance.rs` | Did the member's document conform, and is it improving | 307 | **pattern** | sqlx (3) | 7 | — |
| `src/port_trust.rs` | Rung 5: invocation matches declared interface (InputBinding gate) | 872 | **pattern** | pure | 17 | — |
| `src/declaration_ladder.rs` | What an agent must declare before anything can be said about it | 732 | **pattern** + **data** | pure | 9 | — |
| `src/completeness.rs` | Did the agent fill the contracted fields | 353 | **pattern** | pure | 7 | — |
| `src/field_state.rs` | One field, two clocks (contract-time vs run-time trust) | 1,320 | **pattern** | pure | 13 | — |
| `src/field_probe.rs` | Runs the tool a field contract names | 715 | **pattern** | async | 11 | — |
| `src/verification_queue.rs` | Enqueue contracted fields for verification | 507 | **pattern** | sqlx (3) | 8 | — |
| `src/gate_trust.rs` | Gate decision ledger (`Gate` enum `:98`, `decided_about`) | 1,210 | **pattern** (gate model) | sqlx (3), statics (7), clock | 14 | Fixed "no gate decision persisted anywhere" |
| `src/gate_review.rs` | Was the gate right (human review of a decision) | 508 | **pattern** | sqlx (2) | 7 | — |
| `src/gate_api.rs` | Gate stance views | 684 | **pattern** | pure | 10 | — |
| `src/ladder.rs` | Verification ladder rungs (Presence, Liveness, Truth, Grounding, Binding) | 207 | **data** (`LADDER` `:66`) | pure | 5 | The rung *names* are worth keeping; the record-only checks are discarded |
| `src/schema_trust.rs` | Rung 1: boot-time schema drift check vs a manifest of tables/columns | 1,127 | **discard** | sqlx (7) | 0 (lib); `tests/schema_trust_contract.rs` 13 | Tied to legacy schema |
| `src/rollup_trust.rs` | Columns that exist but lie | 321 | **discard** | pure | 4 | Legacy-schema specific |
| `src/liveness_trust.rs` | Rung 2: does the writer ever run (sweeper) | 1,358 | **discard** (keep the *idea*: wired vs turning) | sqlx (8), async | 16 | `spawn_liveness_sweeper` (`api_server.rs:2742`) |
| `src/outcome_trust.rs` | Does a loop's output carry the signal its claim requires | 857 | **discard** / **pattern?** | pure | 16 | — |
| `src/write_accounting.rs` | Was the write *attempted* | 604 | **discard** | statics (4), clock | 10 | Named in the brief |
| `src/seam_vocabulary.rs` | Closed token sets checked against CHECK constraints | 899 | **data** (`VOCABULARIES` `:399`, `EPISODE_PROVENANCE` `:347`) + checks **discard** | sqlx (7) | 9 | — |
| `src/command_registry.rs` | Every verb and the gate in front of it | 1,052 | **pattern** + **data** | statics (3) | 8 | Useful as the new core's capability map |
| `src/surface.rs` | Shared trust-surface shape | 251 | **pattern** | pure | 5 | — |
| `src/reliance.rs` | "What a caller may rely on" token | 398 | **pattern** | pure | 6 | — |
| `src/artifact_trace.rs` | Per-artifact checkpoint trace | 1,333 | **pattern** | pure | 14 | — |
| `src/artifact_hash.rs` | Input/output digests computed on read | 328 | **pattern** | pure | 5 | — |
| `src/panel_absence.rs` | Stamped "why is this panel empty" | 1,835 | **discard** (UI) | sqlx (3) | 18 | — |
| `src/panel_contract.rs` | Panel stamp at three densities | 459 | **discard** / tenant:glasses | pure | 9 | — |
| `src/fleet_digest.rs` | Fleet shape in fixed lines | 475 | **drop?** | pure | 7 | — |
| `src/taxonomy.rs` | Derived agent taxonomy ranks (SPEC_30) | 452 | **data** (`RANKS`, `ORDER_PATTERNS` `:35-79`) | pure | 7 | Plus `agents/taxonomy_*.json` |
| `src/pipeline.rs` | Validate a declared `workflow_template` (planning only) | 627 | **pattern** | pure | 14 | — |
| `src/slug.rs` | Slug validation rules | 142 | **pattern** (tiny) | axum (2) | 8 | — |
| `src/attachments.rs` | Image attachments; a dropped frame is an error | 569 | **pattern** | pure | 16 | — |

#### A.2.4 Tenant, economics, plumbing

| path | purpose | lines | bin | purity evidence | tests | note |
|---|---|---|---|---|---|---|
| `src/gas.rs` | A2A gas fees, royalties, charge-and-distribute | 607 | **replaced:acn** | sqlx (11), axum | 0 | — |
| `src/agent_economics.rs` | Measured exec stats rollup | 214 | **replaced:acn** | sqlx | 4 | — |
| `src/email.rs` | Resend email | 434 | **discard** | reqwest | 6 | — |
| `src/a2a_card.rs`, `a2a_task.rs`, `a2a_webhook.rs` | A2A AgentCard mapping, task JSON, webhook delivery | 292 / 294 / 160 | **pattern** (card mapping, pure) / **discard** (webhook) | card pure; webhook sqlx+reqwest | 7 / 5 / 0 | — |
| `src/glasses_shell.rs` | Generated AIUI glasses app shells | 867 | **tenant:glasses** | pure | 9 | `glasses/`, example `new_glasses_app` |
| `src/hud_contract.rs` | HUD glanceable-card contract | 1,169 | **tenant:glasses** | pure | 9 (+ `tests/hud_contract.rs` 20) | — |
| `src/lens_rendering.rs` | Grounding gate for regulatory lens translator | 765 | **tenant:dpp** | pure | 10 | — |
| `src/verification.rs` | Verification corpus (species ID safety; Hodgson 2023) | 955 | **tenant:wild** | pure | 18 | — |
| `src/api_server.rs` | api-server binary: router, rate limiter, sweepers, `stamp_invocation` | 7,569 | **discard**. Keep `stamp_invocation` semantics (`:7116`) as **pattern**. | sqlx, axum, clock | 14 | Sweepers at `:2707,2728,2742,2759` |
| `src/lib.rs` | Module wiring | 582 | **discard** | — | 0 | — |
| `src/agent_backend/` (41 files) | Agent execution runtime, tools, KG context | 32,712 | split ↓ | sqlx 73, reqwest 31, async 290 | 287 | — |
| ↳ `kg_context.rs` | Loop 1 retrieval gate + KG block injection | 923 | **pattern** | sqlx (6) | 11 (`gate_tests` `:469`, `retrieved_knowledge_reaches_the_prompt_text`) | `MIN_SIMILARITY=0.30` (`:24`), top-k 5 rules / 8 entities (`:711-712`) |
| ↳ `envelope.rs` | Delegation envelope (provenance survives a hop) | 1,099 | **pattern** | pure | 24 | — |
| ↳ `coordination_graph.rs` | Typed workflow-graph executor (3 topologies) | 480 | **pattern** | pure | **0** | Untested |
| ↳ `rate_card.rs`, `funding_parity_tests.rs`, `credentials.rs` | Provider cost basis, funding parity, per-execution creds | 720 / 278 / 287 | **replaced:acn** / **replaced:acn** / **discard** | pure | 16 / 8 / 4 | — |
| ↳ `llm_executor.rs`, `multi_model_executor.rs`, `mcp_client.rs`, `executor.rs`, `registry.rs`, `tool_executor.rs`, `agent_card.rs` | LLM provider calls, MCP client, tool dispatch, card type | ~7,200 | **discard** (runtime); `agent_card.rs` → **data** (card schema) | reqwest, async | 23+15+22+17+4+4 | — |
| ↳ `tools/` (`domains/*`, `skills`, `mod`) | ~200 tool defs across 15 domains | ~13,000 | **pattern** (tool contract: `BuiltinToolDef`, `requires_workspace`); domains → **tenant:*** (biology, rabble, spatial, video, weather, simops, workspace…); `coordination.rs` → **pattern** | mixed sqlx | ~100 | `coordination.rs:394,701,766` dual gate on `record_coordination_observation` (`:1185`); `propose_composition_change` (`:1233`); `solicit_agent_plan` (`:912`) |
| ↳ `weather_tools.rs` | Weather/prediction-market tools | 4,175 | **tenant:weather**. Kernel-small: `ols` `:1723`, `quantile` `:1110`, `ladder_kelly` `:2668`, `log_growth` `:2716`, `corr` `:2588`. | reqwest | 27 | — |
| ↳ `simops_tools.rs`, `ncbi_tools.rs` | SimOps tool bridge; NCBI assembly | 1,334 / 341 | **tenant:simops** / **tenant:bio** | sqlx / reqwest | 0 / 6 | — |
| `src/polymarket/mod.rs` | Gamma API client, liquidity classification | 1,246 | **tenant:polymarket** | reqwest | 13 | — |
| `src/feeds/mod.rs` | BayesOps `FeedRegistry` impls (Loop 5.B intake) | 345 | **pattern** | sqlx (9) | 0 | — |
| `src/apps/` | App primitive; workspace → draft manifest | 514 / 3 | **pattern** | sqlx (9) | 2 | — |
| `src/workflows/` | Agent contract (one definition, two authoring paths), state machine, publish pipeline, fork | 1,838 / 6 | `agent_contract.rs` **pattern** + **data**; `agent_state.rs` **data**; `publish_pipeline.rs` **pattern**; `fork.rs` **replaced:acn** (fork pricing) | sqlx in fork/publish | 21 | — |
| `src/api/` | Legacy REST API (used only by `src/bin/legacy`) | 296 / 4 | **drop?** | axum | 1 | Dead [I] |
| `src/voice/` | Cartesia TTS client | 145 / 2 | **drop?** / tenant | reqwest | 0 | — |
| `src/bin/` | `agent-mcp-server` (MCP surface incl. 7 BayesOps tools); WC one-offs; `legacy/` | 2,163 / 6 | `agent-mcp-server` **pattern** (tool surface); `resim_wc`, `apply_wc_cascades`, `legacy/*` **drop?** | sqlx | 0 | — |
| `src/handlers/` (113 files) | All HTTP handlers | 102,352 | mostly **discard** (write paths); exceptions ↓ | sqlx 1,535 refs, axum 186 | 263 | — |
| ↳ `attribution.rs` | Resolution-time Shapley job + validity gates + persist | 893 | **pattern** (gates are carry-over **data**) | sqlx | 5 DB tests (`full_pipeline_persists_gated_credit`, `reconstruction_mismatch_is_recorded_not_suppressed`, `claims_made_after_resolution_are_excluded`) | — |
| ↳ `composition_evolution.rs` | Loop 4.A proposals from Shapley evidence | 576 | **pattern**; `derive_proposals` (`:99`) **kernel-small** | sqlx | 8 | — |
| ↳ `composition.rs` | Composition versions, accept/reject, composition dreaming (3.B) | 414 | **pattern** | sqlx | 0 | — |
| ↳ `workspace/refit.rs` | Loop 5.B refit + impact gate | 1,563 | **pattern**; `classify_decision` (`:887`) **kernel-small** | sqlx | 12 | — |
| ↳ `consolidation.rs` | Dreaming endpoints, sweeper, snapshot, extractor resolution | 1,636 | **pattern** (consolidation orchestration) | sqlx | 5 | — |
| ↳ `observatory.rs` | Plane D API, HITL queue, interventions, consensus, Loops tab | 3,873 | **pattern** (HITL flow) | sqlx (30) | 24 | Locally modified |
| ↳ `live_observability.rs`, `dreaming_maturity.rs`, `evolution.rs`, `loops.rs`, `fleet.rs` | Loop instrumentation, maturity, agent evolution ledger | 335 / 887 / 1,494 / 2,009 / 470 | **pattern** (instrumentation); `loops.rs` **discard** | sqlx | 3 / 12 / 21 / 0 / 2 | — |
| ↳ `relationships/*` (9 files) | Spec 25 forecast groups: mutex / at_most_n / implies propagation, undo, requeue | 2,139 | **kernel** (math in `propagation.rs:232,355,492`, needs extraction from async sqlx fns) + **pattern** (pending-cascade queue, undo) | sqlx | **0 in-module**; `tests/relationships_mutex_math.rs` (6) tests a *mirror copy* (`:25-30`), not the prod fn | FLOOR 0.001 / CEIL 0.999 (`propagation.rs:16-17`) |
| ↳ `eval*.rs`, `ground.rs`, `forecast_benchmark.rs`, `simops_benchmark.rs`, `observations.rs` | Eval runs, Brier/projection lookups, grounding service, commitment/spacetime, SOSA ingest | ~6,200 | **pattern** (benchmark commitment anchor, SOSA observation model); rest **discard** | sqlx | eval_projection 6, ground 2 | — |
| ↳ `creatures/*`, `rabble_*.rs`, `social.rs`, `swarm_*.rs`, `beacons.rs`, `streams.rs`, `wild.rs`, `qr_codes.rs` | Creature/Rabble/AR/swarm app | ~17,000 | **tenant:rabble** / **tenant:wild** | sqlx-heavy | 0 | — |
| ↳ `workspace/{carbon,bom_pricing,claim_evaluation,dpp_publish,lens_actions}.rs` | DPP Studio | ~6,860 | **tenant:dpp** | sqlx | 16+7+0+5+4 | — |
| ↳ `billing.rs`, `wallet.rs`, `agent_wallet.rs`, `agent_funding.rs`, `economics.rs`, `sql/economics_*.sql` | Stripe, credits, wallets, earnings | ~1,650 | **replaced:acn** | sqlx | 1+2 | — |
| ↳ remaining (`agents`, `forecasts`, `auth`, `admin*`, `apps`, `teams`, `invites`, `shares`, `pages`, `execution*`, `workspace/{core,messages,coherence,outputs,resolution}`, `polymarket`, `bayesops`, `xaman`, …) | CRUD/plumbing and HTTP surfaces | ~60,000 | **discard**. Loop wiring inside is documented in Task B. | sqlx | — | `forecasts.rs:1461-1515` Loop 4.B outcome annotation; `workspace/messages.rs:916-966` auto-coherence; `workspace/coherence.rs:130,282-316,820` shelf |

#### A.2.5 Non-code assets to port as data (not crates, but losable)

| path | what | bin |
|---|---|---|
| `agents/curated/` (104 dirs) | Agent cards incl. prompts, tool declarations, `output_contract.sketch.json` | **data** |
| `agents/{taxonomy_vocab,taxonomy_derived_expected,port_binding_expected,port_burndown_baseline,taxonomy_proposals}.json`, `agents/templates/ontology_seeds/*.json` | Vocabularies, expected fixtures, ontology seeds | **data** |
| `migrations/` (248 files) | Schema history; esp. 187 `forecast_agent_claims`, 188 `forecast_attributions`, 193 route views, 148 BayesOps ledger, 212 `member_delta`, 218 intentions `declared_by` | **data** (reference only) |
| `forecasts/*.fpl`, `templates/*.fpl`, `templates/world_cup`, `templates/weather`, `examples/*.fpl` | FPL corpus | **data** (parser/executor oracle corpus) |
| `tree-sitter-fpl/grammar.js` | Independent FPL grammar | **data** / oracle |
| `scripts/*.sql` (26), e.g. `loop1_retrievability_census.sql`, `loop5_brier_mechanical_check.sql` | Loop probes | **pattern** (probe designs) |
| `apps/*.json`, `glasses/`, `rabble-web/`, `fermi-deign/` | Tenant app manifests and frontends | **tenant:*** |
| `abw-invariant-compiler-kit/` (untracked) | Invariant rules/ratchets | **Needs owner decision**: not in git |

---

## Task B — loops and strategies

Status vocabulary, used consistently: **fires-in-tests** means a test exercises the gate's decision. **fires-in-code** means a production call site reaches it. **observed** means the docs report it moving on production data. **declared-only** means a document or prompt names it and no code computes it.

### Loop 1 — Individual agent learning

- **Changes:** the agent's semantic memory (entities/facts/semantic_rules). These are retrieved into the system prompt via the KG block. It is not a weight update.
- **Gates:**
  - **Retrieval gate:** `retrievable_knowledge` (`src/agent_backend/kg_context.rs:407`) queries the knowledge tables directly. It distinguishes three states: no rows, rows with no embedding ("stranded"), and retrievable. ANN top-k is 5 rules and 8 entities at `MIN_SIMILARITY=0.30` (`kg_context.rs:24,711-712`). `cep_*` entities are always injected.
  - **Refuse-degraded:** the API path refuses when there is no extraction LLM unless `?allow_degraded=true` (`src/handlers/consolidation.rs:1017`). The creature path also refuses now.
  - **Data-loss guard:** episodes aren't marked consolidated by a zero-yield run (doc §Loop 1).
  - **Hard-verified signals bypass any coherence gate** by design.
- **Algorithm:**
  - DBSCAN over episode embeddings with cosine distance, `eps=0.5`, `min_samples=2` (`handlers/consolidation.rs:930,1178`; `creatures/agent_modules.rs:1771`; impl `agent-bestiary/memory/src/clustering.rs:28-120`). Failure clusters become semantic rules.
  - Success episodes go to LLM rule extraction with **budget 30, ranked by authority_weight first** (`memory/src/consolidation.rs:862,998`).
  - Digest budgets: response 1,200 chars, context 200 (`consolidation.rs:301,308`).
- **Cadence:** `spawn_consolidation_sweeper` (`handlers/consolidation.rs:760`, started `api_server.rs:2759`). It is **opt-in**, with `CONSOLIDATION_SWEEP_SECS` defaulting to 0 (`:770`), because it debits wallets.
- **Observation leg:** `spawn_live_observation` is called from `execution.rs:753`, `execution_stream.rs:582`, `workspace/messages.rs:698`, `a2a.rs:912` and `rabble_workspace.rs:500`. A sweeper runs at `api_server.rs:2728`.
- **Status:**
  - fires-in-tests: `kg_context::gate_tests` (`kg_context.rs:469`, incl. `retrieved_knowledge_reaches_the_prompt_text`) and `consolidation::authority_tests` (`memory/src/consolidation.rs:2570`).
  - fires-in-code: yes.
  - observed: "8 eval runs… 23 ontology rows" and 1,096 embedded facts (FEEDBACK_LOOPS §5).
  - `ontology_snapshots` had never succeeded as of 2026-08-21.
- **Carry over:**
  - The DBSCAN with exact semantics. `find_neighbors` **excludes the point itself** (`clustering.rs:49-51`), so `min_samples=2` here equals sklearn `min_samples=3`.
  - The θ values: eps 0.5, min 2, MIN_SIMILARITY 0.30, top-k 5/8, budget 30, digest 1,200/200.
  - The authority-first ranking. The three-state retrieval gate.
  - The rule "unembedded knowledge is unreachable".
  - Embedding provenance (`ProvenancedEmbedding`).
  - Seed idempotency per `(entity_name, entity_type)`.
  - Refuse-rather-than-run-degraded.

### Loop 2 — HITL behavioral correction

- **Changes:** the agent's persona/episodic memory. It writes a `SyntheticCorrection` episode (authority_weight 1.0, **embedded at write**) and an immutable `episode_corrections` row. An AgentWide correction bumps `persona_version`, which gives drift a new baseline (`coherence-gate/src/two_write.rs:186,201`).
- **Gate:** `CoherenceGate::check_against` (`agent-bestiary/coherence-gate/src/gate.rs:199`), checked in this order:
  1. `!world.is_sufficient()` (target present and ≥2 nodes, `:111`) ⇒ **Undetermined**. This is not a pass.
  2. Scope is Episode or Dyad ⇒ **Settled**. Advisory only; it never blocks.
  3. AgentWide and the correction's own post-settling activation <0 ⇒ **Blocked** (primary, `:233`).
  4. Γ < `DEFAULT_GATE_THRESHOLD=0.5` (`:66,240`) ⇒ **Blocked** (secondary floor).
  5. Otherwise **Approved**.

  After the gate, AgentWide needs a **second, different reviewer** (`src/handlers/observatory.rs:761`, 403 if the same user). The world model is built from the agent's episodes and rules: `build_agent_world_model` (`observatory.rs:366`), invoked at `:538-540`. Decisions are recorded to the gate ledger as `Gate::Coherence` (`gate_trust.rs:100`).
- **Status:**
  - fires-in-tests: strongly. `gamma_alone_cannot_tell_the_two_cases_apart` (Γ=0.632 in both cases), peripheral-vs-load-bearing.
  - fires-in-code: yes.
  - **observed: never.** `anomaly_events` is 0, and the loop is a deadlock: drift skips `persona_version<=1` (`observability/src/worker.rs:240`), and only an AgentWide intervention bumps it. The grounding-anomaly seed (`src/grounding_anomaly.rs`) was rejected by a CHECK constraint until it was fixed (HANDOFF §10).
- **Defect found during harvest [verified in code]:** the consensus confirm path re-runs `gate.check(&encoded)` with **no world model** (`observatory.rs:811-812`). That always yields `Undetermined`, and that throwaway outcome is what `TwoWriteMemory::execute` writes into `episode_corrections.coherence_check` (`two_write.rs:162-182`). So the audit trail for every AgentWide correction records "Undetermined", not the verdict the first reviewer passed. It also never re-checks against a world model that may have changed between the two reviews.
- **Carry over:**
  - Gate semantics and ordering. The Γ-is-not-the-discriminator finding and its test fixtures.
  - The four verdicts. Settler mode for Episode/Dyad. Two different reviewers for AgentWide.
  - Embed-at-write with the note "embedding failure still stores".
  - Authority 1.0 > 0.6 (coordinator) > ordinary.
  - The DRIFT threshold 0.20 and the `persona_version` baseline rule. **Fix the deadlock by design**: seed anomalies from hard-verified violations.

### Loop 3.A — Workspace coherence, inner/prospective

- **Changes:**
  - The current conversation, via a `coherence_update` system message.
  - Members' memory, via a **coordination-observation episode** (authority 0.6, embedded, unconsolidated) in each member's memory (`src/coordination_note.rs:88,190`). This is the Loop 3→1 cascade.
  - Solicited first-hand member plans for Stage 0.
- **Measurement:**
  - `SettlingEngine::with_defaults()`: decay 0.05, η 0.05, ε 0.001, max 200 cycles, update rule at `coherence-engine/src/settling.rs:174`, convergence at `:190`.
  - Γ = mean positive contribution of scorable activations (`coherence-core/src/system.rs` ≈`:188-190`).
  - The 7 principles are in `principles.rs`.
  - `classify` (`coherence-core/src/incoherence.rs:181`) assigns Coherent/Destructive/ProductiveCompetitive/Analogical/Contradictory, with LOW 0.5, HIGH 0.6, and a **TensionBand** with `OPTIMAL_TENSION=(0.45,0.75)` where "Above" means homophily risk (`:46-56,71`). `is_productive`/`should_remedy` are consumed at `workspace/messages.rs:977-978` and `workspace/coherence.rs:315-316`.
- **Gates:**
  - The cascade tool is **dual-gated**: the caller must be the workspace's registered `coordination_strategist_id` (`tools/domains/coordination.rs:701`), and the target must be a current member (`:394,766`).
  - The plan-solicitation floor runs *before* the strategist, only at `depth=recommendations`, with `FRESHNESS_SECS=600` and `MAX_PER_RUN=8`, concurrently (`src/plan_solicitation.rs:86,99,282`, called `workspace/coherence.rs:130`).
  - Auto-eval runs every `COHERENCE_AUTO_EVAL_INTERVAL` messages (default 10, `workspace/messages.rs:916-919`). It does **not** invoke the strategist.
- **Status:**
  - fires-in-tests: coherence-core 77 + engine 15; `tests/coordination_loop_closure.rs` (15, a source scan); `intentions` 14; `plan_solicitation` 9.
  - fires-in-code: shelf `workspace/coherence.rs:282-316,820`.
  - observed: "6 evaluations across 1/127 workspaces, Γ(C) mean 0.97" (FEEDBACK_LOOPS §5).
  - **[I] A Γ of 0.97 is *Above* the module's own optimal band.** By its own taxonomy that's homophily risk, not success.
- **Carry over:**
  - ECHO settling plus its θ. The principle scorers. The incoherence taxonomy, with Destructive tested first.
  - The tension band as a *range*. The relevance θ (`relevance.rs:69-85`).
  - The dual gate on cross-agent memory writes (it's a memory-poisoning primitive).
  - Platform floor before the model (the "judgement is the model's, round trip is the platform's" split).
  - The strategist resolved from the workspace, never a literal.
  - The `declared_by`/`source='solicited'|'inferred'` distinction.

### Loop 3.B — Workspace coherence, outer (composition dreaming)

- **Changes:** produces a qualitative composition proposal (`propose_composition_change`, `tools/domains/coordination.rs:1233`). That proposal goes into Loop 4.A's `composition_versions` and names no members by design.
- **Gate:** owner accept/reject (`handlers/composition.rs:78`, `require_workspace_owner_or_admin`). A rejection writes a HumanCorrected episode into the strategist's memory. Dreaming charges 5 credits (`composition.rs:394`).
- **Status:** **mostly declared-only.**
  - The valence-homophily threshold "spread < 0.25" exists **only as prompt text** (`composition.rs:359`).
  - `homophily_detected` is a **caller-supplied bool** (`composition.rs:247,262`). Break #10 is still open.
  - The computed `homophily_risk` (TensionBand Above) in `incoherence.rs` is a partial substitute [I].
  - Never observed (no session history).
- **Carry over:**
  - Either compute valence/arousal spread or drop the claim. The 0.25 θ is the only spec.
  - "Proposals are never applied automatically."
  - Accepted ≠ applied (`composition.rs:91-109`).

### Loop 4.A — Composition evolution (tune-team RSI)

- **Changes:** team membership, via a `composition_versions` row. Accept reconciles `workspace_agents` transactionally, additive-first, with the strategist exempt, using `member_delta` (mig 212).
- **Gate:**
  - **Quantitative generator:** `derive_proposals` (`src/handlers/composition_evolution.rs:99`) proposes removal only when `mean_credit < 0` **and** `n_forecasts ≥ MIN_FORECASTS_FOR_PROPOSAL=5` (`:57,108`). A redundant pair needs `mean_interaction < REDUNDANCY_THRESHOLD=−0.005`, a synergy pair `> SYNERGY_THRESHOLD=0.005` (`:61,64,139,191`).
  - Evidence is read **only through the Shapley validity gates** (`:234-235,262-263`).
  - Final gate: owner accept/reject (`composition.rs:78`).
- **Status:**
  - fires-in-tests: 5 tests at `composition_evolution.rs:484-530` (`thin_evidence_proposes_nothing`, `redundancy_without_credit_withholds_a_verdict`).
  - fires-in-code: the suggestions and materialise handlers (`:302,343`).
  - **observed: 0 proposals ever.** No composition identity exists, and the upstream Shapley claims ledger is INERT (see §Shapley).
- **Carry over:**
  - The thresholds. "A negative φ may be a model error, so never auto-prune."
  - Every proposal carries its sample size.
  - The synergy/redundancy reading of the interaction sign. Strategist eviction exemption.
  - Two generators, one mechanism.

### Loop 4.B — Routing accuracy

- **Changes:** which member is selected for a query.
- **Mechanism:**
  - The console router: `DeclaredSpecialist` from card `domains` is checked first (`crates/fermi-console/src/routing.rs:711`), ahead of the hardcoded `domain_specialist` table (`:117`).
  - Route provenance tags `route:{reason}`, `qsrc:*`, `ibind:*`, slugged ≤64 chars so they can't be forged (`src/api_server.rs:7116` `stamp_invocation`). Server-side `RouteSelection::CallerNamed` (`src/route_trust.rs`).
  - On resolve, `moe_routing_decision` episodes from the last 7 days get `outcome_quality = 1 − brier` (`handlers/forecasts.rs:1461-1515`).
  - SQL views `route_outcomes`, `route_reason_performance`, `domain_agent_ranking`, `router_override_scorecard`, `declaration_quality_outcomes` (mig 193).
  - The router *agent* reads calibration via the `get_agent_calibration` tool (`tools/domains/observability.rs:47`).
- **Gate:** **none deterministic.** The decision is made by an LLM card (`moe_router_strategist`), informed by `evidence_class`. The views are read by nothing in Rust except as docs/metrics [I].
- **Status:**
  - fires-in-tests: routing 40 tests (console).
  - fires-in-code: only on the console path.
  - **observed: the `route:` tag was on 0 of 3,581 episodes** (`route_trust.rs:8-10`). The only producer is fermi-console, which Docker strips.
  - The `route_outcomes` join is a heuristic: `(agent_id, driver)` within −2/+10 min (break #11).
- **Carry over:**
  - Route provenance as a server-stamped fact.
  - The "CallerNamed vs router-selected are different populations" rule.
  - Declared domains before a hardcoded table; an explicit empty `domains: []` is a meaningful opt-out.
  - Stamp `episode_id` on claims to kill the heuristic join.
  - The cold-start progression (declarations, then accuracy-weighted, then calibrated classifier).

### Loop 5.A — Calibration measurement

- **Changes:** nothing directly. It is measurement-only, consumed by Loops 1, 4 and 5.B.
- **Brier path:**
  - `record_forecast_calibration_signals` (`handlers/forecasts.rs:1837`, called `:1614` and `polymarket.rs:1376`) writes the **team** Brier to every agent in `agents_used`. That's rank-1, so it cannot identify agents.
  - `compute_agent_calibration` (`src/calibration.rs:58`) returns `brier_skill_score = 1 − brier/(b(1−b))`, defined only if b(1−b) > 1e-9 (`:586-602`).
  - `evidence_class` (`:573`): `none` (n=0), `undiscriminating` (baseline ≤1e-9), `no_skill` (skill ≤0), `provisional` (n<5), `thin` (n<20), `usable`.
  - Per-agent `contribution` = gated Shapley mean + cluster-bootstrap CI (`:324-325,363`). It is labelled `score_scope: team` vs contribution.
- **Projection path:**
  - `ProjectionScoringEvaluator` (`evaluators/src/projection_scoring.rs:106`), score = `1 − min(|pred−act|/max(|act|,1e-9), 1)` (`:135-136`).
  - Matching: `projection_id` first. The 30-day heuristic fallback is **off unless requested by name** (`handlers/eval_projection.rs:33-51`).
  - The commitment anchor `commit_projection` (`src/projection_commit.rs:81`; called `handlers/observations.rs:617`, `simops_tools.rs:700`) runs **before** measurement. `resolve_against_projection` is at `handlers/simops_benchmark.rs:66`, called `observations.rs:645`.
- **Gates:**
  - `evidence_class`, plus a "read skill, not raw" rule.
  - Shapley validity gates applied in SQL.
  - The mechanism probe (`scripts/loop5_brier_mechanical_check.sql`, 9 checks).
  - Projection: anchor must pre-date measurement; no backfill of anchors (HANDOFF §9.3).
- **Status:**
  - Brier: fires-in-tests (`brier_skill_tests`, `calibration.rs:604+`), fires-in-code; observed "MECHANISM SOUND 9/9, skill +0.35 vs 2% base rate, n=48, 1 cluster".
  - **Projection: never had an input.** The projected properties and measured properties are disjoint (one overlapping row), and `ProjectionScoringEvaluator` is registered **only on the eval-run path** (`handlers/eval.rs:1047`). Link 5 (triggered from resolution) is still open (HANDOFF §9.5).
- **Carry over:**
  - The Brier skill formula and the base-rate guard.
  - `evidence_class` thresholds (5, 20).
  - Team score ≠ agent score.
  - The projection score formula. The anchor-before-measure rule. Fallback off by default.
  - One shared implementation for route and tool (`fermi::calibration`).
  - Cluster CI undefined below 3 clusters.

### Loop 5.B — BayesOps parameter correction

- **Changes:** distribution parameters of `learnable: true` FPL drivers. It writes `params.<driver>_fitted` (`handlers/workspace/refit.rs:994`) or stages a pending row.
- **Feed:** workspace resolutions (`feeds_from.source == "upstream_resolutions"`). Triggered post-commit from `workspace/resolution.rs:356`, from the multiplier hook (`agent_params_hook.rs:487`), or manually (`refit.rs:1331`). It refuses workspaces with no linked fermi_forecast (`refit.rs:125`).
- **Gate:** `classify_decision` (`refit.rs:887`), in order:
  1. `delta_pp > HARD_BLOCK_PP=20` ⇒ HardBlock.
  2. `DataQuality::Insufficient` (n_eff<5, `posterior/src/lib.rs:237`) ⇒ **Stage**, checked *before* impact.
  3. `delta_pp < threshold` (default `DEFAULT_AUTO_ACCEPT_PP=2.0`, per-driver override `feeds_from.auto_accept_threshold_pp`) ⇒ AutoAccept.
  4. Otherwise Stage.

  Impact is measured by two Monte Carlo runs of 10,000 iterations, current vs proposed params (`:163-171,913-937`). An **unassessable impact stages, it does not auto-accept** (`:429-449`). Interval widening counts as impact.
- **Status:**
  - fires-in-tests: `refit.rs:1360-1460` (`an_insufficient_fit_is_staged_however_small_its_impact`, `an_unassessable_impact_stages_rather_than_auto_accepting`, `widening_the_interval_counts_as_impact`) + `tests/bayesops_refit.rs` (DB, 4 ignored).
  - fires-in-code: yes.
  - observed: "refits on workspace resolution". Before the fix, all 55 snapshots were auto-accepted with n≈2 (`refit.rs:868`).
  - The doc comment at `refit.rs:908-912` still says a failed run "defaults to AutoAccept". The code stages. The comment is stale.
- **Carry over:**
  - Gate ordering (quality before impact).
  - The thresholds 2pp, 20pp, 10k iterations, n_eff 5/20.
  - "Unmeasurable ≠ small".
  - Fit families + bootstrap. The `Feed`/`Extractor` registry (`posterior/src/feeds.rs`) replacing the single `if`.
  - Snapshot ledger + pending review (mig 148).
  - Gaps: only LinearNormal; Phase 4 not built; conditional posteriors are cache-only.

### Shapley credit assignment (the substrate feeding 4.A and 5.A)

- **Changes:** per-agent credit φ and pairwise interaction index, per resolved forecast. Written to `forecast_attributions` (header), `forecast_agent_credit` and `forecast_agent_interactions` (mig 188).
- **Algorithm:**
  - `exact_shapley` (`src/attribution/mod.rs:124`): memoises v(S) once per subset (2ⁿ), weights s!(n−s−1)!/n!. Refuses n=0 or n > `MAX_EXACT_PLAYERS=16` (`:174`). No sampling, by design.
  - `pairwise_interactions` (`:191`) averages Δᵢⱼ(S) over the same memoised values.
  - Value function `v(S) = Brier(p_∅,y) − Brier(p_S,y)` (`:236,249`).
  - `counterfactual::attribute_forecast` (`counterfactual.rs:322`) re-runs the FPL with non-members neutralised: **Identity** (multiplier 1.0) or **Reference** (a supplied triple). φ values from the two modes are not comparable. Unclaimed declared drivers are pre-seeded at identity.
  - All subsets run under `stable_seed(forecast_id)`: FNV-1a 64, masked to 63 bits (`mod.rs:274`). Determinism **requires** BTreeMap driver iteration in the executor (`executor.rs:286-310`).
- **Validity gates** (`src/handlers/attribution.rs`):
  - `efficiency_residual = |Σφ − (v(N)−v(∅))|`. **Refuses to write above `MAX_EFFICIENCY_RESIDUAL=1e-6`** (`:61,149-152`). Exact is about 1e-12.
  - `reconstruction_error = |p_full − scored_probability|`, using mig-174's *frozen* `scored_probability`. **Warn and record above 0.01, never refuse** (`:65,159-170`), because a BayesOps refit can legitimately move params.
  - Consumers must filter on both: `calibration.rs:324-325`, `composition_evolution.rs:234-235,262-263`.
  - Claims are bound by explicit `forecast_id`, or else by an as-of join on the latest claim per `(workspace, driver)` at or before scoring time. Claims made after resolution are excluded.
  - The CI uses a cluster bootstrap by domain and returns `None` below 3 clusters (`mod.rs:308`).
- **Status:**
  - fires-in-tests: strongly. All four axioms. The dummy axiom is what caught the HashMap seed bug, which efficiency could not catch. DB pipeline tests at `handlers/attribution.rs:605-833`.
  - fires-in-code: `spawn_attribution` (`:857`) from `forecasts.rs:1621` and `polymarket.rs:1381`.
  - **observed: the input ledger `forecast_agent_claims` was INERT, with 0 writes and 0 opportunities** (HANDOFF §2, §9.3). 61 judgements were discarded because binding is workspace-only and the old regex missed markdown emphasis (§4.2). **There is no evidence in the docs that Shapley has ever run on production claims.**
- **Carry over:**
  - Both gates with their exact thresholds and the refuse/record asymmetry.
  - Exact enumeration with the n≤16 cap. Memoisation. Interactions from the same memo.
  - Neutralisation mode recorded with every result.
  - Frozen `scored_probability`. The append-only claims ledger (claims must be retained; no backfill).
  - The as-of binding rule. Identity pre-seeding.
  - `stable_seed` and the *sample order is part of the seed contract* rule.
  - Cluster bootstrap with the <3-cluster rule.
  - The "test all axioms, not the headline one" lesson.

---

## Kernel port candidates, ranked by value, with independent oracles

| # | kernel | source | proposed independent oracle |
|---|---|---|---|
| 1 | **Exact Shapley + pairwise interaction + efficiency residual** | `src/attribution/mod.rs:104-230` | Python brute force: average marginal contributions over all n! permutations (`itertools.permutations`) for n≤7 on random games. Closed-form games: glove game, weighted majority, additive game φ=wᵢ, unanimity game φ=1/|T|. Shapley interaction index from Grabisch's formula. Property tests for all four axioms, with dummy at <1e-12. |
| 2 | **Counterfactual value fn over FPL** (+ Brier, `stable_seed`) | `src/attribution/counterfactual.rs:322`, `mod.rs:236-283` | Make all driver claims degenerate (p5=p50=p95) so p_S is analytic. Recompute the WC template in numpy (v(∅) base rate 0.0208). FNV-1a test vectors from the reference spec. Brier vs `sklearn.metrics.brier_score_loss`. |
| 3 | **FPL Monte Carlo executor + samplers** | `src/executor.rs`, `src/distributions.rs` | scipy.stats KS/Anderson tests for each sampler. Analytic moments: product of lognormals, triangular mean (a+b+c)/3, Beta mean. Seed reproducibility across two instances; this must fail under HashMap. Decide the triangular semantics first (see owner decisions). |
| 4 | **FPL lexer/parser/semantic** | `src/lexer.rs`, `parser.rs`, `semantic.rs`, `ast.rs` | `tree-sitter-fpl/grammar.js` (independent grammar) for accept/reject parity on the corpus (`forecasts/**/*.fpl` 71, `templates/`, `examples/`). Round-trip AST → pretty-print → parse. |
| 5 | **TEC/ECHO settling + principle scoring + incoherence classify** | `coherence-engine/src/settling.rs`, `scoring.rs`; `coherence-core/src/{principles,system,incoherence,relevance}.rs` | An independent ECHO implementation (Thagard 1989 / *Coherence in Thought and Action* worked networks) run with the same θ. Golden activations to 1e-6. Taxonomy truth table over principle-score grid points. |
| 6 | **Coherence gate `check_against`** | `coherence-gate/src/gate.rs:199-250` | Port the peripheral vs load-bearing fixtures as golden cases. Verify Γ=0.632 invariance and correction_activation sign with the ECHO oracle from #5. |
| 7 | **BayesOps marginal fits + bootstrap + DataQuality** | `crates/posterior/src/{beta,normal,lognormal,triangular,auto,bootstrap,lib}.rs` | Conjugate closed forms (Beta(α+s, β+f); Normal known-variance update); `scipy.stats.*.fit`; `scipy.stats.bootstrap` percentile CI (seeded). Threshold table for n_eff 5/20. |
| 8 | **posterior-reg HMC LinearNormal + R-hat/ESS + what-if** | `crates/posterior-reg/src/{sampler,diagnostics,whatif,improvement}.rs` | Closed-form Normal-Inverse-Gamma Bayesian linear-regression posterior; Stan/PyMC on the same data; ArviZ `rhat`/`ess`; SALib Saltelli on the posterior mean function. |
| 9 | **Sobol sensitivity** | `src/sensitivity.rs` (and `posterior-reg/src/whatif.rs`) | SALib on the Ishigami function (analytic S1 ≈ 0.314/0.442/0, ST ≈ 0.558/0.442/0.244) expressed as FPL. Must be seeded first: the current code isn't (`:88` etc.). |
| 10 | **DBSCAN (cosine)** | `agent-bestiary/memory/src/clustering.rs` | `sklearn.cluster.DBSCAN(metric='cosine', eps=0.5, min_samples=k+1)`. Note the self-exclusion offset (`clustering.rs:49-51`). Compare label partitions up to relabeling; border-point order may differ. |
| 11 | **Dynamics ODE models** | `crates/dynamics/src/**` | `scipy.integrate.solve_ivp` (DOP853, tight tol) on the same RHS. Analytic `linear_decay`: v(t)=target+(v₀−target)e^{−kt}. Conservation/monotonicity invariants. |
| 12 | **Brier skill + evidence_class + projection score + Wilson + Silverman** | `src/calibration.rs:573-602`; `evaluators/src/projection_scoring.rs:135`; `fermi-console/src/calibration.rs:260`; `plot/density.rs:212` | Hand tables. `statsmodels.stats.proportion.proportion_confint(method='wilson')`. Silverman formula 0.9·min(σ, IQR/1.34)·n^(−1/5), computed independently (scipy's `bw_method='silverman'` is a different factor). |
| 13 | **Cluster bootstrap CI** | `src/attribution/mod.rs:308` | Independent numpy cluster resampling with the same seed scheme, or a distributional check (coverage over simulated clustered data). The <3-cluster rule returns None. |
| 14 | **Forecast-group propagation (mutex / at_most_n / implies)** | `src/handlers/relationships/propagation.rs:232,355,492` (math inside async sqlx) | Extract first. Then Hypothesis property tests: live sum = 1 − FLOOR·n_no − CEIL·n_yes, ranking preserved, YES collapses survivors to FLOOR. The existing `tests/relationships_mutex_math.rs` tests a mirror copy, not the prod fn, so it isn't an oracle. |
| 15 | **PCA projector** | `agent-bestiary/projector/src/projection.rs:220-260` | `sklearn.decomposition.PCA`. Compare explained variance and the subspace (sign-invariant). Currently 0 tests. |
| 16 | **Drift / trend / anomaly window** | `observability/src/drift.rs:63,197`, `trend.rs`, `anomaly.rs:80` | numpy cosine and rolling mean/std; adaptive σ-multiplier rule as a truth table. |
| 17 | **Decision tables** (`derive_proposals`, `classify_decision`, `detect_conflicts`) | `composition_evolution.rs:99`, `refit.rs:887`, `src/intentions.rs:162` | Golden decision tables written from the docs (thresholds listed in Task B), not from the code. |
| 18 | **SimOps cascade_v2 / slot_match** (tenant, but pure) | `crates/simops/src/cascade_v2.rs:427`, `slot_match.rs` | Mass-balance conservation invariants. kask JS original (`kask-sim-client.js:7896-7980`) for slot_match parity. |
| 19 | **Weather small numerics** (tenant) | `src/agent_backend/weather_tools.rs:1110,1723,2588,2668,2716` | `numpy.linalg.lstsq`, `np.corrcoef`, `np.quantile` (check the interpolation method), cvxpy log-optimal Kelly. |

---

## Needs owner decision

1. **Triangular semantics.** FPL samples `triangular(p5,p50,p95)` with p5/p95 as **min/max** (`src/distributions.rs:30-40`). Keep it, or solve for true percentiles? Every forecast's numbers depend on this. The oracle has to match whichever you pick.
2. **DBSCAN `min_samples` convention.** It excludes the point itself, which is off-by-one from sklearn/standard. Preserve it so learned clusters reproduce, or normalise it.
3. **One Sobol implementation.** There are two: `src/sensitivity.rs` (nested-MC first order + Saltelli total, unseeded) and `posterior-reg/src/whatif.rs` (Saltelli pick-freeze, seeded). Pick one and require a seed.
4. **`grounding_trust.rs` (8,979 lines, 88 tests).** Port `FIELD_CONTRACTS`/`PROV_*`/`floor()` as data + pattern, or discard with the rest of the ladder? It also backs `hud_contract`, `lens_rendering`, `provenance_oracle` and `card_contract`.
5. **Loop 2 consensus audit defect.** The second-reviewer path stores an `Undetermined` gate outcome (`observatory.rs:811-812`). For the new core: persist the first verdict, or re-check against a fresh world model at confirmation time?
6. **Loop 2 deadlock.** Drift needs `persona_version>1`, and only an AgentWide correction bumps it. Change the baseline rule or seed from hard-verified violations?
7. **Valence-homophily (3.B).** Compute the 0.25 spread, rely on the computed TensionBand `Above`, or drop it?
8. **Γ band vs production.** Production Γ mean 0.97 sits above `OPTIMAL_TENSION` (0.45–0.75) [I]. Is the band calibrated, or is the observer producing inflated coherence?
9. **Shapley player cap and the reconstruction gate.** Keep refusing above 16 players (no sampling)? Keep `reconstruction_error` as record-only?
10. **Roster resolution.** Keep the longest-prefix heuristic (`attribution/roster.rs:77`), or require `agent_id` on claims at write time? Same question for the `route_outcomes` time-window join: stamp `episode_id` instead.
11. **Team-score writes.** `record_forecast_calibration_signals` writes the team Brier to every agent (rank-1). Carry it over only as `score_scope: team`?
12. **simops / dynamics / projections.** The brief bins simops as a tenant, but it's pure and well-tested. Port it under the kernel oracle process anyway?
13. **Tooling.** `fermi-lsp` + `tree-sitter-fpl`: keep as tooling or drop? Keep tree-sitter at least as the parser oracle.
14. **Dead or duplicate code.** Candidates to drop: `coherence-api`, `coherence-protocols`, `agent-bestiary/consolidate`, `src/api/`, `src/bin/legacy/`, `src/report/`, `src/main.rs` REPL, `evaluator-sotopia`/`evaluator-character`.
15. **Consolidation cadence.** It's opt-in because it debits wallets. The new ACN credit model decides the default.
16. **Uncommitted work.** `abw-invariant-compiler-kit/` (untracked) and the 18 modified files: harvest them, or harvest strictly at `a3cea516`?

---

## Surprising or important

- **The most rigorous kernel has probably never seen production input.** The Shapley ledger `forecast_agent_claims` was INERT (0 writes, 0 opportunities) at the last audit (HANDOFF §2, §9.3). Loop 4.A has produced 0 proposals. Its correctness rests entirely on tests, so the oracle step matters more here than anywhere else.
- **Seed reproducibility depended on map type.** `HashMap` iteration order consumed the RNG differently per `Executor`, so `with_seed` didn't reproduce runs. The dummy axiom caught it; efficiency could not (`executor.rs:286-310`; CCA §8). The new core must treat "sample order is part of the seed contract" as a spec rule.
- **The executor has 3 direct tests** despite being the engine behind every forecast, Shapley and refit. Most coverage is indirect.
- **Γ is not a discriminator for corrections.** It is 0.632 whether a correction is absorbed or rejected. The gate's primary condition is the correction's own activation (`gate.rs:233`). The paper's §3.2 was corrected; any carried-over spec that says "Γ ≥ 0.5" is wrong.
- **Loop 2's audit trail is subtly wrong** for AgentWide corrections (consensus re-check with no world model; see owner decisions #5).
- **Routing provenance was wired against the desktop console.** Production goes through the HTTP door, so the `route:` tag was on 0/3,581 episodes (`route_trust.rs:8-16`).
- **The projection-accuracy path (5.A) has never had an input.** The projected and measured properties are disjoint (one overlapping row). An earlier "12,167 opportunities" figure was counting projections, not measurements (HANDOFF §Session 2).
- **The mutex propagation test checks a copy** of the math (`tests/relationships_mutex_math.rs:25-30`), not the production function, which lives inside async sqlx code.
- **posterior-reg is vanilla HMC, not NUTS** (`sampler.rs:5`), despite FEEDBACK_LOOPS §4. It also has a single model (LinearNormal), so the NLPD "improvement ladder" has one rung.
- **Many integration tests are source scans** (`include_str!`/`read_to_string` over `src/`) that assert code text, not behaviour. Examples: `coordination_loop_closure`, `*_coverage`, `*_parity`. Their *rules* are worth keeping as review checklists; the tests themselves don't port.
- **Recurring failure class worth designing out:** defaults that silently pass. Unassessable impact was auto-accepted; the 30-day projection fallback was on by default; `Undetermined` was easy to read as a pass; `unmeasured` was shown for a broken query. The docs' rule "an instrument that cannot fail loudly is not an instrument" should be a core invariant.
- **Scale and layout:** `fermi-console` is 75k lines (~55% of the 136k lines in non-root crates) and is stripped from Docker. `api_server.rs` is 7.5k lines and `handlers/` is 102k lines, nearly all discard. Worktree copies in `.wt/` and `.delta/` will double any naive repo-wide search. The 307 GB `target/` was not touched.
