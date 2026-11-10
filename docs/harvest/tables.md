# Harvest — database tables, views, functions, triggers

| | |
| --- | --- |
| Source repo | `/home/ilabra/fermi` |
| Source commit | `a3cea516` (HEAD; committed Thu Oct 8 11:40:15 2026 +0200) |
| Harvest date | 2026-11-10 |
| Scope | `migrations/*.sql` (247 numbered files + `rollback_auth.sql`), DDL embedded in Rust (`src/api_server.rs`), `agent-bestiary/memory/migrations/`, `scripts/*.rs` bins |
| Target model | `docs/architecture/ABW — Logical Architecture in Capability Terms.md` §Purpose (trust = append-only history; workspace = editable) and §"The nouns" |
| Mode | Read-only. Nothing in the legacy repo was modified. |

## 0. Method and caveats

**Method.**

1. Table list: every `CREATE [UNLOGGED] TABLE [IF NOT EXISTS] <name>` in `migrations/`, excluding comment lines. That gives **147 distinct table names**. The "~86" estimate in the brief was low. The count includes `creatures_rebuild`, a transient table that is renamed to `creatures` (`208_rebuild_creatures_reclaim_attnums.sql:138,156`), so there are **146 live tables**.
2. "Created in" is the **first** migration, in filename order, that contains the CREATE. Several tables are re-declared later: `fermi_forecasts` in 048/094/196, `fermi_forecast_updates` in 094/196, the social tables in 090/092, `orchestra_*` in 172/180. Note that the runner executes files in the order of the list in `src/api_server.rs:565` (`run_migrations`), not by filename. For example, 195 runs second (`195_declare_the_ghost_schema.sql:3-8`).
3. Writers and readers: a regex over `src/`, `crates/`, `agent-bestiary/`, `fermi-auth/` (`*.rs`, `*.sql`) for `INSERT INTO|UPDATE|DELETE FROM|TRUNCATE|FROM|JOIN <table>`, 5,900 sites in total. The **Writers** column lists up to 3 `INSERT/UPDATE/DELETE` sites (`H:` = `src/handlers/`, `AB:` = `agent-bestiary/memory/src/`).
4. **#U/D**: `UPDATE` + `DELETE` sites in non-test code, plus `INSERT … ON CONFLICT DO UPDATE` upserts (marked `+Nup`). Sites below a file's first `#[cfg(test)]`, or under `tests/`, are excluded. Their count is given as `(+Nt)` when non-zero.

**Caveats.**

- **Citations are against the working tree, not a clean `a3cea516`.** The tree had uncommitted edits before this harvest started, including `agent-bestiary/memory/src/store.rs`, `src/api_server.rs`, `src/handlers/agents.rs`, `src/handlers/economics.rs` and `.github/workflows/ci.yml` (`git status`). Line numbers in those files may be off by some lines at HEAD. Also, **`migrations/240_agent_versions_whole_card.sql` and `241_episodes_produced_by_version.sql` are untracked** (not in HEAD). They only add columns or comments, so no table count changes, but the "whole-card snapshot" and `produced_by_version` findings describe work in progress, not committed schema. All other migration files match HEAD.

- Regex, not a SQL parser. Dynamic SQL built with `format!`, and macros such as `delete_by_ws!` (`H:admin.rs:1480-1590`), are partly caught. Queries whose table name is on a different line from the verb are missed. Counts are **lower bounds**.
- `ON DELETE CASCADE` foreign keys are mutation paths that no grep finds. Every trust-ish table that FKs to `agents`/`episodes`/`teams`/`fermi_forecasts` with CASCADE can be erased by deleting the parent. See §2.
- Purposes come from the migration header comments plus the code. Where a purpose is my reading rather than a stated comment, it is marked **[I]**.
- Production row counts were **not** checked (no DB access in this harvest). "Dead" means "no code writer found", not "empty".
- `.delta/worktrees/` and `.wt/` (worktree copies) and `target/` were excluded.
- Tables read but **created by no migration**: `workspaces` (`H:beacons.rs:217`, `src/calibration.rs:415`) and `relationships` (only `migrations/rollback_auth.sql:48`). `docs/agent-bestiary/MEMORY_SCHEMA.sql:322` also declares `verification_tests`, which no migration creates. See §3.
- fermi-console (`crates/fermi-console`) declares `rusqlite` in `Cargo.toml:24`, but no `rusqlite::` usage or local DDL was found. It talks to the server over HTTP. No local tables.

---

## 1. Table inventory (147)

Disposition key: `trust:<noun>` | `crud:<area>` | `tenant:<name>` | `replaced:acn` | `derived` | `drop?`.

### 1a. Agency core: agents, versions, episodes, memory

| Table | Created in | Purpose | Writers (≤3) | #U/D | Disposition | Note |
| --- | --- | --- | --- | --- | --- | --- |
| `agents` | `010_add_adm_tables_and_dreaming.sql:15` | Agent registry: identity, owner, card fields, status, contracts | AB:store.rs:673, AB:store.rs:857, src/workflows/fork.rs:165 | 27 +1up (+3t) | `crud:agent` (identity) → card goes to `trust:agent_version` | 43 ADD COLUMNs across 24 migrations. It is the card *and* the registry *and* the counters. Five counters (`total_executions`…) are never written (`192_agent_execution_rollup.sql:5-13`). Capability columns (`fermi_contract`, `simops_contract`, `skills`, `input_contract`, `competition`) are card fields. |
| `agent_versions` | `024_agent_versions.sql:2` | Snapshot of the agent card before each edit; drives restore/diff | AB:store.rs:1199 | 0 | `trust:agent_version` | Insert-only in code, but not DB-enforced, and `ON DELETE CASCADE` from `agents` (`024:4`). Only 7 fields were snapshotted until `240_agent_versions_whole_card.sql` ("restore did not restore", :28). No content hash. The trigger `trg_agent_versions_bump_persona` (`103:199`) bumps `agents.persona_version`, which drifts from `version_number` (`241:1-24`). |
| `episodes` | `010_add_adm_tables_and_dreaming.sql:48` | One row per agent run: query, context, status, tokens, cost, embedding, assertions, provenance | AB:store.rs:208 (reserve), AB:store.rs:311 (upsert), src/workflows/fork.rs:322 | 5 +1up (+1t) | `trust:episode` | 25 ADD COLUMNs across 12 migrations. **Not append-only**: the store upsert overwrites ~25 columns (`AB:store.rs:345-372`), and `consolidated` is flipped (`AB:store.rs:1809`), as are `context` (`H:forecasts.rs:1559`) and `workspace_id` (`src/episode_boundary.rs:552`). `provenance` CHECK (`103:62`, extended in `200`). `assertions` JSONB is "written once… never updated" (`205:87-99`), but only by convention. |
| `episode_corrections` | `103_observability_foundations.sql:102` | HITL corrections appended instead of editing episodes | AB:store.rs:4319 | 0 | `trust:human_decision` | **DB-enforced** no-UPDATE trigger (`103:162-172`). DELETE is not blocked, and FK CASCADE from episodes/agents (`103:104-105`). |
| `semantic_rules` | `010_add_adm_tables_and_dreaming.sql:76` | Loop 1 distilled rules (sleep phase), embedded | AB:store.rs:1990, AB:store.rs:2100, src/workflows/fork.rs:241 | 8 | `crud:memory` | Seam #2 in the ABW doc: these change behaviour but are workspace state. Columns `provenance_floor`, `origin`, `extracted_by` (`201`,`203`,`223`) and `verification_status` ("four readers and no production writer", `235:10`). |
| `rule_retrievals` | `235_rule_retrievals.sql:59` | Which rule was injected into which run (Loop 1 efficacy) | src/agent_backend/kg_context.rs:359 | 0 | `trust:provenance` [I] | The rule→episode link is "a guess" when ambiguous (`235:59-84`). Could stay `crud:memory` if rules stay CRUD. |
| `entities` | `010_add_adm_tables_and_dreaming.sql:106` | Knowledge-graph entities per agent | AB:store.rs:2039, AB:store.rs:2405, src/workflows/fork.rs:214 | 2 | `crud:memory` | Embedding provenance columns (`135`). |
| `facts` | `010_add_adm_tables_and_dreaming.sql:131` | KG facts/edges | AB:store.rs:2540, src/workflows/fork.rs:290, AB:store.rs:2673 | 2 | `crud:memory` | |
| `communities` | `010_add_adm_tables_and_dreaming.sql:157` | KG community clusters | AB:store.rs:2969, AB:store.rs:1358 | 1 | `derived` | Recomputed by consolidation [I]. |
| `ontology_snapshots` | `010_add_adm_tables_and_dreaming.sql:175` | Per-agent ontology/dream snapshot (mermaid, synopsis, github tracking) | agent-bestiary/ontology/src/snapshot.rs:169, :101, :152 | 4 | `crud:memory` | `agent-bestiary/memory/migrations/003` adds github columns. |
| `consolidation_jobs` | `010_add_adm_tables_and_dreaming.sql:200` | Dreaming/consolidation job records | AB:store.rs:1840, AB:store.rs:1885, AB:store.rs:1917 | 3 | `crud:memory` | Job queue. Could be `derived`. |
| `consolidation_locks` | `010_add_adm_tables_and_dreaming.sql:226` | Advisory lock rows for consolidation | AB:locking.rs:31, AB:locking.rs:117, AB:locking.rs:101 | 3 +2up (+1t) | `derived` | Infra lock; disappears with a proper scheduler. |
| `agent_avatars` | `053_creature_image_storage.sql:22` | Agent avatar image cache (ephemeral FS workaround) | H:agents.rs:545, H:agents.rs:693 | 0 | `crud:workspace` | Blob-in-DB because of Railway's ephemeral FS (`053:22`). |
| `agent_credentials` | `171_agent_credentials.sql:24` | Encrypted BYOK provider credentials per agent | fermi-auth/src/secrets.rs:295 | 0 +1up | `crud:user/auth` | Secret store. |
| `agent_evolution` | `190_agent_evolution.sql:22` | High-water mark for the evolution badge (detects regression) | H:evolution.rs:869 | 0 +1up | `derived` | "Everything else on the badge is derived" (`190:1-10`). |
| `embedding_provenance` | `135_embedding_provenance.sql:70` | Append-only log of every embedding generated (model, version, dim, source) | AB:store.rs:33, H:agents.rs:1995 (+ scripts/backfill_embedding_provenance.rs) | 0 | `trust:provenance` | **REVOKE UPDATE, DELETE FROM PUBLIC** (`135:106`); weak, see §2. Spec 22. |
| `embedding_anchors` | `137_embedding_anchors.sql:20` | Fixed anchor texts co-embedded in vendor + open model, so vectors can be translated if a vendor dies | scripts/embedding_anchors.rs:298 (bin only) | 0 | `crud:memory` | No server code reads it. It is insurance data and is valuable if populated. See §4. |
| `orchestra_membership_requests` | `172_orchestra_membership.sql:47` | Request/approve/reject for an agent joining an orchestra (fermi, xaman_ek, simops) | H:orchestras.rs:480, :1082, :760 | 3 | `trust:human_decision` [I] | Governance decision. Status is mutated in place. |
| `orchestra_members` | `180_orchestra_members.sql:54` | Membership as governed state (SPEC_29) | src/api_server.rs:6022, H:orchestras.rs:790, :902 | 1 +1up | `crud:composition` | Current-state projection of the requests [I]. |
| `admin_bypass_events` | `164_admin_bypass_events.sql:37` | Audit of an admin forcing past a workflow gate (e.g. `publish?force=true`) | H:admin.rs:2664, H:admin.rs:2924, H:lifecycle.rs:48 | 0 | `trust:gate_decision` | An override is a gate event. Insert-only in code; not enforced. |

### 1b. Workspaces, composition, coordination

| Table | Created in | Purpose | Writers (≤3) | #U/D | Disposition | Note |
| --- | --- | --- | --- | --- | --- | --- |
| `teams` | `009_add_teams_and_sharing.sql:11` | **Is the workspace** (`"teams = workspaces"`, `034:69`): budget, git repo, mission, strategist, status, resolution, Brier | fermi-auth/src/teams.rs:91, H:forecast_git.rs:148, AB:consolidation.rs:820 | 23 (+1t) | `crud:workspace` | 23 ADD COLUMNs (`013…231`). Resolution columns (`147`: `resolved_at/by`, `resolution_outcome`, `brier_score`) are **trust:outcome** data in a CRUD row. Split them out. `workspace_budget/spent` → `replaced:acn`. |
| `team_members` | `009_add_teams_and_sharing.sql:36` | Human members + roles (+`capabilities`, `179`) | fermi-auth/src/teams.rs:242, :320, :384 | 5 +1up | `crud:user/auth` | |
| `object_shares` | `009_add_teams_and_sharing.sql:61` | Polymorphic sharing ACL (forecast, portfolio, workspace…) | fermi-auth/src/teams.rs:500, H:rabble_chat.rs:727, H:social.rs:914 | 3 +1up | `crud:user/auth` | Tenant isolation (T17) depends on this being right. |
| `workspace_agents` | `015_workspace_agents.sql:7` | Agents seated in a workspace (separate from people) | AB:store.rs:3397, AB:store.rs:3474, H:apps.rs:1368 | 4 | `crud:composition` | Current roster; history is in `composition_versions`. |
| `workspace_messages` | `014_workspace_messages.sql:6` | Workspace thread (people + agents) | AB:store.rs:3018, src/agent_backend/tools/domains/coordination.rs:662, H:bayesops.rs:750 | 1 | `crud:workspace` | Only "what was said". `episode_id` link (`222`) filled on 10 of 5,657 rows (`226:17-20`). |
| `workspace_outputs` | `143_workspace_outputs.sql:14` | Typed KV per workspace (e.g. `params`) | src/bin/resim_wc.rs:193, H:apps.rs:1335, H:apps.rs:617 | 2 +7up | `crud:workspace` | Current-state store that **overwrote per-agent claims** until `187` added a ledger (`187:22-31`). |
| `workspace_dependencies` | `143_workspace_outputs.sql:31` | DAG of workspace→workspace edges | H:apps.rs:1347, H:apps.rs:634, H:workspace/outputs.rs:293 | 2 +1up | `crud:workspace` | |
| `workspace_action_log` | `125_workspace_action_protocol.sql:31` | Every structured action against a workspace doc + confirm/apply lifecycle | H:workspace/actions.rs:108, H:workspace/lens_actions.rs:1028, :330 | 12 | `trust:gate_decision` (confirmation) + `crud:workspace` (apply) | Commented as "Append-only" (`125:87`), but 12 UPDATE sites mutate `confirmation/applied/apply_result` in place (e.g. `H:workspace/actions.rs:216`). The human accept/reject is a trust event; the payload stays as an artifact. |
| `workspace_annotations` | `125_workspace_action_protocol.sql:93` | Schema/doc annotations from actions | H:workspace/actions.rs:623, :625, :926 | 1 | `crud:workspace` | |
| `workspace_intentions` | `210_workspace_intentions.sql:61` | Loop 3 Stage 0: agents declare what they're about to do | src/agent_backend/tools/domains/coordination.rs:200, :189, :496 | 2 | `crud:composition` | Backs `intention_coordinator` tools that were "phantom" before (`210:12`). |
| `workspace_intention_signals` | `210_workspace_intentions.sql:113` | Align/conflict signals between intentions (feeds TEC graph) | src/agent_backend/tools/domains/coordination.rs:635 | 0 | `crud:composition` | Write-only in code (no FROM found). The coordination-observation seam (#3). |
| `composition_versions` | `113_composition_as_first_class.sql:53` | Team-recipe history: members, weights, strategist, mission, diff, proposed/accepted/rejected-by | AB:store.rs:3218, H:composition_evolution.rs:435, AB:store.rs:3367 | 2 (+1t) | `trust:human_decision` (proposal accept/reject) [I] | ABW seam #4: "approved but not recorded". Acceptance is an in-place UPDATE of `accepted_by`/`rejected_by` (`AB:store.rs:3367,3518`). Also hard-coded in `src/api_server.rs:1645`. FK CASCADE from `teams`. |
| `coherence_evaluations` | `016_coherence_evaluations.sql:4` | Γ(C)/TEC coherence results per workspace conversation | AB:store.rs:3111 | 0 | `crud:composition` (Γ signal) / `trust:gate_decision` when used as gate | Seam #3. Insert-only in code. |
| `select_agent_decisions` | `229_select_agent_decisions.sql:8` | Each `select_agent` call: candidates, choice, criteria weights | src/agent_backend/tools/domains/workspace.rs:641 | 0 | `trust:provenance` (route) | Typed routing trace for Loop 4 / MoE. Best-effort write (`229:8-12`). |
| `pending_cascades` | `153_pending_cascades.sql:26` | Operator-gated queue of forecast-relationship cascades after a resolve | H:pending_cascades.rs:140, :262, H:relationships/requeue.rs:108 | 4 | `trust:human_decision` [I] | Status mutated in place (pending→applied/superseded). Preserves "every parameter mutation must pass through a human" (`153:1-8`). |
| `xaman_sessions` | `115_xaman_sessions.sql:14` | Persistent navigator (xaman_ek) working sessions | H:xaman.rs:83, :252, :289 | 4 | `crud:workspace` | UI session state. |
| `apps` | `116_apps.sql:14` | App primitive: composition + schema + workspace template + UI pointer | src/api_server.rs:6301, H:apps.rs:297, H:xaman.rs:445 | 4 +1up | `crud:app` | App manifest noun. `touch_apps_updated_at` trigger (`116:77`). Linked via `teams.origin = apps.slug`, no FK. |
| `ground_runs` | `242_ground_runs.sql:25` | "Ground your agent": run token (hashed) for an externally hosted agent | H:ground.rs:272, H:ground.rs:472 | 1 | `trust:episode` | `run_id` = episode id (`242:19-20`). |
| `ground_run_tool_calls` | `242_ground_runs.sql:44` | Every tool call under a grounding run, verbatim output | H:ground.rs:391 | 0 | `trust:tool_call` | **The only persisted tool-call record in the schema.** Hosted-agent `tool_invocations` (`src/agent_backend/executor.rs:265`) are not stored in any table found [I]. See §4. |
| `a2a_push_configs` | `224_a2a_push_configs.sql:9` | A2A task webhooks + delivery status | src/a2a_webhook.rs:143, :83, H:a2a.rs:1420 | 2 | `crud:app` | Credentials are stored in plaintext (`224:22`). |

### 1c. Observability, evaluation, HITL, verification, gates

| Table | Created in | Purpose | Writers (≤3) | #U/D | Disposition | Note |
| --- | --- | --- | --- | --- | --- | --- |
| `eval_test_cases` | `027_eval_framework.sql:5` | Per-agent test cases with rubrics | AB:store.rs:3538, :3729, :3585 | 2 | `crud:agent` [I] | Contract/test fixtures. |
| `eval_runs` | `027_eval_framework.sql:21` | One row per "run all test cases" + aggregated signal | AB:store.rs:3613, AB:store.rs:3631 | 1 | `trust:evaluator_signal` | |
| `eval_signals` | `104_evaluator_signals.sql:32` | One evaluator measurement (LLM-judged or hard), versioned evaluator | AB:store.rs:4212, H:consolidation.rs:467, H:forecasts.rs:1895 | 0 | `trust:evaluator_signal` | Insert-only in code. Also carries **Loop 5.A per-agent "calibration"**, which is the forecast's Brier copied to every agent (`H:forecasts.rs:1837`, `src/outcome_trust.rs:24-36`). Do not carry those rows over as calibration. |
| `agent_timeline_entries` | `105_longitudinal_observability.sql:28` | Denormalised per-episode timeline for observatory charts | AB:store.rs:4433, AB:store.rs:4543 | 1 | `derived` | "denormalized projections" (`105:25-28`). |
| `dyad_state` | `105_longitudinal_observability.sql:95` | Running rapport/trust/reciprocity per human–agent dyad | AB:store.rs:4564 | 0 +1up | `derived` | Recomputed from episodes. |
| `dyad_profiles` | `133_dyad_profiles.sql:16` | Named human–agent relationship identity | H:observatory.rs:1345, :1742 | 0 +2up | `crud:memory` | |
| `anomaly_events` | `105_longitudinal_observability.sql:136` | Anomalies detected by AnomalyDetector (+ grounding anomalies, `200`/`225`) | AB:store.rs:4652, AB:store.rs:4877, src/agent_backend/tools/domains/observability.rs:324 | 2 | `trust:evaluator_signal` [I] | Commented "Append-only" (`105:133`), but `resolved_at/by` are updated in place (`AB:store.rs:4877`). The resolution belongs in `hitl_actions`. |
| `agent_observability_state` | `105_longitudinal_observability.sql:190` | Worker cursor (where the drift scan got to) | AB:store.rs:4721 | 0 +1up | `derived` | Infra cursor. |
| `hitl_actions` | `106_hitl_actions.sql:23` | Reviewer actions on anomaly events | AB:store.rs:4834 | 0 | `trust:human_decision` | **DB-enforced** no-UPDATE trigger (`106:63-73`). DELETE allowed; FK CASCADE from agents/anomaly_events (`106:27-28`). |
| `two_reviewer_requests` | `108_intervention_feedback_loop.sql:25` | Two-reviewer consensus for agent-wide interventions (D26) | AB:store.rs:4934, AB:store.rs:5013 | 1 | `trust:human_decision` | Status updated in place; `touch_two_reviewer_requests` trigger (`108:99`). |
| `assertion_verifications` | `205_assertion_layer.sql:111` | Append-only verdicts on agent assertions (`episodes.assertions`); actor, actor_kind, citation, evidence | src/verification_queue.rs:207, :71 | 0 | `trust:verdict` | The closest existing thing to a lattice verdict. CHECK: `human_sourced` requires `source_citation` (`205:159-160`); `actor_kind IN (tool,human,platform)` (`205:166`). Append-only by convention only. FK CASCADE from episodes (`205:121`). |
| `gate_decisions` | `214_gate_decisions.sql:49` | Recorded-tier gate decisions: approved/refused/**undetermined** | src/gate_trust.rs:790 | 0 | `trust:gate_decision` | Closed vocabularies registered in `src/seam_vocabulary.rs` (`214:34-47`). Only the `coherence` and `admission` gates write rows. `episode_id` added in `220`. |
| `gate_decision_reviews` | `216_gate_decision_reviews.sql:72` | Human/harness judgement whether a gate decision was right | src/gate_review.rs:69 | 0 | `trust:human_decision` | "NEVER updated and never deleted" (`216:31-33`), but only by convention; FK CASCADE from gate_decisions (`216:80`). |
| `impersonation_sessions` | `189_impersonation_audit.sql:34` | Admin "act as user" sessions (revocable) | H:impersonation.rs:174, :239 | 1 | `crud:user/auth` | |
| `impersonation_events` | `189_impersonation_audit.sql:77` | Each request made under impersonation | fermi-auth/src/middleware.rs:440 | 0 | `trust:provenance` [I] | Audit trail; also reasonable as `crud:user/auth` audit log. |
| `secret_access_log` | `039_user_secrets.sql:21` | Append-only audit of secret reads | fermi-auth/src/secrets.rs:256 | 0 | `crud:user/auth` | Security audit log. Append-only by comment only (`039:20`). |

### 1d. Forecasting (Fermi), calibration, attribution, BayesOps

| Table | Created in | Purpose | Writers (≤3) | #U/D | Disposition | Note |
| --- | --- | --- | --- | --- | --- | --- |
| `fermi_forecasts` | `048_fermi_notebooks.sql:43` | The forecast: question, probability, FPL, drivers, status, resolution, Brier | H:forecasts.rs:475, H:polymarket.rs:854, src/api_server.rs:1970 | 25 | `crud:forecast` (working state) + `trust:outcome` / `trust:claim` (commitment, resolution) | 44 ADD COLUMNs across 9 migrations; schema drift is so bad that `196` exists (`195:27-32`). The freeze trigger `trg_fermi_forecasts_freeze_resolved` (`174:115-163`) *silently keeps* the old `scored_probability/brier_score/actual_outcome/predicted_probability` once resolved, with a WARNING rather than an error. ABW tenant 1. |
| `fermi_forecast_updates` | `094_fermi_forecasting.sql:143` | Probability revision history + trigger kind + actor | src/bin/apply_wc_cascades.rs:203, H:bayesops.rs:702, H:forecast_git.rs:807 | 1 | `trust:claim` (commitment revisions) | 9 insert sites, 0 UPDATE. DELETE only in admin purge (`H:admin.rs:1554`). Not enforced. Also re-declared in `src/api_server.rs:1701`. |
| `forecast_commitments` | `140_forecast_benchmark.sql:77` | Tamper-evident hash anchor of each emitted probability (`commitment_hash` UNIQUE) | H:forecast_benchmark.rs:112 | 1 | `trust:claim` (commitment) | `sha256(forecast_id‖p‖fpl_hash‖emitted_ts)` (`140:66-80`). "Properly benchmarkable" only if committed_at < resolved_at. Admin purge deletes it (`H:admin.rs:1545`). |
| `harness_snapshots` | `140_forecast_benchmark.sql:33` | Content-addressed harness version (conductor card hash, routing weights, roster, BayesOps params) with parent_hash lineage | H:forecast_benchmark.rs:662 | 0 | `trust:agent_version` (team/harness version) | A hash-chained config lineage that already exists. See §4. |
| `forecast_splits` | `140_forecast_benchmark.sql:122` | Deterministic held_in/held_out/validation split, salted, "immutable at ingestion" | H:forecast_benchmark.rs:180 | 1 | `trust:policy` [I] | Pre-registered split = evaluation policy. Not enforced. |
| `forecast_spacetime` | `140_forecast_benchmark.sql:147` | Per-revision trajectory (p, Brier-at-point, harness id) | src/api_server.rs:1891, H:forecasts.rs:1728, :1795 | 4 | `derived` | Filled by trigger `trg_forecast_spacetime` on `fermi_forecast_updates` (`140:233`) + handler; Brier back-filled by UPDATE at resolve. `drivers_snapshot/sobol_snapshot` "no writer exists" (`187:35-40`). |
| `forecast_agent_claims` | `187_forecast_agent_claims.sql:67` | Append-only ledger of each agent's driver claim (p5/p50/p95, neutral_value) | H:workspace/agent_params_hook.rs:379 | 0 (+1t) | `trust:claim` | **Foundation of Shapley credit.** "the backlog cannot be reconstructed" (`187:29-31`). `episode_id` (`197`) and `assertion_id` (`205`) link it to episodes. FK SET NULL (not cascade) on agents/forecasts (`187:77,94`). |
| `forecast_attributions` | `188_forecast_attributions.sql:64` | Shapley header: seed, iterations, p_baseline, p_full, **efficiency_residual**, **reconstruction_error**, neutralisation mode | H:attribution.rs:315 | 0 +1up | `trust:calibration` | Upsert on (forecast_id, neutralisation): "no history worth keeping" (`188:58-61`), because it is a pure function of inputs. Could be `derived` if inputs are trust (but then the validity-gate values must still be kept). |
| `forecast_agent_credit` | `188_forecast_attributions.sql:92` | Per-agent Shapley φ (signed) | H:attribution.rs:361, H:attribution.rs:352 | 1 | `trust:calibration` | Delete-then-insert per recompute (`H:attribution.rs:352`). ABW seam #1: "must be carried over explicitly". |
| `forecast_agent_interactions` | `188_forecast_attributions.sql:112` | Pairwise Shapley interaction index (synergy/redundancy) | H:attribution.rs:395, H:attribution.rs:375 | 1 +1up | `trust:calibration` | The Loop 4 composition-evolution signal (`188:116-118`). |
| `fermi_market_observations` | `099_polymarket_observations.sql:22` | Append-only Polymarket price snapshots | H:polymarket.rs:1148, :263, :459 | 1 | `trust:provenance` (external source) | "rows are never updated or deleted" (`099:13`), but admin purge deletes them (`H:admin.rs:1558`). |
| `fermi_forecast_schedules` | `109_forecast_agent_schedules.sql:5` | Recurring agent research schedules per forecast | H:forecasts.rs:3440, :3534, H:admin.rs:1566 | 3 +1up | `crud:forecast` | |
| `fermi_notebooks` | `048_fermi_notebooks.sql:4` | Authoring container (optional) | H:notebooks.rs:144, :279, :501 | 3 | `crud:forecast` | |
| `fermi_portfolios` | `048_fermi_notebooks.sql:25` | Named forecast collections | H:forecasts.rs:2305, :2808, :2710 | 2 | `crud:forecast` | |
| `fermi_portfolio_forecasts` | `048_fermi_notebooks.sql:64` | Portfolio membership | H:forecasts.rs:2601, :587, H:polymarket.rs:880 | 2 | `crud:forecast` | |
| `forecast_invites` | `151_forecast_invites.sql:41` | Collaboration invites (pending→accepted/declined/revoked/expired) | H:invites.rs:282, fermi-auth/src/invites.rs:61, H:invites.rs:546 | 5 | `crud:user/auth` | |
| `forecast_relationships` | `150_forecast_relationships.sql:25` | Pairwise forecast relationships (legacy) | H:relationships/legacy.rs:59, :148 | 1 | `crud:forecast` | Superseded by groups? (`155`). |
| `forecast_relationship_groups` | `155_forecast_relationship_groups.sql:31` | Constraint groups (e.g. mutually exclusive) | H:relationships/groups.rs:75, :273, :398 | 2 | `crud:forecast` | Members via `fermi_forecasts.relationship_groups`. |
| `driver_annotations` | `183_driver_annotations.sql:58` | Teammate comment on a driver ("your base rate is wrong"); accepted/declined outcome | H:annotations.rs:249, :322, :464 | 4 | `crud:forecast` | "An annotation is a claim someone made… record what happened, not erase it" (`183:58`), yet it has 1 DELETE (`H:annotations.rs:367`). Possibly `trust:human_decision` [I]. |
| `bayesops_posterior_snapshots` | `148_bayesops_refit_ledger.sql:49` | Every BayesOps posterior fit ever computed (append-only ledger) | H:workspace/refit.rs:1180, H:admin.rs:1514 | 1 | `trust:provenance` (fitted parameters) [I] | ABW seam #2 ("Loop 5.B rewrites forecast parameters"). Commented "Append-only" (`148:142`); admin purge deletes it. |
| `bayesops_pending_fits` | `148_bayesops_refit_ledger.sql:108` | Gate queue: fit staged for human accept/reject/expire | H:workspace/refit.rs:1228, H:bayesops.rs:649, :847 | 4 | `trust:human_decision` | Status mutated in place (`H:workspace/refit.rs:1216`). θ/parameter change acceptance = invariant 10. |

### 1e. Money (→ ACN)

| Table | Created in | Purpose | Writers (≤3) | #U/D | Disposition | Note |
| --- | --- | --- | --- | --- | --- | --- |
| `wallets` | `012_credit_ledger.sql:7` | Balance per user/workspace/agent (granted/purchased split, `066`) | fermi-auth/src/credits.rs:83, :141, :191 | 5 | `replaced:acn` | Balance is a mutable cache alongside the ledger. |
| `credit_ledger` | `012_credit_ledger.sql:21` | "Append-only" credit ledger with `balance_after` | fermi-auth/src/credits.rs:160, :209, :283 | 1 | `replaced:acn` | Single-entry, not double-entry (T9). 1 UPDATE (`H:billing.rs:334`, stamps `stripe_session_id`). `tx_type` CHECK "declared by seventeen migrations and applied by none" (`205:83-84`). |
| `stripe_sessions_processed` | `182_stripe_session_idempotency.sql:37` | Structural Stripe idempotency claim | H:billing.rs:266, :352, :304 | 2 | `replaced:acn` | Documents the double-credit bugs (`182:1-20`). Keep the lesson. |
| `agent_episode_payouts` | `057_rabble_workspaces.sql:5` | Agent royalty/collab payout per episode | src/gas.rs:480 | 0 | `replaced:acn` | |
| `marketplace_transactions` | `030_shopping_marketplace.sql:52` | Advertiser match queries against consumer profiles (paid) | AB:store.rs:4152 | 0 | `replaced:acn` | See shopping below. |
| `swarm_activations` | `061_swarm_algorithms.sql:20` | Purchases/activations of formation algorithms | src/agent_backend/tools/domains/rabble.rs:379, H:swarm_algorithms.rs:269 | 0 | `tenant:rabble` | Has a money side (`formation_activate` tx_type) → ACN. |

### 1f. Auth, users, platform infra

| Table | Created in | Purpose | Writers (≤3) | #U/D | Disposition | Note |
| --- | --- | --- | --- | --- | --- | --- |
| `users` | `004_add_users_table.sql:6` | Zitadel/OIDC/SIWE identity cache + profile | fermi-auth/src/oidc.rs:480, H:auth.rs:764, H:forecasts.rs:77 | 7 (+1t) | `crud:user/auth` | Ghost columns (`id`, `password_hash`, `name`) created by no migration until `195`. `stripe_customer_id` → ACN. `update_users_updated_at` trigger (`004:54`). |
| `api_keys` | `005_add_api_keys.sql:5` | Programmatic access keys | fermi-auth/src/api_keys.rs:43, :115, :183 | 2 | `crud:user/auth` | |
| `siwe_nonces` | `008_add_siwe_nonces.sql:5` | SIWE replay protection | fermi-auth/src/siwe.rs:92, :151, :176 | 2 | `crud:user/auth` | `cleanup_expired_siwe_nonces()` (`008:19`). |
| `user_secrets` | `039_user_secrets.sql:4` | Encrypted user integration credentials | fermi-auth/src/secrets.rs:94, :193 | 1 +1up | `crud:user/auth` | |
| `notifications` | `021_notifications.sql:2` | In-app notifications | src/api_server.rs:7226, H:push.rs:368, H:profile.rs:331 | 2 | `crud:app` | |
| `push_subscriptions` | `098_push_subscriptions.sql:9` | Web Push subscriptions per device | H:push.rs:98, :137, :292 | 5 +1up | `crud:app` | |
| `push_config` | `098_push_subscriptions.sql:37` | VAPID key singleton | — (no writer found; read `H:push.rs:51`) | 0 | `crud:app` | Populated out-of-band [I]. |
| `waitlist` | `023_waitlist.sql:2` | Early-access waitlist | H:admin.rs:928, H:misc.rs:36, H:admin.rs:890 | 2 +1up | `crud:app` | |
| `schema_migrations` | `207_migration_ledger.sql:65` | Per-file migration attempt counters (+ hash) | src/api_server.rs:1601 | 0 +1up | `derived` | Infra. "NOT append-only" (`207:92`). Also DDL at `src/api_server.rs:1568`. |
| `migrations_log` | `195_declare_the_ghost_schema.sql:129` | Exists only because 089/090 insert into it | — (SQL-only, 089/090) | 0 | `drop?` | "records attempts, not state" (`195:120-128`). |
| `creatures_rebuild` | `208_rebuild_creatures_reclaim_attnums.sql:138` | Transient copy, renamed to `creatures` | — | 0 | `drop?` | Not a live table after 208 (`208:155-156`). Listed only for completeness. |

### 1g. Tenant: Rabble (creatures, swarms, social, governance)

| Table | Created in | Purpose | Writers (≤3) | #U/D | Disposition | Note |
| --- | --- | --- | --- | --- | --- | --- |
| `creatures` | `042_rabble_creatures.sql:4` | Minted AR insect specimens (core Rabble table) | src/agent_backend/tools/domains/rabble.rs:238, H:creatures/identity.rs:132, src/agent_backend/tools/domains/biology.rs:1054 | 18 | `tenant:rabble` | Hit the 1600-attnum ceiling from boot replays (`208:11-19`). 26 inbound FKs. The ABW doc lists "creatures" as workspace state. |
| `creature_state` | `078_creature_versioned_state.sql:19` | Current-state pointer per creature | H:creatures/helpers.rs:235, H:creatures/state.rs:1153, :587 | 15 +4up | `tenant:rabble` | |
| `creature_versions` | `078_creature_versioned_state.sql:54` | Immutable state-transition history | H:creatures/helpers.rs:205 | 0 | `tenant:rabble` | Append-only by comment (`078:51`, `085:19`); insert-only in code. A good event-sourcing pattern already exists here. |
| `creature_conditions` | `078_creature_versioned_state.sql:40` | Owner-defined social attributes, genome, cognition tier | H:creatures/helpers.rs:270, H:creatures/agent_modules.rs:788, H:creatures/flights.rs:1302 | 11 | `tenant:rabble` | |
| `creature_flights` | `042_rabble_creatures.sql:42` | Flight log per location | H:creatures/flights.rs:1228, :138, :1398 | 28 | `tenant:rabble` | Highest-churn table. |
| `creature_collections` | `042_rabble_creatures.sql:109` | Named creature groupings | H:creatures/collections.rs:81, :155 | 1 | `tenant:rabble` | |
| `creature_images` | `053_creature_image_storage.sql:4` | Generated creature art blobs | H:creatures/identity.rs:812, H:creatures/query.rs:840 | 0 +2up | `tenant:rabble` | |
| `creature_animation_layers` | `064_creature_animation_layers.sql:5` | Segmented wing/body layers for client animation | src/agent_backend/tools/domains/biology.rs:1102, H:creatures/identity.rs:966 | 0 +2up | `tenant:rabble` | |
| `creature_devices` | `056_devices.sql:2` | GPS/BLE device pairing | H:creatures/devices.rs:94, :153, :220 | 3 +1up | `tenant:rabble` | |
| `creature_tethers` | `074_creature_tethers.sql:5` | Link creature to live signal source | H:creatures/tethering.rs:245, H:creatures/flights.rs:1289, H:creatures/tethering.rs:171 | 3 | `tenant:rabble` | |
| `telemetry_points` | `074_creature_tethers.sql:21` | Position stream from tethered creatures | H:creatures/tethering.rs:515 | 0 | `tenant:rabble` | Overlaps `sosa_observations` [I]. |
| `creature_favourites` | `087_creature_favourites.sql:5` | Star a creature | H:creatures/state.rs:1393, :1413 | 1 | `tenant:rabble` | |
| `creature_friendships` | `090_social_layer.sql:16` | Symmetric creature friendships | H:social.rs:348, H:governance.rs:105, :213 | 7 | `tenant:rabble` | Re-declared in `092`. |
| `creature_invites` | `090_social_layer.sql:53` | Creature→creature invite to a rabble | H:social.rs:857, :1038, :1073 | 5 | `tenant:rabble` | |
| `creature_goals` | `131_creature_goals.sql:14` | Standing objectives evaluated by Wild agents | H:creatures/goals.rs:72, H:creatures/agent_modules.rs:2237, H:creatures/goals.rs:182 | 4 | `tenant:rabble` | Bridge to kask_wild. |
| `creature_blocks` | `097_governance.sql:17` | Creature-level block | H:governance.rs:78, :145 | 1 | `tenant:rabble` | |
| `forage_observations` | `132_forage_observations.sql:11` | Structured foraging observations (feed dream consolidation) | H:creatures/agent_modules.rs:2196, H:workspace/actions.rs:1259, :1261 | 0 | `tenant:rabble` (kask_wild) | Write-only in Rust (no FROM found). |
| `swarm_events` | `042_rabble_creatures.sql:72` | Rabbles: gatherings with funding, invite pool, QR, walk-in budget | H:creatures/state.rs:491, H:creatures/swarms.rs:687, H:admin.rs:1155 | 24 | `tenant:rabble` | Funding columns (`045`,`046`,`072`) → ACN. |
| `swarm_participants` | `091_swarm_participants.sql:5` | Join/leave per user per rabble | (seed INSERT `091:49`), H:creatures/helpers.rs:95 | 1 | `tenant:rabble` | No Rust INSERT found. |
| `swarm_sub_flocks` | `063_sub_flocks.sql:6` | Creature groups within a rabble | H:rabble_workspace.rs:1268, H:swarm_algorithms.rs:191, :289 | 2 | `tenant:rabble` | |
| `swarm_algorithms` | `061_swarm_algorithms.sql:6` | Catalogue of Onto4MAT formation specs (seeded) | — (seeded in `061`) | 0 | `tenant:rabble` | |
| `swarm_sessions` | `051_swarm_telemetry.sql:5` | Telemetry collection window | H:swarm_telemetry.rs:105, :232, :396 | 2 | `tenant:rabble` | Generalised by `sosa_*`. |
| `swarm_telemetry` | `051_swarm_telemetry.sql:21` | High-frequency swarm samples | H:swarm_telemetry.rs:315 | 0 | `tenant:rabble` | |
| `rabble_messages` | `044_rabble_messages.sql:2` | Creature-attributed chat | H:creatures/flights.rs:730, H:creatures/state.rs:1243, H:creatures/swarms.rs:1084 | 0 | `tenant:rabble` | |
| `rabble_co_presence` | `090_social_layer.sql:171` | Which creatures met in a rabble | H:social.rs:1216, :1515, :1542 | 1 | `tenant:rabble` | No FROM reads in Rust; read by SQL functions (`get_creatures_met_in_rabble`). |
| `rabble_follows` | `094_rabble_follows.sql:12` | User follows a rabble | H:social.rs:1587, :1689, :1638 | 2 +1up | `tenant:rabble` | |
| `rabble_ejections` | `097_governance.sql:65` | Ejections with cooldown | H:governance.rs:424, :545 | 1 | `tenant:rabble` | |
| `activity_events` | `090_social_layer.sql:104` | Social feed events (SSE source) | H:social.rs:1485 | 0 | `tenant:rabble` | |
| `contacts` | `055_contacts.sql:2` | Asymmetric user follow | H:social.rs:114, :185, :192 | 4 | `tenant:rabble` | Arguably `crud:user/auth` if social goes platform-wide. |
| `user_blocks` | `097_governance.sql:44` | User-level block | H:governance.rs:201, :241 | 1 | `tenant:rabble` | Platform-safety concern [I]. |
| `reports` | `097_governance.sql:101` | Abuse reports with content snapshot | H:governance.rs:709 | 0 | `tenant:rabble` | |
| `saved_locations` | `095_saved_locations.sql:12` | Map pins | H:dashboard/mod.rs:258, :387, :334 | 2 | `tenant:rabble` | |
| `ar_beacons` | `041_ar_beacons.sql:4` | AR asset placement (H3 cells) | src/agent_backend/tools/domains/spatial.rs:682 | 0 | `tenant:other` (AR spatial suite / architect lens) [I] | Its reader joins `workspaces`, a table that does not exist (`H:beacons.rs:217`), so that path is probably broken [I]. |
| `ar_grid_maps` | `041_ar_beacons.sql:66` | Named spatial grids | src/agent_backend/tools/domains/spatial.rs:911 | 0 | `tenant:other` | |
| `ar_choreographies` | `041_ar_beacons.sql:47` | Motion sequences on beacons | — | 0 | `drop?` | No code reference. |
| `flight_telemetry` | `078_creature_versioned_state.sql:105` | Observations during FLY state | — | 0 | `drop?` | No code reference. Superseded by `telemetry_points`/`sosa_observations` [I]. |

### 1h. Tenant: SimOps / sensors (SOSA), DPP, shopping, AKP, misc

| Table | Created in | Purpose | Writers (≤3) | #U/D | Disposition | Note |
| --- | --- | --- | --- | --- | --- | --- |
| `sosa_platforms` | `052_sosa_observations.sql:6` | W3C SOSA platforms (drone, greenhouse…) | H:creatures/flights.rs:522, H:observations.rs:141 | 0 | `tenant:simops` | Shared with Rabble. Could be a platform substrate ("sensor-data provenance", Refactor plan tenant 5). |
| `sosa_sensors` | `052_sosa_observations.sql:18` | Sensors on platforms | H:observations.rs:160 | 0 | `tenant:simops` | No Rust reads. |
| `observation_sessions` | `052_sosa_observations.sql:30` | Collection windows | H:creatures/flights.rs:539, H:observations.rs:275, H:creatures/flights.rs:637 | 2 | `tenant:simops` | |
| `sosa_observations` | `052_sosa_observations.sql:44` | Individual observations (append-only, high freq); `produced_by_agent/version` (`128`) | src/agent_backend/simops_tools.rs:657, H:creatures/flights.rs:562, :573 | 0 | `tenant:simops` → `trust:outcome` for ground truth [I] | Real measurements are what SimOps projections are scored against. Insert-only in code. |
| `process_projection_commits` | `141_process_benchmark.sql:34` | Commitment anchor for a simulated prediction before measurement | src/projection_commit.rs:126 | 0 | `trust:claim` (commitment) | SimOps equivalent of `forecast_commitments`. |
| `process_spacetime` | `141_process_benchmark.sql:72` | Projection × resolution events trajectory | H:simops_benchmark.rs:192 | 0 | `trust:outcome` | `committed_before_measured` flag (`215`). |
| `process_sample_config` | `141_process_benchmark.sql:145` | Per-workspace sample-point schedule | H:simops_benchmark.rs:540 | 0 +1up | `tenant:simops` | |
| `carbon_emission_factors` | `238_carbon_emission_factors.sql:74` | Append-only ledger of every emission factor `carbon_accountant` retrieved, with source | H:workspace/carbon.rs:918 | 0 | `trust:provenance` | "the ledger that makes an emission factor falsifiable" (`238:1`). Needed by DPP (tenant 3). Append-only by convention. |
| `dpp_publications` | `243_dpp_publications.sql:53` | Public passport: stable token, current snapshot, release count, revoked_at | H:workspace/dpp_publish.rs:454, :471, :538 | 2 | `trust:publication` | Current snapshot is replaced in place; history is in releases. |
| `dpp_publication_releases` | `243_dpp_publications.sql:70` | Every released snapshot | H:workspace/dpp_publish.rs:489 | 0 | `trust:publication` | No reader found yet. Not hashed/signed (Refactor plan wants G20 signed+hashed). |
| `shopping_profiles` | `030_shopping_marketplace.sql:5` | Consumer preference profile (embedded) | AB:store.rs:3856, :3974, :4017 | 2 +1up | `drop?` | Embedding marketplace. Routes still live (`src/api_server.rs:5117-5133`). No noun in ABW. |
| `marketplace_listings` | `030_shopping_marketplace.sql:28` | Consumer lists profile for advertisers | AB:store.rs:3980, :4006, :4026 | 3 | `drop?` | Same as above. |
| `agent_alignments` | `049_akp_foundation.sql:5` | AKP ontology-similarity between agent pairs | — | 0 | `drop?` | No code reference. |
| `pairwise_coherence` | `049_akp_foundation.sql:19` | AKP pairwise coherence history | — | 0 | `drop?` | No code reference. Superseded by `coherence_evaluations` [I]. |
| `knowledge_transfers` | `049_akp_foundation.sql:31` | AKP append-only knowledge-transfer log | — | 0 | `drop?` | No code reference. |
| `agent_interaction_policies` | `049_akp_foundation.sql:45` | AKP socialisation rules | — | 0 | `drop?` | No code reference. |
| `voice_assets` | `048b_voice_assets.sql:4` | TTS audio asset tracking | — | 0 | `drop?` | No code reference. `audio_url` columns on other tables are used instead (`048b`). |

### 1i. Views, materialized views, functions, triggers

**Views** (`CREATE [MATERIALIZED] VIEW`). There are 14, and some are redefined later.

| View | Defined in (last) | Purpose | Rust readers | Disposition |
| --- | --- | --- | --- | --- |
| `agent_execution_rollup` | `200_coordinator_observation_provenance.sql:75` (first `192:69`) | One definition of agent economics from episodes; excludes `coordinator_observation` + `synthetic_correction` | 13 files (`src/rollup_trust.rs`, `H:profile.rs`…) | `derived` (projection over `trust:episode`) |
| `route_outcomes` | `197_claim_episode_correlation.sql:84` (first `193:62`) | Routing provenance (`route:*`, `qsrc:*` tags) joined to realised outcomes | `src/route_trust.rs`, `src/episode_boundary.rs` | `derived` |
| `route_reason_performance` | `193:145` | Outcome by route reason | none | `derived` / `drop?` |
| `domain_agent_ranking` | `193:174` | Ranking for MoE routing | `src/feeds/mod.rs`, `src/route_trust.rs` | `derived` |
| `router_override_scorecard` | `193:211` | Human overrides of router vs outcome | none | `derived` (important if kept: human-vs-machine scoring) |
| `declaration_quality_outcomes` | `193:241` | Contract declaration quality vs outcome | `src/route_trust.rs` | `derived` |
| `forecast_cost_attribution` | `198_episode_delegation_tree.sql:83` | Per-forecast cost incl. delegated child episodes | `src/api_server.rs` only | `derived` |
| `fermi_leaderboard` (MATERIALIZED) | `167_fermi_leaderboard_float8_minmax.sql:77` (first `094:178`) | Brier leaderboard | 8 files | `derived` |
| `fermi_brier_audit` | `174:221` | Audit of resolved forecasts' Brier integrity | none | `derived` |
| `fermi_forecast_workspaces` | `139_fermi_workspace_link.sql:17` | Forecast ↔ workspace convenience join | none | `drop?` |
| `orchestra_fermi_members` | `185_hide_test_cruft_from_rosters.sql:37` | Roster predicate | 5 files | `derived` |
| `orchestra_xaman_ek_members` | `185:67` | Roster | 3 files | `derived` |
| `orchestra_simops_members` | `233_simops_contract.sql:40` | Roster | 2 files | `derived` |
| `rbac_orphans` | `208_rebuild_creatures_reclaim_attnums.sql:179` (first `163:29`) | Objects whose owner no longer resolves | `H:admin_rbac.rs`, `fermi-auth/src/rbac.rs` | `derived` |

**Functions with logic worth carrying.** Trivial `touch_*` updated_at functions are omitted.

| Function | Defined in | What it does | Disposition |
| --- | --- | --- | --- |
| `compute_brier_score` | `094:223` | Brier score (IMMUTABLE) | `trust:outcome`. Becomes a pure kernel. |
| `resolve_forecast` | `174:174` (first `094:239`) | Resolves a forecast, snapshots `scored_probability`, stamps Brier. Signature pinned by `src/schema_trust.rs:241` | `trust:outcome` |
| `fn_fermi_forecasts_freeze_resolved` + trigger | `174:115-163` | Freezes the scoring tuple after resolve, by **silent revert + WARNING** rather than an error | `trust:outcome`. In the new core, replace with append-only; see §2. |
| `fn_forecast_spacetime_on_update` + `trg_forecast_spacetime` | `149:49` (first `140:205`), trigger `140:233` | Writes a spacetime row on each forecast update | `derived` |
| `episode_corrections_immutable` + trigger | `103:162-172` | Block UPDATE | append-only enforcement (§2) |
| `hitl_actions_immutable` + trigger | `106:63-73` | Block UPDATE | append-only enforcement (§2) |
| `bump_agent_persona_version` + `trg_agent_versions_bump_persona` | `103:188-201` | Bump `agents.persona_version` on version insert. Root of the off-by-N counter (`241:1-24`) | `drop?`. Replace with the version id itself. |
| `auto_add_team_owner` + trigger | `009:90-100` | Owner auto-joins team | `crud:user/auth` |
| `refresh_fermi_leaderboard` | `094:291` | Refresh MV | `derived` |
| Social/spatial functions: `get_activity_feed`, `get_creature_friends`, `get_creatures_met_in_rabble`, `get_pending_*`, `canonical_creature_pair`, `expire_old_creature_invites` (`090`/`092`), `get_nearby_rabbles`, `get_my_rabbles_with_status`, `get_creatures_with_deployment`, `check_boundary_violations` (`089`, PostGIS), `get_nearby_creatures`, `get_saved_locations` (`095`), `get_followed_rabbles`, `get_rabble_followers` (`094_rabble_follows`), `is_blocked`, `is_user_blocked`, `is_ejected`, `get_user_blocks` (`097`) | — | Rabble read models | `tenant:rabble` |
| `cleanup_expired_siwe_nonces` | `008:19` | Nonce GC | `crud:user/auth` |

**Extensions:** `uuid-ossp`, `vector` (pgvector, `010:8-9`). PostGIS is required by `089:302`.
**RLS:** none enabled in forward migrations; `rollback_auth.sql:43-48` only *disables* it.
**Embedded DDL in Rust** (`src/api_server.rs:1568-2070`) re-declares `schema_migrations`, `composition_versions`, `harness_snapshots`, `fermi_forecast_updates`, `forecast_relationships`, `pending_cascades`, `forecast_commitments`, `forecast_splits`, `forecast_spacetime`, `fermi_market_observations`, `forecast_invites`. This is a workaround for PgBouncer splitting multi-statement migrations (`src/api_server.rs:1643, 1666-1677`). No new tables there. The test fixtures (`H:attribution.rs:491`, `scripts/sql/*.sql`, `scripts/spec26_sql_check.sql`, `scripts/loop5_probe_selftest.sql`) are also not new tables.

---

## 2. Append-only tables that already exist, and how they are enforced

None of these is protected the way the target requires (T1/T2: one writer, no UPDATE/DELETE by the app role, as in `abw-invariant-compiler-kit/invariants/templates/pgtap/reject/events_mutation.sql`). Three levels exist:

| Table | Claimed where | DB enforcement | Code reality | Erasable by |
| --- | --- | --- | --- | --- |
| `episode_corrections` | `103:97` | **BEFORE UPDATE trigger raises** (`103:162-172`) | insert-only | DELETE not blocked; FK CASCADE from `episodes`/`agents` (`103:104-105`) |
| `hitl_actions` | `106:12` | **BEFORE UPDATE trigger raises** (`106:63-73`) | insert-only | DELETE not blocked; CASCADE from `agents`/`anomaly_events` (`106:27-28`) |
| `embedding_provenance` | `135:61-64` | **`REVOKE UPDATE, DELETE … FROM PUBLIC`** (`135:106`) | insert-only | Ineffective if the app connects as the table owner/creator, since "the creator role retains full access" (`135:104-105`) [I: the app likely runs as owner] |
| `fermi_forecasts` (scoring tuple only) | `174:73` | **BEFORE UPDATE trigger silently reverts** + WARNING (`174:115-163`) | 23 UPDATE sites still try | Un-resolving (status ≠ resolved) re-opens the tuple by design (`174:117-119`). DELETE allowed |
| `assertion_verifications` | `205:138-142` | Comment only; `human_sourced ⇒ citation` CHECK (`205:159`) | insert-only | CASCADE from `episodes` (`205:121`) |
| `gate_decision_reviews` | `216:31-33,157` | Comment only | insert-only | CASCADE from `gate_decisions` (`216:80`) |
| `gate_decisions` | `214` (implied) | Comment + closed-vocab CHECKs (`214:52-60`) | insert-only (`src/gate_trust.rs:790`) | — |
| `forecast_agent_claims` | `187:42-60` | Comment only ("no UPDATE path and no unique constraint") | insert-only; test DELETE only | FK SET NULL, so it survives parent deletes |
| `carbon_emission_factors` | `238:29,117` | Comment only | insert-only | — |
| `bayesops_posterior_snapshots` | `148:142` | Comment only | insert-only | admin purge DELETE (`H:admin.rs:1514`) |
| `fermi_market_observations` | `099:3,13` | Comment only | insert-only | admin purge DELETE (`H:admin.rs:1558`) |
| `fermi_forecast_updates` | `094:143` (implied) | None | insert-only (9 sites) | admin purge DELETE (`H:admin.rs:1554`) |
| `forecast_commitments` | `140:5,64` ("immutable clock") | `commitment_hash UNIQUE` only | insert-only | admin purge DELETE (`H:admin.rs:1545`); CASCADE from `fermi_forecasts` |
| `forecast_splits` | `140:15,116` | None | insert-only | admin purge DELETE (`H:admin.rs:1549`) |
| `process_projection_commits` | `141:13` | None | insert-only | — |
| `credit_ledger` | `012:20` | None; `tx_type` CHECK never applied (`205:83-84`) | **1 UPDATE** (`H:billing.rs:334`) | — |
| `creature_versions` | `078:51`, `085:19` | None | insert-only | — |
| `secret_access_log` | `039:20` | None | insert-only | — |
| `swarm_telemetry`, `sosa_observations` | `051:20`, `052:42` | None | insert-only | — |
| `knowledge_transfers` | `049:30` | None | never written | — |
| `workspace_action_log` | `125:10,87` | None | **12 UPDATE sites**, so not append-only in practice | CASCADE from `teams` |
| `anomaly_events` | `105:133` | None | **2 UPDATE sites** (`resolved_at/by`) | — |
| `agent_versions` | implied by purpose | None | insert-only | CASCADE from `agents` (`024:4`) |

Cross-cutting risks:

- **Admin purge.** `H:admin.rs:~1500-1590` deletes forecasts and workspaces together with their commitments, splits, spacetime, updates, market observations and BayesOps snapshots in one transaction. Trust history is erased whenever test cruft is cleaned up.
- **Agent deletion.** `AB:store.rs:1352` (`delete_agent`) and `H:admin.rs:2945` cascade into `episodes`, `agent_versions`, `episode_corrections`, `hitl_actions` and `assertion_verifications`.

---

## 3. Needs owner decision

All `drop?` proposals are conservative. None was verified against production row counts.

| Item | Why `drop?` / why unclassifiable | Check before dropping |
| --- | --- | --- |
| `agent_alignments`, `pairwise_coherence`, `knowledge_transfers`, `agent_interaction_policies` (`049`) | AKP foundation; zero code references outside docs (`docs/architecture/AKP_DESIGN.md`) | Prod row counts; whether AKP is still on the roadmap |
| `ar_choreographies` (`041:47`) | Zero code references | AR spatial suite plans (`static/docs/ar-spatial-suite.md`) |
| `flight_telemetry` (`078:105`) | Zero code references; superseded by `telemetry_points`/`sosa_observations` [I] | Row count |
| `voice_assets` (`048b:4`) | Zero code references; `audio_url` columns used instead | `docs/VOICE_FEATURES.md` |
| `migrations_log` (`195:129`) | Exists only so 089/090 don't abort | Nothing; the new core has its own ledger |
| `creatures_rebuild` (`208:138`) | Transient | — |
| `shopping_profiles`, `marketplace_listings`, `marketplace_transactions` (`030`) | Embedding-marketplace product, no ABW noun, but routes are live (`src/api_server.rs:5117-5133`) | Is anyone using it? If yes → `tenant:other` + `replaced:acn` |
| `fermi_forecast_workspaces` view (`139:17`), `route_reason_performance`, `router_override_scorecard` views (`193`) | No Rust readers | `router_override_scorecard` is conceptually important (scores humans vs router). Keep the *query* even if the view goes |
| `bump_agent_persona_version` trigger (`103:188`) | Creates an unrecoverable second version counter (`241:1-24`) | Replace with the agent_version id |
| `workspaces` (referenced, never created) | `H:beacons.rs:217`, `src/calibration.rs:415` query a table no migration creates. Either a prod-only ghost or a dead path [I] | Check prod `\d workspaces`. If absent, both paths error at runtime |
| `relationships`, `verification_tests` | Only in `rollback_auth.sql:48` / `docs/agent-bestiary/MEMORY_SCHEMA.sql:322` | Confirm absent in prod |
| `teams` resolution columns (`147`) | CRUD row holding outcome data | Split them into a `trust:outcome` stream |
| `composition_versions` | Doc seam #4: trust or workspace? Classified `trust:human_decision` [I] | Owner decides whether team changes are trust events |
| `semantic_rules`, `bayesops_*`, `rule_retrievals`, `workspace_intention_signals`, `coherence_evaluations` | Doc seams #2/#3: behaviour-changing but classified workspace | Owner decides; at minimum, record *accepted* rule/fit changes as events |
| `driver_annotations` | `crud:forecast` vs `trust:human_decision` | Is accept/decline a scored human decision? |
| `impersonation_events`, `secret_access_log`, `admin_bypass_events` | Security audit logs: trust stream or ordinary audit? | Recommend at least append-only enforcement |
| `ar_beacons`, `ar_grid_maps` | Tenant unclear: AR suite / architect lens / glasses? | Owner names the tenant |
| `sosa_*`, `observation_sessions` | Shared by Rabble and SimOps; the Refactor plan wants "sensor-data provenance" as substrate | Platform substrate vs `tenant:simops` |
| `contacts`, `user_blocks`, `reports` | Rabble today; platform safety if social goes cross-tenant | — |
| `eval_signals` Loop 5.A rows | Valid table, but rows from `record_forecast_calibration_signals` carry no agent-level information (`src/outcome_trust.rs:24-36`) | Don't migrate them as calibration |

---

## 4. Surprising or important

1. **Shapley credit with validity gates is real and persisted.** `forecast_agent_claims` (append-only claim ledger) → `forecast_attributions` (header with `efficiency_residual`, `reconstruction_error`, `seed`, neutralisation mode) → `forecast_agent_credit` (signed φ) + `forecast_agent_interactions` (synergy/redundancy index). The job refuses to write if residual > 1e-6 (`H:attribution.rs:61,149-152`) and warns at reconstruction > 0.01 (`:65`). Math is in `src/attribution/{counterfactual,roster}.rs`. The target design drops this (ABW doc §Open, seam #1). **Must be carried explicitly.** The claim ledger is the irreplaceable part: "the backlog cannot be reconstructed" (`187:29-31`).
2. **The calibration that looks per-agent isn't.** `eval_signals` rows from `record_forecast_calibration_signals` (`H:forecasts.rs:1837`) copy the team Brier to every agent. 239 signals, one distinct score per forecast (`src/outcome_trust.rs:24-36`). Migrating them as "calibration" would launder noise into history.
3. **A hash-chained harness lineage already exists.** `harness_snapshots` (`140:33-62`) is content-addressed, with `parent_hash`, `surface_changed` and `change_rationale`. `forecast_commitments` (`140:66-80`) is a hash-anchored probability commitment with `anchor_method/anchor_ref`. Its SimOps twin is `process_projection_commits` (`141:34`). These are the closest existing analogues to *agent_version-for-teams* and *commitment*.
4. **Embedding provenance and model-death insurance.** `embedding_provenance` (append-only, REVOKE-enforced, `135:106`), per-row provenance columns on 5 vector tables (`135`, NOT NULL in `136`/`184`), and `embedding_anchors` (`137:20`) for fitting a vendor→open-model translator. The anchors are populated only by a bin (`scripts/embedding_anchors.rs:298`). Easy to lose.
5. **Tool calls are not persisted for hosted agents.** `AgentOutput::tool_invocations` (`src/agent_backend/executor.rs:265`) feeds completeness checks in memory (`src/episode_boundary.rs:335-358`). The only tool-call table is `ground_run_tool_calls` (`242:44`), which covers external grounding runs only [I: no other sink found]. "Tool call" is a core trust noun with almost no legacy data behind it.
6. **Verdicts and gates have the right shape already.** `assertion_verifications` is a derived-current-state verdict log with actor/actor_kind/evidence, and enforces "human_sourced needs a citation" (`205:159`). `gate_decisions` has `undetermined` as a first-class decision (`214:44-47`), and its CHECK vocabularies are contract-tested against Rust (`tests/seam_vocabulary_contract.rs`, `216:72`). `gate_decision_reviews` scores the gates. Good templates for the lattice.
7. **Coordination observations are episodes.** Loop 3 writes `coordinator_observation` episodes into member agents' memory (`200:1-30`). They are excluded from `agent_execution_rollup`, as are `synthetic_correction` episodes, so they don't inflate economics. This is the behaviour-changing seam #3, and it lives inside `episodes.provenance`, not in a table of its own.
8. **Composition history and human acceptance.** `composition_versions` (members, weights, strategist, mission, `member_delta` `212`, proposed/accepted/rejected-by) is the only record of team changes. Acceptance is an in-place UPDATE (`AB:store.rs:3367,3518`), and the table is wiped by CASCADE from `teams`.
9. **Several decision queues mutate status in place:** `bayesops_pending_fits`, `pending_cascades`, `two_reviewer_requests`, `orchestra_membership_requests`, `workspace_action_log.confirmation`, `anomaly_events.resolved_*`. Each holds human decisions (invariant 10) that become event streams. Today, the history of who decided what is partly lost on every transition.
10. **Schema integrity history, as lessons.** The migration set cannot rebuild from empty: 26 files failed (`195:11-14`). `IF NOT EXISTS` silently skipped `fermi_forecasts`' 15 columns (`196`). Boot-replay of DROP/ADD exhausted `creatures`' 1600 attnums (`208:11-19`). The `credit_ledger.tx_type` CHECK was "declared by seventeen migrations and applied by none" because PgBouncer splits statements (`205:83-84`). Users' `id`/`password_hash` were created by no migration (`195:21-24`). `workspaces` is queried but never created.
11. **Episode ↔ workspace link is new and empty by design.** `episodes.workspace_id` (`226`) was not backfilled: only 10 of 5,657 messages had episode ids (`226:17-26`). The new core should not infer historic workspace attribution.
12. **Version counter drift.** `agents.persona_version` ≠ `agent_versions.version_number`, offset by 1 + the number of HITL bumps, and "no arithmetic recovers one from the other" (`241:1-24`). `episodes.produced_by_version_*` (`241`) is the correct stamp. Before `240`, `agent_versions` snapshotted only 7 card fields, so old versions cannot faithfully restore.
13. **DPP publication** (`243`): a stable printed token, snapshot-not-live, release history, and 410-on-revoke semantics. This is the `trust:publication` precedent, but not yet signed or hashed. `carbon_emission_factors` (`238`) is the source ledger behind it.
14. **The money ledger is not double-entry.** `wallets.balance` is a mutable cache next to the single-entry `credit_ledger` with `balance_after`. Stripe double-credit bugs are documented in `182:1-20`. Retire it to ACN, but keep the idempotency lessons.
15. **Secrets at rest:** `a2a_push_configs.auth_credentials` are plaintext (`224:22`). `dpp_publications` tokens are deliberately plaintext (`243`), while `ground_runs` hashes its tokens (`242:22-23`).

---

## 5. Disposition counts

These are computed from the first disposition in each §1 row: 147 table names (146 live + `creatures_rebuild`). Views and functions are not counted (see §1i).

| Disposition | Count | Tables |
| --- | --- | --- |
| `trust:*` | **38** | agent_version 2 (`agent_versions`, `harness_snapshots`) · episode 2 (`episodes`, `ground_runs`) · tool_call 1 (`ground_run_tool_calls`) · claim 4 (`forecast_agent_claims`, `forecast_commitments`, `fermi_forecast_updates`, `process_projection_commits`) · verdict 1 (`assertion_verifications`) · gate_decision 3 (`gate_decisions`, `admin_bypass_events`, `workspace_action_log`) · human_decision 8 (`episode_corrections`, `hitl_actions`, `two_reviewer_requests`, `gate_decision_reviews`, `composition_versions`, `pending_cascades`, `bayesops_pending_fits`, `orchestra_membership_requests`) · evaluator_signal 3 (`eval_runs`, `eval_signals`, `anomaly_events`) · calibration 3 (`forecast_attributions`, `forecast_agent_credit`, `forecast_agent_interactions`) · provenance 7 (`embedding_provenance`, `rule_retrievals`, `select_agent_decisions`, `fermi_market_observations`, `bayesops_posterior_snapshots`, `carbon_emission_factors`, `impersonation_events`) · outcome 1 (`process_spacetime`) · policy 1 (`forecast_splits`) · publication 2 (`dpp_publications`, `dpp_publication_releases`) |
| `crud:*` | **45** | user/auth 10 · forecast 8 · memory 7 · workspace 7 · app 6 · composition 5 · agent 2 |
| `tenant:*` | **40** | rabble 33 · simops 5 · other 2 (`ar_beacons`, `ar_grid_maps`) |
| `replaced:acn` | **5** | `wallets`, `credit_ledger`, `stripe_sessions_processed`, `agent_episode_payouts`, `marketplace_transactions` |
| `derived` | **8** | `communities`, `consolidation_locks`, `agent_evolution`, `agent_timeline_entries`, `dyad_state`, `agent_observability_state`, `forecast_spacetime`, `schema_migrations` |
| `drop?` | **11** | `agent_alignments`, `pairwise_coherence`, `knowledge_transfers`, `agent_interaction_policies`, `ar_choreographies`, `flight_telemetry`, `voice_assets`, `migrations_log`, `creatures_rebuild`, `shopping_profiles`, `marketplace_listings` |

Rows with a secondary disposition: `fermi_forecasts` (crud:forecast + trust:outcome/claim), `teams` (crud:workspace, resolution columns → trust:outcome, budget → ACN), `agents` (crud + card → trust:agent_version), `workspace_action_log` (trust:gate_decision + crud), `coherence_evaluations` (crud + gate), `sosa_observations` (tenant:simops → trust:outcome for ground truth), `driver_annotations` (crud or trust:human_decision), `swarm_activations`/`swarm_events` (tenant + ACN money side).
