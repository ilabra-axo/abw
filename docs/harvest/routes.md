# Legacy HTTP route harvest (reviewed)

- **Source:** `Replicant-Partners/fermi` at `a3cea516` (2026-10-08). The working tree has uncommitted edits to `src/api_server.rs` (+48/−2 lines: 3 new routes, `/api/fleet`, `/api/agents/:agent_id/versions/:version_num/diff` and `/api/agents/:agent_id/versions/timeline`, plus reformatting). Line numbers are from the **working tree**, so they run up to about 30 lines later than HEAD after line 2777. HEAD has 592 routes in `src/api_server.rs`; the working tree has 595.
- **Reviewed:** 2026-11-10. This file replaces the regex first pass from `harvest_routes.py`.
- **Method:**
  1. Every `.route(` in `src/api_server.rs` (595) and `src/api/server.rs` (6) is parsed with balanced parentheses: **601 routes**.
  2. For each handler module, I read the module docs and the comments where its routes are registered, then assigned dispositions per module, with per-route overrides.
  3. Callers come from three sources: (a) `tenants.md`; (b) a stricter rescan of all client sources for `/api/…` substrings, including `$baseUrl/api/…` and `${x}` forms; (c) a fixed-string grep of each route's last literal segments. The sources scanned were `~/rabble/lib`, `~/kask`, `crates/fermi-console`, `crates/abw-cli`, `static/js`, `static/adaptogen-lab`, `templates`, `src/agent_backend` (agent tools), `scripts`, `apps` and `agents`. For drop candidates I also grepped string-concatenation forms by hand.
- **Caveats:**
  - A route with no caller found may still be reached by an external integration, by URLs built at run time, or from production logs, which I didn't check [I].
  - `file:line` is where the route is **registered**, not where the handler is defined.
  - Routes under `.nest("/rabble", …)` are listed with their local path.
  - `ui` routes are reachable by links, so they weren't checked for callers.
  - `harvest_routes.py` no longer reproduces this file: its rules are the old first pass. Its `OUT` points at `.abw-staging/docs/harvest/routes.md`, not `.abw-staging/repo/docs/harvest/routes.md`, so rerunning it won't overwrite this review, but don't treat its output as current.
- **Counts:** `CAP-01` 48, `CAP-02` 31, `CAP-03` 6, `CAP-04` 4, `CAP-05` 28, `CAP-06` 28, `CAP-07` 2, `CAP-08` 4, `CAP-09` 2, `CAP-10` 14, `CAP-11` 3, `CAP-13` 8, `CAP-14` 37, `CAP-15` 56, `drop?` 38, `ops` 24, `replaced:acn` 19, `tenant:dpp` 7, `tenant:fermi-console` 69, `tenant:rabble` 119, `tenant:shopping` 6, `tenant:simops` 11, `tenant:wild` 1, `ui` 36. **Total: 601.** Compared with the first pass, the disposition prefix changed on 229 routes.

## Decisions for the owner

### A. Proposed drops (`drop?`, 38 routes in 9 groups)

1. **Legacy `agent-server` router (6)**: `src/api/server.rs:20-26` (`/health`, `/agents`, `/agents/:id`, `/agents/:id/execute`, `/agents/:id/save`). Its only binary is commented out: `Cargo.toml:188` reads "agent-server archived — superseded by api-server" (`src/bin/legacy/agent-server.rs`). Every function is covered by `/api/agents*` in `api_server`. The first pass marked `/agents/:id/execute` as CAP-01, which was wrong.
2. **Debug probe `/api/debug/startup` (1)**: `src/api_server.rs:2821`. Dumps the `AGENTS_DIR` listing and existence checks (`src/handlers/misc.rs:77`), and it's mounted on the unauthenticated `public_routes` router (`src/api_server.rs:2775`). No caller anywhere. `/api/health` and `/api/admin/schema-health` cover the legitimate need.
3. **Agent knowledge-graph read API (8)**: `/api/agents/:agent_id/kg*`, `src/api_server.rs:3421-3449`. No caller in any client, template, script or agent tool. The grep for `/kg`, `kg/entities`, `kg/facts` and `kg/rules` finds only kask unit strings like "kWh/kg". The KG **store** is Loop 1 output (CAP-01 Remember) and must be kept; only these 8 read routes go. Re-expose them when a UI needs them.
4. **Ontology history, snapshot and diff (3)**: `src/api_server.rs:3408`, `3412`, `3416`. No caller found for `ontology/history`, `ontology/snapshots` or `ontology/diff`. The live `GET /api/agents/:id/ontology` (`3403`) is kept as CAP-01.
5. **Persisted FPL notebooks (6)**: `/api/notebooks*`, `src/api_server.rs:4501-4521`. No caller in the console, kask, the web UI or tools. The console only carries a `notebook_id` field (`crates/fermi-console/src/api/client.rs:225`). kask's brief asks how they differ from `/api/fpl/execute` (`~/kask/adaptogen/simops-v2/ABW_CAPABILITY_BRIEF.md:172`). The stateless `POST /api/fpl/execute` is the live path and is kept (CAP-06).
6. **Generic `/api/shares` (2)**: `src/api_server.rs:4853-4854` → `teams::share_object_handler` and `revoke_share_handler`. Its successor documents it as taking "a free-form `object_type` and perform[ing] zero authorization" (`src/handlers/shares.rs:13`). It's superseded by per-target forecast and portfolio shares, and no client calls it. kask specs *planned* to use it for workspace sharing (`~/kask/adaptogen/simops-v2/specs/01_APP_PRIMITIVE.md:482`), so CAP-15 *Share a workspace* needs a proper replacement, not this route.
7. **Swarm telemetry sessions (6)**: `/api/swarm/sessions*`, `src/api_server.rs:5562-5584`. Superseded by the universal SOSA API, whose docs say "same learning loop as swarm telemetry, but universal" (`src/handlers/observations.rs:7`). `/api/observe/*` has a caller (`~/kask/abw-client.js:712`); `/api/swarm/sessions` has none. The first pass tagged these CAP-14, which was wrong.
8. **Rabble dashboard duplicates (2)**: `/api/dashboard/my-rabbles` (`5256`), `/api/dashboard/boundary-violations` (`5268`). `my-rabbles` duplicates `/api/my/rabbles`, which Rabble actually calls (`~/rabble/lib/services/api_client.dart:415`); only the one-off `scripts/patch_dashboard.py:64` names it. `boundary-violations` has no caller. The other dashboard routes are called by Rabble and are kept.
9. **One-off admin data repairs (4)**: `/api/admin/agents/legacy-slugs` (`4945`), `/api/admin/agents/cleanup-test-cruft` (`4957`), `/api/admin/wipe-fermi-forecasts` (`5056`), `/api/admin/recompose-mutex-groups` (`5065`). The comments at registration call them a v0.10.20 rename, a v0.10.25 test-fixture purge, a "Spec 23 demo cleanup — one-shot wipe" and a "one-shot admin recompose" after a bug fix. The only callers are `abw-cli admin` (`crates/abw-cli/src/commands/admin.rs`) and `scripts/world_cup/respawn_aligned.py:120`. A new core starts with clean data. Keep the code in git history in case a migration needs it.

### B. Other judgement calls

1. **The tenant harvest under-counts Rabble.** `harvest_tenants.py` only matches string literals that *start* with `/api`, so it misses Rabble's `Uri.parse('$baseUrl/api/…')` calls. 27 of the 73 routes that `tenants.md` lists as "no tenant client calls" are in fact called from `~/rabble/lib/services/api_client.dart`: friendships, creature invites, blocks, reports, push, locations, follow, feed events, eject, recap, co-presence and others. Fix the regex before using that list for any decision. This review uses the rescan.
2. **Rabble is 119 routes (`tenant:rabble`).** That covers creatures, swarms, social, chat, governance, push, beacons, dashboard, the swarm algorithm catalogue and Rabble admin moderation. It answers tenants-notes decision 2: it's almost all game CRUD, and only `rabble_workspace` touches agents. Rabble routes with **no caller found**, kept but worth confirming: `/api/beacons/nearby`, `/api/beacons/:beacon_id` and `/api/grid-maps/:map_id` (`3493-3505`; the agent tool `src/agent_backend/tools/domains/spatial.rs:722` emits only `/asset` URLs), `/api/me/workspace` (`4401`), `/api/rabble/:id/qr` (`3552`), `/api/creatures/generate-art-batch` (`5297`) and `PATCH …/goals/:goal_id` (`5219`). `POST /api/swarms/create` (`5240`) appears only in the compiled bundle `static/rabble/main.dart.js`, not in the Rabble source [I].
3. **BayesOps over HTTP has no HTTP caller.** The 9 stateless kernel routes (`/api/bayesops/fit_*`, `predict`, `input_sensitivity`, `compare_scenarios`, `prob_exceeds`, `optimise_for_target`, `posteriors`; `4027-4059`) are exercised only by `tests/bayesops_http_shapes.rs`. The MCP server calls the `posterior` crates directly (`src/bin/agent-mcp-server.rs:59`). I kept them as CAP-06 Loop 5.B, because the kernel is a port candidate. Decide whether the new core exposes the kernels over HTTP, over MCP, or both.
4. **Loop mechanisms that exist but have never been called over HTTP.** These are kept, not dropped, because PROT-06 needs each gate shown firing: `composition/suggestions` and `materialise` (Loop 4.A, `3085-3089`), `composition/propose` (`4353`), `observatory/loops/brier/mechanism` (`3069`), `POST /api/workspaces/:id/resolve` (`4261`), `forecasts/:id/commit` and `benchmark/anchor-sweep` (CAP-07 commitment anchors, `4608-4612`), `simops/workspaces/:id/{process-spacetime,sample-config}` (`4120-4124`), and `pending-cascades/requeue` and `cascade-history` (`4784-4788`). Each is either the missing wiring the README's headline findings describe, or dead. The owner should say which.
5. **Legacy forecast relationships (mig-150).** `/api/forecast-relationships*` (`4711-4720`) calls itself legacy, "superseded by groups" (`src/handlers/relationships/legacy.rs:1`), but the console still calls it (`crates/fermi-console/src/api/client.rs:3048`). It's kept as `tenant:fermi-console`. Migrate the console to `relationship-groups` first, then drop it.
6. **What moves to ACN (19 routes).** Besides wallet, billing, Stripe and agent wallet, I moved these to `replaced:acn`: `/api/agents/:id/fork-pricing` (royalties), `/api/agents/:id/dreaming/topup` (a credit top-up), workspace `budget` GET/POST (CAP-12 *Fund a spawn*), `admin/users/:id/grant` and `admin/workspaces/:id/grant`, and `admin/economics/platform`. SIWE login (`/auth/siwe/*`) stays CAP-14 as authentication. Revisit it if wallet identity moves to ACN.
7. **Teams.** `/api/teams*` carries a composition mission and a coordination strategist (`src/handlers/teams.rs:13`), so I kept it as CAP-02. Its collaboration read models (`teams/:id/shared`, `activity`, `contributions`, `ops`) are console-only and are now `tenant:fermi-console`.
8. **Corrections to the first pass that change ownership:**
   - `specimen` (6 routes) was matched as Rabble by the word "specimen"; it's the agent specimen view, now CAP-05, CAP-09 and CAP-01.
   - `swarm_telemetry` and `observations` were tagged CAP-14 because their paths contain "session".
   - `ecology`, `social` and `dashboard` were tagged `crud:misc`, which isn't in the vocabulary.
   - `marketplace` listings were tagged CAP-14 instead of `tenant:shopping`.
   - `orchestras` moved from CAP-04 to CAP-02 *Hire an agent*, because it's a join registry.
   - The DPP public and publish routes moved to CAP-14 *Publish an artifact*, the doc's named example.
   - `fleet` moved from CAP-02 to CAP-05.
   - `ground` moved from CAP-09 to CAP-08 *Collect evidence*.
9. **Small calls marked [I]:**
   - `observations` → CAP-13 *Observe an outcome*. It's ground truth for projections; CAP-05 is the alternative.
   - `rounds` → CAP-10 *Cap human load*.
   - Notifications → CAP-10.
   - `waitlist` (public and admin) → `ops`.
   - Invites for teams and by token → CAP-15 *Share a workspace*.
   - `agent_funding` and `/api/me/providers` → CAP-14 *Keep secrets*.
10. **The 36 `ui` pages.** These are server-rendered Askama pages. The new core is API-first, so the open question from tenants-notes decision 3 is which pages need a UI at MVP. The three `/fermi-console/{install,install.sh,download}` routes are console distribution and are tagged `tenant:fermi-console`; the console's updater calls `download` (`crates/fermi-console/src/updater.rs:63`).
11. **Shopping (6 routes).** Embedding-marketplace listings are used only by `templates/marketplace.html`; `/api/shopping/profile*` has no caller. Keep it as a tenant or drop it (tenants-notes decision 5).

## Summary by handler module

| Module | Routes | What it does | Disposition(s) |
| --- | --- | --- | --- |
| `creatures` | 55 | Rabble game: creatures, flights, swarms, devices, goals, art, presence, feed | tenant:rabble ×55 |
| `pages` | 37 | Askama HTML pages (+ console installer/download) | ui ×34; tenant:fermi-console ×3 |
| `agents` | 31 | Agent cards: CRUD, registry, versions/diff/timeline, tools/MCP, calibration, fidelity, embeddings, avatar | CAP-01 Version an agent ×16; CAP-01 Invoke a tool ×4; CAP-01 Remember ×4; tenant:fermi-console ×2; CAP-01 Run an episode ×1; CAP-06 Loop 5.A Calibration measurement ×1; CAP-13 Score fidelity ×1; CAP-14 Keep secrets ×1; CAP-06 Learn and improve (loop health) ×1 |
| `workspace` | 27 | Workspace core: list/get, agents & hire, budget, messages (+SSE), files, git, coherence, ontology | CAP-15 Read and write state ×7; CAP-15 Spawn a workspace ×4; CAP-02 Hire an agent ×4; CAP-15 Invoke agents ×4; CAP-04 Measure coherence ×3; replaced:acn ×2; CAP-15 Register an app ×1; CAP-02 Compose teams ×1; CAP-01 Remember ×1 |
| `admin` | 23 | Admin console: stats, users, agents, apps, waitlist, Rabble moderation, one-off repairs | ops ×13; drop? ×4; tenant:rabble ×4; replaced:acn ×2 |
| `forecasts` | 20 | Fermi forecasts, schedules, portfolios, leaderboard | tenant:fermi-console ×20 |
| `social` | 19 | Rabble social: contacts, creature friendships/invites, follow, feed | tenant:rabble ×19 |
| `observatory` | 15 | Observatory: fleet, timeline, anomalies, dyads, HITL queue | CAP-05 Observe ×11; CAP-10 Involve humans ×3; CAP-06 Loop 5.A Calibration measurement ×1 |
| `loops` | 15 | Loop & gate surface: loops, gates, decisions, trace, verification queue, declarations | CAP-06 Learn and improve ×4; CAP-11 Approve or refuse an effect ×2; CAP-10 Endorse a claim ×2; CAP-04 Cascade to members ×1; CAP-10 Sign off a claim ×1; CAP-09 Walk a chain ×1; CAP-08 Collect evidence ×1; CAP-10 Return or decline ×1; CAP-03 Declare an expert contract ×1; CAP-05 Run the evaluators ×1 |
| `workspace::actions` | 15 | Generic App action protocol (mutate_document, fork_state, compare, invoke_member, annotate, accept/reject) | CAP-15 Read and write state ×9; CAP-10 Sign off a claim ×3; tenant:simops ×2; CAP-15 Invoke agents ×1 |
| `apps` | 12 | App registry: CRUD, schema, spawn workspaces, publish/archive | CAP-15 Register an app ×4; CAP-15 Spawn a workspace ×3; CAP-15 Set visibility ×2; CAP-15 Define the canonical document ×1; CAP-15 Build and ship apps ×1; CAP-02 Hire an agent ×1 |
| `auth` | 12 | OAuth, CLI login, API keys, SIWE, logout | CAP-14 Isolate tenants ×12 |
| `bayesops` | 12 | BayesOps kernels over HTTP + workspace state + held refits | CAP-06 Loop 5.B Parameter correction (BayesOps) ×12 |
| `teams` | 11 | Teams (composition + members) and the legacy generic share | CAP-02 Compose teams ×9; drop? ×2 |
| `invites` | 10 | Invite lifecycle (forecast, portfolio, team, by-token) | CAP-15 Share a workspace ×8; tenant:fermi-console ×2 |
| `profile` | 10 | Profile, notifications, secrets, profile page | CAP-14 Keep secrets ×4; CAP-10 Involve humans ×3; CAP-14 Isolate tenants ×2; ui ×1 |
| `eval` | 9 | Eval test cases, runs, rubrics, signals | CAP-05 Evaluate an episode ×9 |
| `orchestras` | 9 | Orchestra registry with admin-gated membership requests | CAP-02 Hire an agent ×9 |
| `kg` | 8 | Agent knowledge-graph read API | drop? ×8 |
| `notebooks` | 8 | Persisted FPL notebooks + stateless FPL execute | drop? ×6; CAP-06 FPL Monte Carlo executor ×1; ops ×1 |
| `governance` | 8 | Rabble block/eject/report | tenant:rabble ×8 |
| `ontology` | 7 | Agent ontology, history/diff, projections | CAP-01 Remember ×4; drop? ×3 |
| `collab` | 7 | Spec 26 provenance/activity read models for teams, forecasts, portfolios | tenant:fermi-console ×7 |
| `simops` | 7 | Deterministic SimOps kernels (cascade, dynamics, rheology, project) | tenant:simops ×7 |
| `xaman` | 7 | Xaman Ek navigator sessions → app drafts | CAP-15 Be discovered ×6; CAP-15 Register an app ×1 |
| `rabble_workspace` | 7 | Rabble↔workspace: flock tick, anchors, batch join, personal workspace | tenant:rabble ×7 |
| `dashboard` | 7 | Rabble spatial dashboard queries + saved locations | tenant:rabble ×5; drop? ×2 |
| `a2a` | 6 | A2A provider: agent card, method dispatch, tasks, push configs, directory | CAP-14 Accept tasks ×6 |
| `consolidation` | 6 | Episodes, dreaming budget, consolidation jobs | CAP-06 Loop 1 Agent learning ×4; CAP-01 Run an episode ×1; replaced:acn ×1 |
| `specimen` | 6 | Specimen view, recent episodes, lineage, live stream, bestiary cards, flow | CAP-05 Observe ×3; CAP-01 Run an episode ×1; CAP-09 Record lineage ×1; CAP-01 Version an agent ×1 |
| `contracts` | 6 | Output-contract sketch compile/suggest/decompile, tool requests | CAP-03 Declare an expert contract ×5; CAP-01 Invoke a tool ×1 |
| `lifecycle` | 6 | Fork, publish checks, publish, archive, restore, fork pricing | CAP-01 Version an agent ×5; replaced:acn ×1 |
| `polymarket` | 6 | Polymarket search/link/snapshot/import/resolution oracle | tenant:fermi-console ×6 |
| `marketplace` | 6 | Embedding marketplace and shopping profiles | tenant:shopping ×6 |
| `swarm_telemetry` | 6 | Drone swarm telemetry sessions (Onto4MAT) | drop? ×6 |
| `observations` | 6 | Universal SOSA observation sessions | CAP-13 Observe an outcome ×6 |
| `api::server (legacy bin)` | 6 | Old standalone agent-server router | drop? ×6 |
| `agent_wallet` | 5 | Agent wallet, earnings, collect, allocate | replaced:acn ×5 |
| `users` | 5 | User profiles, search, collaborators, lookup | CAP-14 Isolate tenants ×4; ui ×1 |
| `wizard` | 5 | Agent-creation wizard helpers | CAP-01 Version an agent ×5 |
| `composition` | 5 | Composition versions accept/reject/propose/dream | CAP-02 Change composition ×5 |
| `forecast_benchmark` | 5 | Commitment anchors, spacetime, timeline, cascade provenance | tenant:fermi-console ×3; CAP-07 Bind a commitment ×2 |
| `rabble_chat` | 5 | Rabble chat, stream, invites, members | tenant:rabble ×5 |
| `workspace::dpp_publish` | 4 | Public, frozen product-passport release and its QR | CAP-14 Publish an artifact ×4 |
| `misc` | 4 | Health, debug, geocode, waitlist | ops ×2; drop? ×1; tenant:rabble ×1 |
| `beacons` | 4 | AR beacons and grid maps (Rabble) | tenant:rabble ×4 |
| `billing` | 4 | Stripe tiers, checkout, webhook, dev top-up | replaced:acn ×4 |
| `workspace::outputs` | 4 | Typed workspace outputs and cross-workspace dependencies | CAP-15 Read and write state ×4 |
| `shares` | 4 | Per-target forecast/portfolio shares | tenant:fermi-console ×4 |
| `relationships::groups` | 4 | Relationship groups (Spec 25) | tenant:fermi-console ×4 |
| `impersonation` | 4 | Admin read-only 'view as user' | ops ×3; CAP-14 Isolate tenants ×1 |
| `swarm_algorithms` | 4 | Swarm formation algorithm catalogue | tenant:rabble ×4 |
| `ground` | 3 | Grounding service for externally hosted agents | CAP-08 Collect evidence ×3 |
| `metrics` | 3 | Episode detail, platform and agent metrics | CAP-01 Run an episode ×1; ops ×1; CAP-05 Observe ×1 |
| `qr_codes` | 3 | QR images (Rabble, DPP) and QR join | tenant:rabble ×2; tenant:dpp ×1 |
| `workspace::lens_actions` | 3 | Regulatory Lens render/compare/flag-divergence actions | tenant:dpp ×3 |
| `wallet` | 3 | User wallet, transactions, transfer | replaced:acn ×3 |
| `forecast_git` | 3 | Forecast history/diff/revert on workspace git | tenant:fermi-console ×3 |
| `annotations` | 3 | Driver annotations on forecasts | tenant:fermi-console ×3 |
| `relationships::legacy` | 3 | mig-150 forecast relationships (legacy) | tenant:fermi-console ×3 |
| `pending_cascades` | 3 | Operator-gated cascade queue | tenant:fermi-console ×3 |
| `admin_rbac` | 3 | RBAC orphans/reassign/heal | ops ×3 |
| `push` | 3 | Web push (VAPID, subscribe, proximity) | tenant:rabble ×3 |
| `mcp` | 2 | Per-agent MCP manifest + JSON-RPC | CAP-14 Serve tools ×2 |
| `dreaming_maturity` | 2 | Loop 1 maturity (agent and fleet) | CAP-06 Loop 1 Agent learning ×2 |
| `composition_evolution` | 2 | Loop 4.A composition suggestions from Shapley | CAP-02 Change composition ×2 |
| `ecology` | 2 | Ecology lens over agents | CAP-05 Observe ×2 |
| `simops_benchmark` | 2 | SimOps projection spacetime + sample config | tenant:simops ×2 |
| `relationships::membership` | 2 | Forecast ↔ group membership | tenant:fermi-console ×2 |
| `agent_funding` | 1 | Is the agent runnable (owner keys present) | CAP-14 Keep secrets ×1 |
| `fleet` | 1 | Fleet triage, one row per agent | CAP-05 Observe ×1 |
| `rounds` | 1 | 'What needs you' queue | CAP-10 Cap human load ×1 |
| `bestiary` | 1 | Bestiary register | CAP-01 Version an agent ×1 |
| `gates` | 1 | Gate enforcement surface | CAP-11 Approve or refuse an effect ×1 |
| `ops` | 1 | Spec 27 team ops board | tenant:fermi-console ×1 |
| `execution` | 1 | Execute an agent | CAP-01 Run an episode ×1 |
| `execution_stream` | 1 | Execute an agent with SSE progress | CAP-01 Run an episode ×1 |
| `evolution` | 1 | Agent evolution badge | CAP-06 Learn and improve ×1 |
| `workspace::refit` | 1 | BayesOps refit hook (Loop 5.B) | CAP-06 Loop 5.B Parameter correction (BayesOps) ×1 |
| `workspace::claim_evaluation` | 1 | Regulatory claim evaluation action (Adaptogen Lab) | tenant:dpp ×1 |
| `workspace::bom_pricing` | 1 | BOM pricing action (DPP) | tenant:dpp ×1 |
| `workspace::carbon` | 1 | Carbon footprint action (DPP) | tenant:dpp ×1 |
| `wild` | 1 | Wild photo-identification action | tenant:wild ×1 |
| `workspace::resolution` | 1 | Resolve any workspace (generalised forecast resolution) | CAP-13 Observe an outcome ×1 |
| `relationships::apply` | 1 | Apply a pending cascade | tenant:fermi-console ×1 |
| `relationships::undo` | 1 | Undo a cascade | tenant:fermi-console ×1 |
| `relationships::requeue` | 1 | Requeue cascades | tenant:fermi-console ×1 |
| `economics` | 1 | Platform cost vs revenue | replaced:acn ×1 |
| `rbac_self_check` | 1 | RBAC self-diagnostic | ops ×1 |
| `streams` | 1 | Creature SSE stream | tenant:rabble ×1 |
| `(inline)` | 1 | Rabble SPA redirect | tenant:rabble ×1 |

## All routes

Callers come from the rescan described in the header. "no caller found" means none of the three sources found one. `src-registry` means only `src/command_registry.rs`, `surface.rs` or `loop_api.rs` name it.

| Method | Path | Handler | file:line | Disposition | Note |
| --- | --- | --- | --- | --- | --- |
| GET | `/` | `pages::landing` | `src/api_server.rs:2776` | ui |  |
| GET | `/dpp/:token` | `workspace::dpp_publish::public_dpp_redirect` | `src/api_server.rs:2780` | CAP-14 Publish an artifact | released product passport (frozen snapshot); DPP is the named example in CAP-14; no caller found (first pass: `tenant:dpp`) |
| GET | `/api/dpp/p/:token` | `workspace::dpp_publish::public_dpp_handler` | `src/api_server.rs:2784` | CAP-14 Publish an artifact | released product passport (frozen snapshot); DPP is the named example in CAP-14; callers: adaptogen-lab (first pass: `tenant:dpp`) |
| GET | `/api/dpp/p/:token/qr` | `workspace::dpp_publish::public_dpp_qr_handler` | `src/api_server.rs:2788` | CAP-14 Publish an artifact | released product passport (frozen snapshot); DPP is the named example in CAP-14; no caller found (first pass: `tenant:dpp`) |
| GET | `/aspiration` | `pages::aspiration` | `src/api_server.rs:2792` | ui |  |
| GET | `/catalogue` | `pages::catalogue` | `src/api_server.rs:2793` | ui |  |
| GET | `/fermi-console/install` | `pages::install_page` | `src/api_server.rs:2799` | tenant:fermi-console | console installer/download (desktop distribution); callers: web-ui |
| GET | `/fermi-console/install.sh` | `pages::install_script` | `src/api_server.rs:2800` | tenant:fermi-console | console installer/download (desktop distribution); callers: web-ui |
| GET | `/fermi-console/download` | `pages::fermi_console_download` | `src/api_server.rs:2807` | tenant:fermi-console | console installer/download (desktop distribution); callers: fermi-console, scripts |
| GET | `/apps` | `pages::apps_catalogue_view` | `src/api_server.rs:2811` | ui |  |
| GET | `/apps/:slug` | `pages::app_detail_view` | `src/api_server.rs:2812` | ui |  |
| GET | `/docs` | `pages::docs_view` | `src/api_server.rs:2813` | ui |  |
| GET | `/docs/:slug` | `pages::docs_view` | `src/api_server.rs:2814` | ui |  |
| GET | `/agent/:agent_id` | `pages::agent_detail` | `src/api_server.rs:2815` | ui |  |
| GET | `/agent/:agent_id/ontology` | `pages::ontology_view` | `src/api_server.rs:2816` | ui |  (first pass: `CAP-01`) |
| GET | `/api/health` | `misc::health` | `src/api_server.rs:2820` | ops | health; callers: fermi-console, kask, scripts |
| GET | `/api/debug/startup` | `misc::debug_startup` | `src/api_server.rs:2821` | drop? | debug probe: dumps `AGENTS_DIR` listing (`src/handlers/misc.rs:77`); no caller anywhere |
| GET | `/api/geocode` | `misc::geocode_search_handler` | `src/api_server.rs:2822` | tenant:rabble | geocoding proxy for Rabble location picker; callers: rabble (first pass: `CAP-01`) |
| GET | `/api/forecasts/public` | `forecasts::public_forecasts_handler` | `src/api_server.rs:2824` | tenant:fermi-console | callers: fermi-console |
| GET | `/api/invites/by-token/:token` | `invites::get_invite_by_token_handler` | `src/api_server.rs:2832` | CAP-15 Share a workspace | invite lifecycle (team / by-token) [I]; callers: web-ui (first pass: `CAP-14`) |
| GET | `/invites/:token` | `pages::invite_landing_view` | `src/api_server.rs:2842` | ui |  (first pass: `CAP-14`) |
| GET | `/mcp/agents/:agent_id` | `mcp::mcp_agent_manifest` | `src/api_server.rs:2844` | CAP-14 Serve tools | per-agent MCP manifest + JSON-RPC; external callers [I]; callers: agent-tools, kask, web-ui |
| POST | `/mcp/agents/:agent_id` | `mcp::mcp_agent_rpc` | `src/api_server.rs:2848` | CAP-14 Serve tools | per-agent MCP manifest + JSON-RPC; external callers [I]; callers: agent-tools, kask, web-ui |
| GET | `/a2a/:slug/agent-card.json` | `a2a::agent_card_handler` | `src/api_server.rs:2852` | CAP-14 Accept tasks | A2A provider; external callers by protocol [I]; no caller found |
| POST | `/a2a/:slug/:method` | `a2a::method_dispatch_handler` | `src/api_server.rs:2866` | CAP-14 Accept tasks | A2A provider; external callers by protocol [I]; callers: src-registry |
| POST | `/v1/ground/runs` | `ground::open_run_handler` | `src/api_server.rs:2875` | CAP-08 Collect evidence | grounding service for externally hosted agents: ABW runs the tools, then grades the output (`src/handlers/ground.rs:1`); caller: `scripts/ground_smoke.sh`; callers: scripts (first pass: `CAP-09`) |
| POST | `/v1/ground/runs/:run_id/tools/:tool` | `ground::run_tool_handler` | `src/api_server.rs:2876` | CAP-08 Collect evidence | grounding service for externally hosted agents: ABW runs the tools, then grades the output (`src/handlers/ground.rs:1`); caller: `scripts/ground_smoke.sh`; callers: scripts (first pass: `CAP-09`) |
| POST | `/v1/ground/runs/:run_id/output` | `ground::submit_output_handler` | `src/api_server.rs:2880` | CAP-08 Collect evidence | grounding service for externally hosted agents: ABW runs the tools, then grades the output (`src/handlers/ground.rs:1`); caller: `scripts/ground_smoke.sh`; callers: scripts (first pass: `CAP-09`) |
| GET | `/a2a/:slug/tasks/:episode_id` | `a2a::get_task_handler` | `src/api_server.rs:2884` | CAP-14 Accept tasks | A2A provider; external callers by protocol [I]; no caller found |
| GET | `/.well-known/agent-directory.json` | `a2a::agent_directory_handler` | `src/api_server.rs:2889` | CAP-14 Accept tasks | A2A provider; external callers by protocol [I]; no caller found |
| POST/GET | `/a2a/:slug/tasks/:episode_id/pushNotificationConfigs` | `a2a::create_push_config_handler, a2a::list_push_configs_handler` | `src/api_server.rs:2893` | CAP-14 Accept tasks | A2A provider; external callers by protocol [I]; no caller found |
| GET/DELETE | `/a2a/:slug/tasks/:episode_id/pushNotificationConfigs/:config_id` | `a2a::get_push_config_handler, a2a::delete_push_config_handler` | `src/api_server.rs:2898` | CAP-14 Accept tasks | A2A provider; external callers by protocol [I]; no caller found |
| GET | `/api/agents` | `agents::list_agents` | `src/api_server.rs:2903` | CAP-01 Version an agent | agent card CRUD / registry / versions; callers: fermi-console, kask, scripts, web-ui |
| GET | `/api/agents/:agent_id` | `agents::get_agent_handler` | `src/api_server.rs:2910` | CAP-01 Version an agent | agent card CRUD / registry / versions; callers: app/agent manifests, fermi-console, kask, scripts, web-ui |
| GET | `/api/agents/curated` | `agents::list_curated_agents_handler` | `src/api_server.rs:2914` | CAP-01 Version an agent | agent card CRUD / registry / versions; callers: web-ui |
| GET | `/api/agents/contributions` | `agents::agent_contributions_handler` | `src/api_server.rs:2932` | tenant:fermi-console | per-agent contribution to forecasts (Shapley view) [I]; callers: fermi-console (first pass: `CAP-01`) |
| GET | `/api/apps` | `apps::list_apps_handler` | `src/api_server.rs:2940` | CAP-15 Register an app | callers: abw-cli, kask, other-crates, scripts, web-ui |
| GET | `/api/apps/:slug` | `apps::get_app_handler` | `src/api_server.rs:2941` | CAP-15 Register an app | callers: abw-cli, kask, scripts, web-ui |
| GET | `/api/apps/:slug/schema` | `apps::get_app_schema_handler` | `src/api_server.rs:2942` | CAP-15 Define the canonical document | callers: adaptogen-lab, app/agent manifests |
| POST | `/api/waitlist` | `misc::waitlist_handler` | `src/api_server.rs:2946` | ops | public signup waitlist (pre-launch gate) [I]; callers: kask, web-ui (first pass: `CAP-14`) |
| GET | `/api/models/catalogue` | `agents::model_catalogue_handler` | `src/api_server.rs:2947` | CAP-01 Run an episode | model catalogue for version config; callers: web-ui |
| GET | `/api/agents/:agent_id/avatar` | `agents::get_cached_avatar` | `src/api_server.rs:2951` | CAP-01 Version an agent | avatar (cosmetic card field); callers: web-ui |
| GET | `/api/agents/:agent_id/episodes` | `consolidation::get_agent_episodes_handler` | `src/api_server.rs:2955` | CAP-01 Run an episode | episode list; callers: web-ui (first pass: `CAP-06`) |
| GET | `/api/episodes/:episode_id` | `metrics::get_episode_detail_handler` | `src/api_server.rs:2959` | CAP-01 Run an episode | episode record; callers: scripts, web-ui |
| GET | `/api/metrics/platform` | `metrics::platform_metrics_handler` | `src/api_server.rs:2963` | ops | platform metrics; callers: web-ui |
| GET | `/api/agents/:agent_id/metrics` | `metrics::agent_metrics_handler` | `src/api_server.rs:2967` | CAP-05 Observe | per-agent metrics; callers: adaptogen-lab, web-ui (first pass: `ops`) |
| GET | `/api/agents/:agent_id/eval/test-cases` | `eval::list_eval_test_cases_handler` | `src/api_server.rs:2972` | CAP-05 Evaluate an episode | test cases, runs, rubrics, signals; callers: web-ui |
| POST | `/api/agents/:agent_id/eval/test-cases` | `eval::create_eval_test_case_handler` | `src/api_server.rs:2976` | CAP-05 Evaluate an episode | test cases, runs, rubrics, signals; callers: web-ui |
| GET | `/api/agents/:agent_id/funding` | `agent_funding::get_agent_funding_handler` | `src/api_server.rs:2988` | CAP-14 Keep secrets | is the agent runnable, i.e. has its owner uploaded provider keys (`src/handlers/agent_funding.rs:1`); callers: web-ui (first pass: `CAP-01`) |
| GET | `/api/agents/:agent_id/mcp-servers` | `agents::get_agent_mcp_servers_handler` | `src/api_server.rs:2998` | CAP-01 Invoke a tool | tool/MCP/skill allowlist of a version; callers: web-ui (first pass: `CAP-14`) |
| POST | `/api/agents/:agent_id/mcp-servers/test` | `agents::test_agent_mcp_server_handler` | `src/api_server.rs:3002` | CAP-01 Invoke a tool | tool/MCP/skill allowlist of a version; callers: web-ui (first pass: `CAP-14`) |
| GET | `/api/agents/:agent_id/published-tools` | `agents::get_agent_published_tools_handler` | `src/api_server.rs:3011` | CAP-01 Invoke a tool | tool/MCP/skill allowlist of a version; callers: web-ui |
| GET | `/api/skills` | `agents::list_skills_handler` | `src/api_server.rs:3019` | CAP-01 Invoke a tool | tool/MCP/skill allowlist of a version; callers: web-ui |
| GET | `/api/fleet` | `fleet::fleet_handler` | `src/api_server.rs:3023` | CAP-05 Observe | fleet triage, one row per agent (new in working tree); callers: scripts, web-ui (first pass: `CAP-02`) |
| PUT | `/api/agents/:agent_id/eval/test-cases/:test_case_id` | `eval::update_eval_test_case_handler` | `src/api_server.rs:3024` | CAP-05 Evaluate an episode | test cases, runs, rubrics, signals; callers: web-ui |
| DELETE | `/api/agents/:agent_id/eval/test-cases/:test_case_id` | `eval::delete_eval_test_case_handler` | `src/api_server.rs:3028` | CAP-05 Evaluate an episode | test cases, runs, rubrics, signals; callers: web-ui |
| POST | `/api/agents/:agent_id/eval/test-cases/generate-rubrics` | `eval::generate_rubrics_handler` | `src/api_server.rs:3032` | CAP-05 Evaluate an episode | test cases, runs, rubrics, signals; callers: web-ui |
| POST | `/api/me/eval/runs/batch` | `eval::batch_eval_run_handler` | `src/api_server.rs:3036` | CAP-05 Evaluate an episode | test cases, runs, rubrics, signals; callers: web-ui (first pass: `CAP-14`) |
| POST | `/api/agents/:agent_id/eval/run` | `eval::trigger_eval_run_handler` | `src/api_server.rs:3040` | CAP-05 Evaluate an episode | test cases, runs, rubrics, signals; callers: web-ui |
| GET | `/api/agents/:agent_id/eval/runs` | `eval::list_eval_runs_handler` | `src/api_server.rs:3044` | CAP-05 Evaluate an episode | test cases, runs, rubrics, signals; callers: app/agent manifests, web-ui |
| GET | `/api/agents/:agent_id/eval/runs/:run_id/signals` | `eval::list_eval_signals_handler` | `src/api_server.rs:3048` | CAP-05 Evaluate an episode | test cases, runs, rubrics, signals; callers: web-ui |
| GET | `/api/observatory/fleet/summary` | `observatory::fleet_summary_handler` | `src/api_server.rs:3054` | CAP-05 Observe | callers: web-ui (first pass: `CAP-02`) |
| POST | `/api/observatory/fleet/scan` | `observatory::fleet_scan_handler` | `src/api_server.rs:3058` | CAP-05 Observe | callers: web-ui (first pass: `CAP-02`) |
| GET | `/api/observatory/fleet/agents` | `observatory::fleet_agents_handler` | `src/api_server.rs:3062` | CAP-05 Observe | callers: web-ui (first pass: `CAP-02`) |
| GET | `/api/observatory/loops/brier/mechanism` | `observatory::loop5_mechanism_handler` | `src/api_server.rs:3069` | CAP-06 Loop 5.A Calibration measurement | admin mechanism probe; no caller found (relevant to PROT-06 'show each gate firing') |
| GET | `/api/observatory/loops/dreaming/maturity` | `dreaming_maturity::fleet_dreaming_maturity_handler` | `src/api_server.rs:3076` | CAP-06 Loop 1 Agent learning | did dreaming produce knowledge; callers: web-ui |
| GET | `/api/workspaces/:workspace_id/composition/suggestions` | `composition_evolution::composition_suggestions_handler` | `src/api_server.rs:3085` | CAP-02 Change composition | Loop 4.A proposal generator from Shapley; no client caller found; no caller found |
| POST | `/api/workspaces/:workspace_id/composition/suggestions/materialise` | `composition_evolution::materialise_composition_proposal_handler` | `src/api_server.rs:3089` | CAP-02 Change composition | Loop 4.A proposal generator from Shapley; no client caller found; no caller found |
| POST | `/api/observatory/dyads/auto-form` | `observatory::auto_form_dyads_handler` | `src/api_server.rs:3094` | CAP-05 Observe | agent dyads/relationships (social profile) [I]; callers: web-ui |
| POST | `/api/observatory/agents/:agent_id/backfill-social` | `observatory::backfill_social_handler` | `src/api_server.rs:3098` | CAP-05 Observe | agent dyads/relationships (social profile) [I]; callers: web-ui |
| PATCH | `/api/observatory/dyads/:dyad_id` | `observatory::patch_dyad_profile_handler` | `src/api_server.rs:3102` | CAP-05 Observe | agent dyads/relationships (social profile) [I]; callers: web-ui |
| GET | `/api/observatory/agents/:agent_id/relationships` | `observatory::agent_relationships_handler` | `src/api_server.rs:3106` | CAP-05 Observe | agent dyads/relationships (social profile) [I]; callers: web-ui |
| GET | `/api/observatory/agents/:agent_id/timeline` | `observatory::get_agent_timeline_handler` | `src/api_server.rs:3111` | CAP-05 Observe | callers: web-ui |
| GET | `/api/observatory/agents/:agent_id/dyads` | `observatory::list_agent_dyads_handler` | `src/api_server.rs:3115` | CAP-05 Observe | agent dyads/relationships (social profile) [I]; callers: web-ui |
| GET | `/api/observatory/agents/:agent_id/anomalies` | `observatory::list_agent_anomalies_handler` | `src/api_server.rs:3119` | CAP-05 Observe | callers: app/agent manifests, web-ui |
| GET | `/api/observatory/agents/:agent_id/loops` | `loops::agent_loops_handler` | `src/api_server.rs:3138` | CAP-06 Learn and improve | per-agent loop state; callers: scripts, web-ui |
| GET | `/api/agents/:agent_id/coordination-notes` | `loops::agent_coordination_notes_handler` | `src/api_server.rs:3142` | CAP-04 Cascade to members | coordination notes written to member memory; callers: web-ui |
| GET | `/api/loops` | `loops::list_loops_handler` | `src/api_server.rs:3155` | CAP-06 Learn and improve | loop surface; callers: scripts, web-ui |
| GET | `/api/loops/actions` | `loops::list_loop_actions_handler` | `src/api_server.rs:3157` | CAP-06 Learn and improve | callers: scripts, web-ui |
| GET | `/api/loops/:loop_id` | `loops::get_loop_handler` | `src/api_server.rs:3161` | CAP-06 Learn and improve | callers: scripts, tests, web-ui |
| GET | `/api/gates` | `loops::list_gates_handler` | `src/api_server.rs:3172` | CAP-11 Approve or refuse an effect | callers: scripts, web-ui (first pass: `CAP-06`) |
| GET | `/api/gates/:gate_id/decisions` | `loops::gate_decisions_handler` | `src/api_server.rs:3174` | CAP-11 Approve or refuse an effect | gate decision log; callers: web-ui (first pass: `CAP-06`) |
| POST | `/api/gates/:gate_id/decisions/:decision_id/review` | `loops::review_gate_decision_handler` | `src/api_server.rs:3189` | CAP-10 Sign off a claim | human review of a gate decision; callers: web-ui (first pass: `CAP-06`) |
| GET | `/api/episodes/:episode_id/trace` | `loops::episode_trace_handler` | `src/api_server.rs:3202` | CAP-09 Walk a chain | episode trace; callers: web-ui (first pass: `CAP-06`) |
| GET | `/api/verification-queue` | `loops::verification_queue_handler` | `src/api_server.rs:3214` | CAP-10 Endorse a claim | verification queue for humans; callers: web-ui (first pass: `CAP-06`) |
| POST | `/api/verification-queue/:assertion_id/settle` | `loops::settle_verification_handler` | `src/api_server.rs:3218` | CAP-10 Endorse a claim | human settles an assertion; callers: web-ui (first pass: `CAP-06`) |
| POST | `/api/episodes/:episode_id/probe` | `loops::probe_field_handler` | `src/api_server.rs:3229` | CAP-08 Collect evidence | probe a claimed field; callers: web-ui (first pass: `CAP-06`) |
| POST | `/api/episodes/:episode_id/contradict` | `loops::contradict_field_handler` | `src/api_server.rs:3239` | CAP-10 Return or decline | human contradicts a claimed field; callers: web-ui (first pass: `CAP-06`) |
| GET | `/api/declarations` | `loops::list_declarations_handler` | `src/api_server.rs:3256` | CAP-03 Declare an expert contract | declaration ladder; callers: web-ui (first pass: `CAP-06`) |
| GET | `/api/evaluators` | `loops::list_evaluators_handler` | `src/api_server.rs:3264` | CAP-05 Run the evaluators | evaluator board; callers: scripts, web-ui (first pass: `CAP-06`) |
| POST | `/api/observatory/agents/:agent_id/scan` | `observatory::trigger_agent_scan_handler` | `src/api_server.rs:3268` | CAP-05 Observe | callers: web-ui |
| GET | `/api/observatory/hitl` | `observatory::list_hitl_queue_handler` | `src/api_server.rs:3272` | CAP-10 Involve humans | Loop 2 human-correction queue; callers: app/agent manifests, web-ui (first pass: `CAP-05`) |
| POST | `/api/observatory/hitl/:event_id/action` | `observatory::record_hitl_action_handler` | `src/api_server.rs:3276` | CAP-10 Involve humans | Loop 2 human-correction queue; callers: web-ui (first pass: `CAP-05`) |
| POST | `/api/observatory/hitl/consensus/:request_id` | `observatory::confirm_two_reviewer_handler` | `src/api_server.rs:3281` | CAP-10 Involve humans | Loop 2 human-correction queue; callers: web-ui (first pass: `CAP-05`) |
| GET | `/api/agents/:agent_id/dependencies` | `agents::get_agent_dependencies_handler` | `src/api_server.rs:3285` | CAP-01 Version an agent | agent card CRUD / registry / versions; callers: web-ui |
| GET | `/api/agents/:id/calibration` | `agents::get_agent_calibration_handler` | `src/api_server.rs:3289` | CAP-06 Loop 5.A Calibration measurement | callers: app/agent manifests, fermi-console, web-ui (first pass: `tenant:fermi-console`) |
| GET | `/api/agents/:id/fidelity` | `agents::agent_fidelity_handler` | `src/api_server.rs:3295` | CAP-13 Score fidelity | callers: web-ui (first pass: `CAP-01`) |
| GET | `/api/agents/:id/competition-stats` | `agents::agent_competition_stats_handler` | `src/api_server.rs:3301` | tenant:fermi-console | forecast competition stats per agent [I]; callers: web-ui (first pass: `CAP-01`) |
| GET | `/api/me/providers` | `agents::list_my_providers_handler` | `src/api_server.rs:3305` | CAP-14 Keep secrets | which provider keys the caller has; callers: web-ui |
| GET | `/api/me/loop-health` | `agents::loop_health_handler` | `src/api_server.rs:3309` | CAP-06 Learn and improve (loop health) | callers: web-ui (first pass: `ops`) |
| GET | `/api/me/rounds` | `rounds::rounds_handler` | `src/api_server.rs:3315` | CAP-10 Cap human load | 'what needs you, in order' queue [I]; callers: web-ui (first pass: `CAP-14`) |
| GET | `/api/bestiary` | `bestiary::bestiary_handler` | `src/api_server.rs:3319` | CAP-01 Version an agent | one register, three lenses; callers: web-ui |
| GET | `/api/gates/enforcement` | `gates::gates_handler` | `src/api_server.rs:3334` | CAP-11 Approve or refuse an effect | what was refused / cannot be refused; callers: scripts, web-ui |
| GET | `/api/specimen/:agent_name` | `specimen::specimen_handler` | `src/api_server.rs:3341` | CAP-05 Observe | one specimen, three tabs; callers: web-ui (first pass: `tenant:rabble`) |
| GET | `/api/episodes/recent` | `specimen::recent_episodes_handler` | `src/api_server.rs:3346` | CAP-01 Run an episode | callers: scripts, web-ui (first pass: `tenant:rabble`) |
| GET | `/api/episodes/:episode_id/lineage` | `specimen::episode_lineage_handler` | `src/api_server.rs:3351` | CAP-09 Record lineage | callers: web-ui (first pass: `tenant:rabble`) |
| GET | `/api/stream` | `specimen::stream_handler` | `src/api_server.rs:3356` | CAP-05 Observe | live platform stream (SSE); callers: web-ui (first pass: `tenant:rabble`) |
| GET | `/api/bestiary/cards` | `specimen::bestiary_cards_handler` | `src/api_server.rs:3358` | CAP-01 Version an agent | callers: web-ui (first pass: `tenant:rabble`) |
| GET | `/api/workspaces/:workspace_id/flow` | `specimen::workspace_flow_handler` | `src/api_server.rs:3364` | CAP-05 Observe | workspace message flow view; callers: web-ui (first pass: `tenant:rabble`) |
| GET | `/api/me/apps-health` | `apps::apps_health_handler` | `src/api_server.rs:3368` | CAP-15 Build and ship apps | owner's app health; callers: web-ui (first pass: `ops`) |
| GET | `/api/agents/:agent_id/versions` | `agents::list_agent_versions_handler` | `src/api_server.rs:3372` | CAP-01 Version an agent | agent card CRUD / registry / versions; callers: web-ui (first pass: `ops`) |
| GET | `/api/agents/:agent_id/versions/:version_num` | `agents::get_agent_version_handler` | `src/api_server.rs:3376` | CAP-01 Version an agent | agent card CRUD / registry / versions; callers: web-ui (first pass: `ops`) |
| GET | `/api/agents/:agent_id/versions/:version_num/diff` | `agents::diff_agent_version_handler` | `src/api_server.rs:3385` | CAP-01 Version an agent | agent card CRUD / registry / versions; callers: web-ui (first pass: `ops`) |
| GET | `/api/agents/:agent_id/versions/timeline` | `agents::agent_version_timeline_handler` | `src/api_server.rs:3394` | CAP-01 Version an agent | agent card CRUD / registry / versions; callers: web-ui (first pass: `ops`) |
| GET | `/api/agents/:agent_id/ontology` | `ontology::get_ontology` | `src/api_server.rs:3403` | CAP-01 Remember | agent ontology; callers: web-ui |
| GET | `/api/agents/:agent_id/ontology/history` | `ontology::get_ontology_history` | `src/api_server.rs:3408` | drop? | ontology history/snapshot/diff: no caller in any client, template or tool (first pass: `CAP-01`) |
| GET | `/api/agents/:agent_id/ontology/snapshots/:snapshot_id` | `ontology::get_ontology_snapshot` | `src/api_server.rs:3412` | drop? | ontology history/snapshot/diff: no caller in any client, template or tool (first pass: `CAP-01`) |
| GET | `/api/agents/:agent_id/ontology/diff` | `ontology::get_ontology_diff` | `src/api_server.rs:3416` | drop? | ontology history/snapshot/diff: no caller in any client, template or tool (first pass: `CAP-01`) |
| GET | `/api/agents/:agent_id/kg` | `kg::kg_overview_handler` | `src/api_server.rs:3421` | drop? | KG read API: no caller in any client, template or tool (store itself is CAP-01 Remember / Loop 1) (first pass: `CAP-01`) |
| GET | `/api/agents/:agent_id/kg/entities` | `kg::list_entities_handler` | `src/api_server.rs:3425` | drop? | KG read API: no caller in any client, template or tool (store itself is CAP-01 Remember / Loop 1) (first pass: `CAP-01`) |
| GET | `/api/agents/:agent_id/kg/entities/:entity_id` | `kg::get_entity_handler` | `src/api_server.rs:3429` | drop? | KG read API: no caller in any client, template or tool (store itself is CAP-01 Remember / Loop 1) (first pass: `CAP-01`) |
| GET | `/api/agents/:agent_id/kg/entities/:entity_id/facts` | `kg::get_entity_facts_handler` | `src/api_server.rs:3433` | drop? | KG read API: no caller in any client, template or tool (store itself is CAP-01 Remember / Loop 1) (first pass: `CAP-01`) |
| GET | `/api/agents/:agent_id/kg/facts` | `kg::list_facts_handler` | `src/api_server.rs:3437` | drop? | KG read API: no caller in any client, template or tool (store itself is CAP-01 Remember / Loop 1) (first pass: `CAP-01`) |
| GET | `/api/agents/:agent_id/kg/rules` | `kg::list_rules_handler` | `src/api_server.rs:3441` | drop? | KG read API: no caller in any client, template or tool (store itself is CAP-01 Remember / Loop 1) (first pass: `CAP-01`) |
| GET | `/api/agents/:agent_id/kg/rules/:rule_id` | `kg::get_rule_handler` | `src/api_server.rs:3445` | drop? | KG read API: no caller in any client, template or tool (store itself is CAP-01 Remember / Loop 1) (first pass: `CAP-01`) |
| GET | `/api/agents/:agent_id/kg/communities` | `kg::list_communities_handler` | `src/api_server.rs:3449` | drop? | KG read API: no caller in any client, template or tool (store itself is CAP-01 Remember / Loop 1) (first pass: `CAP-01`) |
| GET | `/api/agents/:agent_id/wallet` | `agent_wallet::get_agent_wallet_handler` | `src/api_server.rs:3454` | replaced:acn | agent wallet/earnings/collect/allocate; callers: web-ui |
| GET | `/api/agents/:agent_id/earnings` | `agent_wallet::get_agent_earnings_handler` | `src/api_server.rs:3458` | replaced:acn | agent wallet/earnings/collect/allocate; no caller found |
| POST | `/api/agents/:agent_id/collect` | `agent_wallet::collect_handler` | `src/api_server.rs:3462` | replaced:acn | agent wallet/earnings/collect/allocate; callers: web-ui |
| POST | `/api/agents/:agent_id/allocate` | `agent_wallet::allocate_handler` | `src/api_server.rs:3466` | replaced:acn | agent wallet/earnings/collect/allocate; callers: scripts, web-ui |
| PUT | `/api/agents/:agent_id/auto-collect` | `agent_wallet::set_auto_collect_handler` | `src/api_server.rs:3470` | replaced:acn | agent wallet/earnings/collect/allocate; callers: web-ui |
| GET | `/projector` | `pages::projector_view` | `src/api_server.rs:3475` | ui |  |
| GET | `/agent/:agent_id/projector` | `pages::projector_view` | `src/api_server.rs:3476` | ui |  |
| GET | `/api/agents/:agent_id/projections` | `ontology::get_agent_projections` | `src/api_server.rs:3480` | CAP-01 Remember | ontology projections that feed `/projector` [I]; callers: web-ui (first pass: `tenant:fermi-console`) |
| GET | `/api/projections/bestiary` | `ontology::get_bestiary_projections` | `src/api_server.rs:3484` | CAP-01 Remember | ontology projections that feed `/projector` [I]; callers: web-ui (first pass: `tenant:fermi-console`) |
| GET | `/api/agents/:agent_id/projections/temporal` | `ontology::get_temporal_projections` | `src/api_server.rs:3488` | CAP-01 Remember | ontology projections that feed `/projector` [I]; callers: web-ui (first pass: `tenant:fermi-console`) |
| GET | `/api/beacons/nearby` | `beacons::nearby_beacons_handler` | `src/api_server.rs:3493` | tenant:rabble | no caller found (agent tool emits only `/asset` URLs) |
| GET | `/api/beacons/:beacon_id` | `beacons::get_beacon_handler` | `src/api_server.rs:3497` | tenant:rabble | no caller found (agent tool emits only `/asset` URLs); callers: agent-tools |
| GET | `/api/beacons/:beacon_id/asset` | `beacons::beacon_asset_handler` | `src/api_server.rs:3501` | tenant:rabble | callers: agent-tools |
| GET | `/api/grid-maps/:map_id` | `beacons::get_grid_map_handler` | `src/api_server.rs:3505` | tenant:rabble | no caller found (agent tool emits only `/asset` URLs) |
| GET | `/api/creatures` | `creatures::list_creatures_handler` | `src/api_server.rs:3510` | tenant:rabble | callers: kask, rabble |
| GET | `/api/creatures/:creature_id` | `creatures::get_creature_handler` | `src/api_server.rs:3514` | tenant:rabble | callers: rabble |
| GET | `/api/creatures/:creature_id/flights` | `creatures::creature_flights_handler` | `src/api_server.rs:3518` | tenant:rabble | callers: rabble |
| GET | `/api/creatures/:creature_id/versions` | `creatures::creature_versions_handler` | `src/api_server.rs:3522` | tenant:rabble | callers: rabble (first pass: `ops`) |
| GET | `/api/creatures/:creature_id/image` | `creatures::creature_image_handler` | `src/api_server.rs:3526` | tenant:rabble | callers: rabble |
| GET | `/api/creatures/:creature_id/animation/:layer_name` | `creatures::creature_animation_layer_handler` | `src/api_server.rs:3530` | tenant:rabble | callers: agent-tools, rabble |
| GET | `/api/creatures/:creature_id/animation-status` | `creatures::creature_animation_status_handler` | `src/api_server.rs:3534` | tenant:rabble | callers: rabble, rabble(compiled-bundle) |
| GET | `/api/creatures/:creature_id/flight-path/:flight_id` | `creatures::creature_flight_path_handler` | `src/api_server.rs:3538` | tenant:rabble | callers: rabble |
| GET | `/api/creatures/:creature_id/cognition` | `creatures::creature_cognition_handler` | `src/api_server.rs:3542` | tenant:rabble | callers: app/agent manifests |
| GET | `/api/swarms` | `creatures::list_swarms_handler` | `src/api_server.rs:3546` | tenant:rabble | callers: kask, rabble |
| GET | `/api/swarms/:swarm_id` | `creatures::get_swarm_handler` | `src/api_server.rs:3547` | tenant:rabble | callers: rabble |
| GET | `/api/rabble/:id/qr` | `qr_codes::rabble_qr_handler` | `src/api_server.rs:3552` | tenant:rabble | rabble event QR; no caller found |
| GET | `/user/:user_id` | `users::user_profile_view` | `src/api_server.rs:3557` | ui |  (first pass: `CAP-14`) |
| GET | `/api/users/:user_id` | `users::get_public_profile_handler` | `src/api_server.rs:3558` | CAP-14 Isolate tenants | user discovery/profile (accounts); callers: fermi-console, rabble, scripts, web-ui |
| GET | `/dashboard` | `pages::dashboard_view` | `src/api_server.rs:3563` | ui |  |
| GET | `/agents/new` | `pages::agent_create_view` | `src/api_server.rs:3564` | ui |  |
| GET | `/contracts` | `pages::contract_builder_view` | `src/api_server.rs:3568` | ui |  (first pass: `CAP-03`) |
| GET | `/workspace/:workspace_id` | `pages::workspace_view` | `src/api_server.rs:3569` | ui |  (first pass: `CAP-15`) |
| GET | `/auth/google` | `auth::auth_google` | `src/api_server.rs:3574` | CAP-14 Isolate tenants | authentication (OAuth, CLI login, API keys, SIWE); callers: adaptogen-lab, kask, web-ui |
| GET | `/auth/github` | `auth::auth_github` | `src/api_server.rs:3575` | CAP-14 Isolate tenants | authentication (OAuth, CLI login, API keys, SIWE); callers: adaptogen-lab, kask, web-ui |
| GET | `/auth/callback` | `auth::auth_callback` | `src/api_server.rs:3576` | CAP-14 Isolate tenants | authentication (OAuth, CLI login, API keys, SIWE); callers: scripts |
| POST | `/auth/logout` | `auth::auth_logout` | `src/api_server.rs:3577` | CAP-14 Isolate tenants | authentication (OAuth, CLI login, API keys, SIWE); callers: adaptogen-lab, kask, web-ui |
| GET | `/auth/cli` | `auth::auth_cli_start` | `src/api_server.rs:3579` | CAP-14 Isolate tenants | authentication (OAuth, CLI login, API keys, SIWE); callers: abw-cli, scripts |
| GET | `/auth/cli/finish` | `auth::auth_cli_finish` | `src/api_server.rs:3580` | CAP-14 Isolate tenants | authentication (OAuth, CLI login, API keys, SIWE); no caller found |
| POST | `/webhooks/stripe` | `billing::stripe_webhook_handler` | `src/api_server.rs:3582` | replaced:acn | no caller found |
| POST | `/auth/siwe/challenge` | `auth::siwe_challenge_handler` | `src/api_server.rs:3587` | CAP-14 Isolate tenants | authentication (OAuth, CLI login, API keys, SIWE); callers: web-ui |
| POST | `/auth/siwe/verify` | `auth::siwe_verify_handler` | `src/api_server.rs:3591` | CAP-14 Isolate tenants | authentication (OAuth, CLI login, API keys, SIWE); callers: web-ui |
| GET | `/profile` | `profile::profile_view` | `src/api_server.rs:3596` | ui |  (first pass: `CAP-14`) |
| GET | `/settings` | `pages::settings_view` | `src/api_server.rs:3597` | ui |  |
| GET | `/marketplace` | `pages::marketplace_view` | `src/api_server.rs:3598` | ui |  |
| GET | `/admin` | `pages::admin_view` | `src/api_server.rs:3599` | ui |  (first pass: `ops`) |
| GET | `/observatory` | `pages::observatory_view` | `src/api_server.rs:3601` | ui |  (first pass: `CAP-05`) |
| GET | `/ecology` | `pages::ecology_view` | `src/api_server.rs:3604` | ui |  |
| GET | `/rounds` | `pages::rounds_view` | `src/api_server.rs:3605` | ui |  |
| GET | `/bestiary` | `pages::bestiary_view` | `src/api_server.rs:3606` | ui |  (first pass: `CAP-01`) |
| GET | `/loops` | `pages::loops_view` | `src/api_server.rs:3607` | ui |  (first pass: `CAP-06`) |
| GET | `/gates` | `pages::loops_view` | `src/api_server.rs:3608` | ui |  (first pass: `CAP-06`) |
| GET | `/evaluators` | `pages::loops_view` | `src/api_server.rs:3614` | ui |  (first pass: `CAP-06`) |
| GET | `/specimen/:agent_name` | `pages::specimen_view` | `src/api_server.rs:3615` | ui |  (first pass: `tenant:rabble`) |
| GET | `/trace/:episode_id` | `pages::trace_view` | `src/api_server.rs:3616` | ui |  (first pass: `CAP-01`) |
| GET | `/grounding/:episode_id` | `pages::grounding_view` | `src/api_server.rs:3617` | ui |  (first pass: `CAP-09`) |
| GET | `/flow/:workspace_id` | `pages::flow_view` | `src/api_server.rs:3621` | ui |  (first pass: `CAP-15`) |
| GET | `/stream` | `pages::stream_view` | `src/api_server.rs:3622` | ui |  |
| GET | `/declarations` | `pages::declarations_view` | `src/api_server.rs:3623` | ui |  |
| GET | `/gate/:gate_id` | `pages::gate_view` | `src/api_server.rs:3624` | ui |  (first pass: `CAP-11`) |
| GET | `/api/ecology/overview` | `ecology::ecology_overview_handler` | `src/api_server.rs:3625` | CAP-05 Observe | ecology lens over the agent register (web-ui); callers: web-ui (first pass: `crud:misc`) |
| GET | `/api/ecology/specimens` | `ecology::ecology_specimens_handler` | `src/api_server.rs:3629` | CAP-05 Observe | ecology lens over the agent register (web-ui); callers: web-ui (first pass: `tenant:rabble`) |
| GET | `/observatory/hitl` | `pages::observatory_hitl_view` | `src/api_server.rs:3633` | ui |  (first pass: `CAP-05`) |
| GET | `/api/auth/me` | `auth::auth_me` | `src/api_server.rs:3655` | CAP-14 Isolate tenants | authentication (OAuth, CLI login, API keys, SIWE); callers: adaptogen-lab, fermi-console, kask, rabble, scripts, web-ui |
| GET | `/api/auth/api-keys` | `auth::list_api_keys` | `src/api_server.rs:3656` | CAP-14 Isolate tenants | authentication (OAuth, CLI login, API keys, SIWE); callers: web-ui |
| POST | `/api/auth/api-keys` | `auth::create_api_key` | `src/api_server.rs:3657` | CAP-14 Isolate tenants | authentication (OAuth, CLI login, API keys, SIWE); callers: web-ui |
| DELETE | `/api/auth/api-keys/:key_id` | `auth::revoke_api_key` | `src/api_server.rs:3658` | CAP-14 Isolate tenants | authentication (OAuth, CLI login, API keys, SIWE); callers: web-ui |
| POST | `/api/teams` | `teams::create_team_handler` | `src/api_server.rs:3663` | CAP-02 Compose teams | team = composition with mission, strategist and members (`src/handlers/teams.rs:13`); callers: fermi-console, kask, scripts, web-ui |
| GET | `/api/teams` | `teams::list_teams_handler` | `src/api_server.rs:3664` | CAP-02 Compose teams | team = composition with mission, strategist and members (`src/handlers/teams.rs:13`); callers: fermi-console, kask, scripts, web-ui |
| GET | `/api/teams/:team_id` | `teams::get_team_handler` | `src/api_server.rs:3665` | CAP-02 Compose teams | team = composition with mission, strategist and members (`src/handlers/teams.rs:13`); callers: fermi-console, kask, web-ui |
| DELETE | `/api/teams/:team_id` | `teams::delete_team_handler` | `src/api_server.rs:3669` | CAP-02 Compose teams | team = composition with mission, strategist and members (`src/handlers/teams.rs:13`); callers: fermi-console, kask, web-ui |
| POST | `/api/teams/:team_id/members` | `teams::add_member_handler` | `src/api_server.rs:3673` | CAP-02 Compose teams | team = composition with mission, strategist and members (`src/handlers/teams.rs:13`); callers: fermi-console, web-ui |
| GET | `/api/teams/:team_id/members` | `teams::list_members_handler` | `src/api_server.rs:3677` | CAP-02 Compose teams | team = composition with mission, strategist and members (`src/handlers/teams.rs:13`); callers: fermi-console, web-ui |
| DELETE | `/api/teams/:team_id/members/:member_id` | `teams::remove_member_handler` | `src/api_server.rs:3681` | CAP-02 Compose teams | team = composition with mission, strategist and members (`src/handlers/teams.rs:13`); callers: fermi-console, web-ui |
| PUT | `/api/teams/:team_id/members/:member_id` | `teams::update_member_role_handler` | `src/api_server.rs:3685` | CAP-02 Compose teams | team = composition with mission, strategist and members (`src/handlers/teams.rs:13`); callers: fermi-console, web-ui |
| PUT | `/api/teams/:team_id/members/:member_id/capabilities` | `teams::set_member_capabilities_handler` | `src/api_server.rs:3695` | CAP-02 Compose teams | team = composition with mission, strategist and members (`src/handlers/teams.rs:13`); callers: fermi-console |
| POST/GET | `/api/teams/:team_id/invites` | `invites::invite_to_team_handler, invites::list_team_invites_handler` | `src/api_server.rs:3703` | CAP-15 Share a workspace | invite lifecycle (team / by-token) [I]; callers: fermi-console (first pass: `CAP-14`) |
| GET | `/api/teams/:team_id/shared` | `collab::team_shared_handler` | `src/api_server.rs:3724` | tenant:fermi-console | Spec 26 collaboration read models; callers: fermi-console (first pass: `CAP-02`) |
| GET | `/api/teams/:team_id/activity` | `collab::team_activity_handler` | `src/api_server.rs:3728` | tenant:fermi-console | Spec 26 collaboration read models; callers: fermi-console (first pass: `CAP-02`) |
| GET | `/api/teams/:team_id/contributions` | `collab::team_contributions_handler` | `src/api_server.rs:3732` | tenant:fermi-console | Spec 26 collaboration read models; callers: fermi-console (first pass: `CAP-02`) |
| GET | `/api/teams/:team_id/ops` | `ops::team_ops_handler` | `src/api_server.rs:3740` | tenant:fermi-console | Spec 27 ops board for a team (console); callers: fermi-console (first pass: `CAP-02`) |
| GET | `/api/me/invites` | `invites::list_my_invites_handler` | `src/api_server.rs:3750` | CAP-15 Share a workspace | invite lifecycle (team / by-token) [I]; callers: fermi-console (first pass: `CAP-14`) |
| GET | `/api/me/invites/sent` | `invites::list_sent_invites_handler` | `src/api_server.rs:3754` | CAP-15 Share a workspace | invite lifecycle (team / by-token) [I]; callers: fermi-console (first pass: `CAP-14`) |
| POST | `/api/invites/:invite_id/decline` | `invites::decline_invite_handler` | `src/api_server.rs:3758` | CAP-15 Share a workspace | invite lifecycle (team / by-token) [I]; callers: fermi-console, web-ui (first pass: `CAP-14`) |
| DELETE | `/api/invites/:invite_id` | `invites::revoke_invite_handler` | `src/api_server.rs:3762` | CAP-15 Share a workspace | invite lifecycle (team / by-token) [I]; callers: fermi-console (first pass: `CAP-14`) |
| POST | `/api/invites/:invite_id/accept` | `invites::accept_invite_handler` | `src/api_server.rs:3771` | CAP-15 Share a workspace | invite lifecycle (team / by-token) [I]; callers: fermi-console (first pass: `CAP-14`) |
| POST | `/api/invites/by-token/:token/accept` | `invites::accept_invite_by_token_handler` | `src/api_server.rs:3775` | CAP-15 Share a workspace | invite lifecycle (team / by-token) [I]; callers: web-ui (first pass: `CAP-14`) |
| GET | `/api/ontology-templates` | `wizard::list_ontology_templates_handler` | `src/api_server.rs:3780` | CAP-01 Version an agent | authoring aids (LLM-generated ontology/prompt, guide, tags); callers: web-ui (first pass: `CAP-15`) |
| POST | `/api/agents/generate-ontology` | `wizard::generate_ontology_handler` | `src/api_server.rs:3784` | CAP-01 Version an agent | authoring aids (LLM-generated ontology/prompt, guide, tags); callers: web-ui |
| POST | `/api/agents/generate-prompt` | `wizard::generate_prompt_handler` | `src/api_server.rs:3788` | CAP-01 Version an agent | authoring aids (LLM-generated ontology/prompt, guide, tags); callers: web-ui |
| GET | `/api/agents/creation-guide` | `wizard::creation_guide_handler` | `src/api_server.rs:3792` | CAP-01 Version an agent | authoring aids (LLM-generated ontology/prompt, guide, tags); callers: web-ui |
| GET | `/api/tags/popular` | `wizard::popular_tags_handler` | `src/api_server.rs:3796` | CAP-01 Version an agent | authoring aids (LLM-generated ontology/prompt, guide, tags); callers: web-ui (first pass: `crud:misc`) |
| POST | `/api/contracts/compile` | `contracts::compile_handler` | `src/api_server.rs:3803` | CAP-03 Declare an expert contract | output-contract sketch compiler; same answer as the publish gate (`src/handlers/contracts.rs:1`); callers: scripts, web-ui |
| GET | `/api/contracts/tools` | `contracts::tool_names_handler` | `src/api_server.rs:3807` | CAP-03 Declare an expert contract | output-contract sketch compiler; same answer as the publish gate (`src/handlers/contracts.rs:1`); callers: scripts, web-ui |
| GET | `/api/contracts/types` | `contracts::types_handler` | `src/api_server.rs:3815` | CAP-03 Declare an expert contract | output-contract sketch compiler; same answer as the publish gate (`src/handlers/contracts.rs:1`); callers: scripts, web-ui |
| POST | `/api/contracts/suggest` | `contracts::suggest_handler` | `src/api_server.rs:3819` | CAP-03 Declare an expert contract | output-contract sketch compiler; same answer as the publish gate (`src/handlers/contracts.rs:1`); callers: scripts, web-ui |
| GET | `/api/contracts/decompile/:agent_id` | `contracts::decompile_handler` | `src/api_server.rs:3826` | CAP-03 Declare an expert contract | output-contract sketch compiler; same answer as the publish gate (`src/handlers/contracts.rs:1`); callers: scripts, web-ui |
| POST | `/api/contracts/tool-request` | `contracts::tool_request_handler` | `src/api_server.rs:3833` | CAP-01 Invoke a tool | request a tool be added to the allowlist [I]; callers: web-ui (first pass: `CAP-03`) |
| POST | `/api/agents` | `agents::create_agent_handler` | `src/api_server.rs:3838` | CAP-01 Version an agent | agent card CRUD / registry / versions; callers: fermi-console, kask, scripts, web-ui |
| POST | `/api/agents/import` | `agents::import_agent_handler` | `src/api_server.rs:3839` | CAP-01 Version an agent | agent card CRUD / registry / versions; callers: web-ui |
| GET | `/api/agents/mine` | `agents::list_my_agents_handler` | `src/api_server.rs:3843` | CAP-01 Version an agent | agent card CRUD / registry / versions; callers: scripts, web-ui |
| PUT | `/api/agents/:agent_id` | `agents::update_agent_handler` | `src/api_server.rs:3847` | CAP-01 Version an agent | agent card CRUD / registry / versions; callers: app/agent manifests, fermi-console, kask, scripts, web-ui |
| DELETE | `/api/agents/:agent_id` | `agents::delete_agent_handler` | `src/api_server.rs:3851` | CAP-01 Version an agent | agent card CRUD / registry / versions; callers: app/agent manifests, fermi-console, kask, scripts, web-ui |
| POST | `/api/agents/:agent_id/fork` | `lifecycle::fork_agent_handler` | `src/api_server.rs:3856` | CAP-01 Version an agent | publish/archive/restore/fork; callers: web-ui |
| PUT | `/api/agents/:agent_id/fork-pricing` | `lifecycle::update_fork_pricing_handler` | `src/api_server.rs:3860` | replaced:acn | fork pricing = royalties; callers: scripts, web-ui (first pass: `CAP-01`) |
| GET | `/api/agents/:agent_id/publish-checks` | `lifecycle::publish_checks_handler` | `src/api_server.rs:3864` | CAP-01 Version an agent | publish/archive/restore/fork; callers: web-ui |
| POST | `/api/agents/:agent_id/publish` | `lifecycle::publish_agent_handler` | `src/api_server.rs:3868` | CAP-01 Version an agent | publish/archive/restore/fork; callers: web-ui |
| POST | `/api/agents/:agent_id/archive` | `lifecycle::archive_agent_handler` | `src/api_server.rs:3872` | CAP-01 Version an agent | publish/archive/restore/fork; callers: scripts, web-ui |
| POST | `/api/agents/:agent_id/restore` | `lifecycle::restore_agent_handler` | `src/api_server.rs:3876` | CAP-01 Version an agent | publish/archive/restore/fork; callers: web-ui |
| POST | `/api/agents/:agent_id/versions/:version_num/restore` | `agents::restore_agent_version_handler` | `src/api_server.rs:3880` | CAP-01 Version an agent | agent card CRUD / registry / versions; callers: web-ui (first pass: `ops`) |
| POST | `/api/agents/:agent_id/avatar/generate` | `agents::generate_avatar` | `src/api_server.rs:3885` | CAP-01 Version an agent | avatar (cosmetic card field); callers: web-ui |
| POST | `/api/agents/:agent_id/embeddings/import` | `agents::import_embeddings_handler` | `src/api_server.rs:3890` | CAP-01 Remember | no caller found; counterpart of export (portability) |
| GET | `/api/agents/:agent_id/embeddings/stats` | `agents::embeddings_stats_handler` | `src/api_server.rs:3895` | CAP-01 Remember | embedding export/consent (memory portability); callers: web-ui |
| POST | `/api/agents/:agent_id/embeddings/export/consent` | `agents::embeddings_export_consent_handler` | `src/api_server.rs:3899` | CAP-01 Remember | embedding export/consent (memory portability); callers: web-ui |
| GET | `/api/agents/:agent_id/embeddings/export` | `agents::embeddings_export_handler` | `src/api_server.rs:3903` | CAP-01 Remember | embedding export/consent (memory portability); callers: web-ui |
| POST | `/api/agents/:agent_id/execute` | `execution::execute_agent_handler` | `src/api_server.rs:3908` | CAP-01 Run an episode | callers: agent-tools, app/agent manifests, fermi-console, kask, scripts, web-ui |
| POST | `/api/agents/:agent_id/execute/stream` | `execution_stream::execute_agent_stream_handler` | `src/api_server.rs:3913` | CAP-01 Run an episode | callers: fermi-console, kask |
| GET | `/api/agents/:agent_id/dreaming/budget` | `consolidation::get_dreaming_budget` | `src/api_server.rs:3918` | CAP-06 Loop 1 Agent learning | consolidation (dreaming) and its budget; callers: web-ui |
| PUT | `/api/agents/:agent_id/dreaming/budget` | `consolidation::set_dreaming_budget` | `src/api_server.rs:3922` | CAP-06 Loop 1 Agent learning | consolidation (dreaming) and its budget; callers: web-ui |
| POST | `/api/agents/:agent_id/dreaming/topup` | `consolidation::topup_dreaming_budget_handler` | `src/api_server.rs:3926` | replaced:acn | credit top-up of dreaming budget; callers: scripts, web-ui (first pass: `CAP-06`) |
| POST | `/api/agents/:agent_id/consolidate` | `consolidation::consolidate_agent_handler` | `src/api_server.rs:3931` | CAP-06 Loop 1 Agent learning | consolidation (dreaming) and its budget; callers: web-ui |
| GET | `/api/agents/:agent_id/dreaming` | `dreaming_maturity::agent_dreaming_maturity_handler` | `src/api_server.rs:3935` | CAP-06 Loop 1 Agent learning | did dreaming produce knowledge; callers: web-ui |
| GET | `/api/agents/:agent_id/evolution` | `evolution::agent_evolution_handler` | `src/api_server.rs:3941` | CAP-06 Learn and improve | earned progression badge across dimensions [I]; callers: web-ui (first pass: `CAP-01`) |
| GET | `/api/agents/:agent_id/consolidation/jobs/:job_id` | `consolidation::get_consolidation_job_handler` | `src/api_server.rs:3945` | CAP-06 Loop 1 Agent learning | consolidation (dreaming) and its budget; callers: web-ui |
| POST | `/api/apps` | `apps::create_app_handler` | `src/api_server.rs:3955` | CAP-15 Register an app | callers: abw-cli, kask, other-crates, scripts, web-ui |
| PUT | `/api/apps/:slug` | `apps::update_app_handler_full` | `src/api_server.rs:3956` | CAP-15 Register an app | callers: abw-cli, kask, scripts, web-ui |
| POST | `/api/apps/:slug/workspaces` | `apps::spawn_workspace_handler` | `src/api_server.rs:3960` | CAP-15 Spawn a workspace | callers: abw-cli, adaptogen-lab, fermi-console, kask, scripts, web-ui |
| GET | `/api/apps/:slug/workspaces` | `apps::list_app_workspaces_handler` | `src/api_server.rs:3964` | CAP-15 Spawn a workspace | callers: abw-cli, adaptogen-lab, fermi-console, kask, scripts, web-ui |
| POST | `/api/apps/:slug/workspaces/batch` | `apps::batch_spawn_workspaces_handler` | `src/api_server.rs:3968` | CAP-15 Spawn a workspace | batch; callers: world_cup/weather scripts only; callers: scripts |
| POST | `/api/apps/:slug/publish` | `apps::publish_app_handler` | `src/api_server.rs:3972` | CAP-15 Set visibility | callers: abw-cli, kask, web-ui |
| POST | `/api/apps/:slug/archive` | `apps::archive_app_handler` | `src/api_server.rs:3976` | CAP-15 Set visibility | callers: kask, scripts, web-ui |
| POST | `/api/apps/:slug/sync-auto-hire` | `apps::sync_auto_hire_handler` | `src/api_server.rs:3984` | CAP-02 Hire an agent | re-sync auto-hire; caller: `scripts/world_cup/respawn_aligned.py` only; callers: scripts |
| POST | `/api/simops/cascade` | `simops::cascade_handler` | `src/api_server.rs:3989` | tenant:simops | deterministic SimOps kernels (no LLM); callers: kask, other-crates |
| POST | `/api/simops/project` | `simops::project_handler` | `src/api_server.rs:3994` | tenant:simops | deterministic SimOps kernels (no LLM); callers: kask, other-crates |
| POST | `/api/simops/cascade/suggest-bindings` | `workspace::actions::suggest_bindings_handler` | `src/api_server.rs:3999` | tenant:simops | cascade binding suggestions; callers: kask |
| POST | `/api/simops/dynamics` | `simops::dynamics_handler` | `src/api_server.rs:4004` | tenant:simops | deterministic SimOps kernels (no LLM); callers: kask |
| GET | `/api/simops/dynamics/models` | `simops::dynamics_list_handler` | `src/api_server.rs:4008` | tenant:simops | deterministic SimOps kernels (no LLM); callers: kask |
| POST | `/api/simops/rheology` | `simops::rheology_handler` | `src/api_server.rs:4013` | tenant:simops | deterministic SimOps kernels (no LLM); callers: kask |
| GET | `/api/simops/rheology/models` | `simops::rheology_list_handler` | `src/api_server.rs:4017` | tenant:simops | deterministic SimOps kernels (no LLM); callers: kask |
| POST | `/api/bayesops/fit_marginal` | `bayesops::fit_marginal_handler` | `src/api_server.rs:4027` | CAP-06 Loop 5.B Parameter correction (BayesOps) | stateless kernel over HTTP; no HTTP caller (only `tests/bayesops_http_shapes.rs`); MCP uses the library directly (`src/bin/agent-mcp-server.rs:59`); callers: tests (first pass: `tenant:fermi-console`) |
| POST | `/api/bayesops/fit_conditional` | `bayesops::fit_conditional_handler` | `src/api_server.rs:4031` | CAP-06 Loop 5.B Parameter correction (BayesOps) | stateless kernel over HTTP; no HTTP caller (only `tests/bayesops_http_shapes.rs`); MCP uses the library directly (`src/bin/agent-mcp-server.rs:59`); callers: tests (first pass: `tenant:fermi-console`) |
| POST | `/api/bayesops/predict` | `bayesops::predict_handler` | `src/api_server.rs:4035` | CAP-06 Loop 5.B Parameter correction (BayesOps) | stateless kernel over HTTP; no HTTP caller (only `tests/bayesops_http_shapes.rs`); MCP uses the library directly (`src/bin/agent-mcp-server.rs:59`); no caller found (first pass: `tenant:fermi-console`) |
| POST | `/api/bayesops/input_sensitivity` | `bayesops::input_sensitivity_handler` | `src/api_server.rs:4039` | CAP-06 Loop 5.B Parameter correction (BayesOps) | stateless kernel over HTTP; no HTTP caller (only `tests/bayesops_http_shapes.rs`); MCP uses the library directly (`src/bin/agent-mcp-server.rs:59`); no caller found (first pass: `tenant:fermi-console`) |
| POST | `/api/bayesops/compare_scenarios` | `bayesops::compare_scenarios_handler` | `src/api_server.rs:4043` | CAP-06 Loop 5.B Parameter correction (BayesOps) | stateless kernel over HTTP; no HTTP caller (only `tests/bayesops_http_shapes.rs`); MCP uses the library directly (`src/bin/agent-mcp-server.rs:59`); no caller found (first pass: `tenant:simops`) |
| POST | `/api/bayesops/prob_exceeds` | `bayesops::prob_exceeds_handler` | `src/api_server.rs:4047` | CAP-06 Loop 5.B Parameter correction (BayesOps) | stateless kernel over HTTP; no HTTP caller (only `tests/bayesops_http_shapes.rs`); MCP uses the library directly (`src/bin/agent-mcp-server.rs:59`); no caller found (first pass: `tenant:fermi-console`) |
| POST | `/api/bayesops/optimise_for_target` | `bayesops::optimise_for_target_handler` | `src/api_server.rs:4051` | CAP-06 Loop 5.B Parameter correction (BayesOps) | stateless kernel over HTTP; no HTTP caller (only `tests/bayesops_http_shapes.rs`); MCP uses the library directly (`src/bin/agent-mcp-server.rs:59`); no caller found (first pass: `tenant:fermi-console`) |
| GET | `/api/bayesops/posteriors` | `bayesops::list_posteriors_handler` | `src/api_server.rs:4055` | CAP-06 Loop 5.B Parameter correction (BayesOps) | stateless kernel over HTTP; no HTTP caller (only `tests/bayesops_http_shapes.rs`); MCP uses the library directly (`src/bin/agent-mcp-server.rs:59`); callers: tests (first pass: `tenant:fermi-console`) |
| DELETE | `/api/bayesops/posteriors/:id` | `bayesops::evict_posterior_handler` | `src/api_server.rs:4059` | CAP-06 Loop 5.B Parameter correction (BayesOps) | stateless kernel over HTTP; no HTTP caller (only `tests/bayesops_http_shapes.rs`); MCP uses the library directly (`src/bin/agent-mcp-server.rs:59`); callers: tests (first pass: `tenant:fermi-console`) |
| GET | `/api/workspaces/:workspace_id/bayesops/state` | `bayesops::workspace_bayesops_state_handler` | `src/api_server.rs:4066` | CAP-06 Loop 5.B Parameter correction (BayesOps) | callers: app/agent manifests, fermi-console (first pass: `tenant:fermi-console`) |
| POST | `/api/bayesops/pending/:pending_id/accept` | `bayesops::accept_pending_handler` | `src/api_server.rs:4070` | CAP-06 Loop 5.B Parameter correction (BayesOps) | held-for-review refits (human gate); callers: fermi-console (first pass: `tenant:fermi-console`) |
| POST | `/api/bayesops/pending/:pending_id/reject` | `bayesops::reject_pending_handler` | `src/api_server.rs:4074` | CAP-06 Loop 5.B Parameter correction (BayesOps) | held-for-review refits (human gate); callers: fermi-console (first pass: `tenant:fermi-console`) |
| POST | `/api/workspaces/:workspace_id/refit` | `workspace::refit::refit_workspace_handler` | `src/api_server.rs:4083` | CAP-06 Loop 5.B Parameter correction (BayesOps) | refit hook; callers: app/agent manifests, fermi-console, scripts (first pass: `CAP-15`) |
| GET | `/api/workspaces` | `workspace::list_workspaces_handler` | `src/api_server.rs:4088` | CAP-15 Spawn a workspace | workspace read; callers: kask, web-ui |
| GET | `/api/workspaces/:workspace_id` | `workspace::get_workspace_handler` | `src/api_server.rs:4092` | CAP-15 Spawn a workspace | workspace read; callers: adaptogen-lab, kask, scripts, web-ui |
| GET | `/api/workspaces/:workspace_id/agents` | `workspace::list_workspace_agents_handler` | `src/api_server.rs:4096` | CAP-15 Spawn a workspace | workspace read; callers: adaptogen-lab, kask, web-ui |
| POST | `/api/workspaces/:workspace_id/agents` | `workspace::create_workspace_agent_handler` | `src/api_server.rs:4100` | CAP-02 Hire an agent | callers: adaptogen-lab, kask, web-ui (first pass: `CAP-15`) |
| POST | `/api/workspaces/:workspace_id/budget` | `workspace::fund_workspace_handler` | `src/api_server.rs:4104` | replaced:acn | workspace budget/funding (CAP-12 Fund a spawn → ACN); callers: adaptogen-lab, web-ui (first pass: `CAP-15`) |
| GET | `/api/workspaces/:workspace_id/budget` | `workspace::workspace_budget_handler` | `src/api_server.rs:4110` | replaced:acn | workspace budget/funding (CAP-12 Fund a spawn → ACN); callers: adaptogen-lab, web-ui (first pass: `CAP-15`) |
| POST | `/api/workspaces/:workspace_id/cascade` | `simops::workspace_cascade_handler` | `src/api_server.rs:4115` | tenant:simops | deterministic SimOps kernels (no LLM); callers: kask |
| GET | `/api/simops/workspaces/:workspace_id/process-spacetime` | `simops_benchmark::process_spacetime_handler` | `src/api_server.rs:4120` | tenant:simops | projection commitment + spacetime; no caller found |
| GET/PUT | `/api/simops/workspaces/:workspace_id/sample-config` | `simops_benchmark::get_sample_config_handler, simops_benchmark::put_sample_config_handler` | `src/api_server.rs:4124` | tenant:simops | projection commitment + spacetime; no caller found |
| GET | `/api/workspaces/:workspace_id/actions` | `workspace::actions::list_actions_handler` | `src/api_server.rs:4132` | CAP-15 Read and write state | generic app action protocol; callers: abw-cli, adaptogen-lab |
| GET | `/api/workspaces/:workspace_id/actions/pending` | `workspace::actions::list_pending_handler` | `src/api_server.rs:4136` | CAP-10 Sign off a claim | pending actions awaiting a human; callers: abw-cli (first pass: `CAP-15`) |
| POST | `/api/workspaces/:workspace_id/actions/mutate_document` | `workspace::actions::mutate_document_handler` | `src/api_server.rs:4140` | CAP-15 Read and write state | generic app action protocol; callers: abw-cli, app/agent manifests, kask |
| POST | `/api/workspaces/:workspace_id/actions/fork_state` | `workspace::actions::fork_state_handler` | `src/api_server.rs:4144` | CAP-15 Read and write state | generic app action protocol; callers: abw-cli, app/agent manifests, kask |
| POST | `/api/workspaces/:workspace_id/actions/compare` | `workspace::actions::compare_handler` | `src/api_server.rs:4148` | CAP-15 Read and write state | generic app action protocol; callers: app/agent manifests, kask |
| POST | `/api/workspaces/:workspace_id/actions/invoke_member` | `workspace::actions::invoke_member_handler` | `src/api_server.rs:4152` | CAP-15 Invoke agents | callers: app/agent manifests, kask |
| POST | `/api/workspaces/:workspace_id/actions/annotate_schema` | `workspace::actions::annotate_schema_handler` | `src/api_server.rs:4156` | CAP-15 Read and write state | generic app action protocol; callers: app/agent manifests, kask |
| POST | `/api/workspaces/:workspace_id/actions/annotate` | `workspace::actions::annotate_handler` | `src/api_server.rs:4160` | CAP-15 Read and write state | generic app action protocol; callers: abw-cli, app/agent manifests, kask |
| POST | `/api/workspaces/:workspace_id/actions/log_observation` | `workspace::actions::log_observation_handler` | `src/api_server.rs:4165` | CAP-15 Read and write state | declared by `apps/kask_wild.json`; callers: app/agent manifests (first pass: `CAP-05`) |
| POST | `/api/workspaces/:workspace_id/actions/render_lens` | `workspace::lens_actions::render_lens_handler` | `src/api_server.rs:4170` | tenant:dpp | Adaptogen Lab / Regulatory Lens action; callers: adaptogen-lab, app/agent manifests (first pass: `CAP-15`) |
| POST | `/api/workspaces/:workspace_id/actions/compare_lenses` | `workspace::lens_actions::compare_lenses_handler` | `src/api_server.rs:4174` | tenant:dpp | Adaptogen Lab / Regulatory Lens action; callers: adaptogen-lab, app/agent manifests (first pass: `CAP-15`) |
| POST | `/api/workspaces/:workspace_id/actions/evaluate_claims` | `workspace::claim_evaluation::evaluate_claims_handler` | `src/api_server.rs:4182` | tenant:dpp | Adaptogen Lab / Regulatory Lens action; callers: adaptogen-lab, app/agent manifests (first pass: `CAP-05`) |
| POST | `/api/workspaces/:workspace_id/actions/price_bom` | `workspace::bom_pricing::price_bom_handler` | `src/api_server.rs:4188` | tenant:dpp | Adaptogen Lab / Regulatory Lens action; callers: adaptogen-lab, app/agent manifests |
| POST | `/api/workspaces/:workspace_id/actions/flag_divergence` | `workspace::lens_actions::flag_divergence_handler` | `src/api_server.rs:4192` | tenant:dpp | Adaptogen Lab / Regulatory Lens action; callers: adaptogen-lab, app/agent manifests (first pass: `CAP-15`) |
| POST | `/api/workspaces/:workspace_id/actions/calculate_carbon` | `workspace::carbon::calculate_carbon_handler` | `src/api_server.rs:4202` | tenant:dpp | Adaptogen Lab / Regulatory Lens action; callers: adaptogen-lab, app/agent manifests |
| POST | `/api/workspaces/:workspace_id/actions/identify` | `wild::identify_action_handler` | `src/api_server.rs:4212` | tenant:wild | photo identification action (`apps/kask_wild.json`); callers: app/agent manifests (first pass: `CAP-15`) |
| POST | `/api/workspaces/:workspace_id/actions/:action_id/accept` | `workspace::actions::accept_action_handler` | `src/api_server.rs:4216` | CAP-10 Sign off a claim | human accepts/rejects a proposed action; callers: abw-cli, kask (first pass: `CAP-15`) |
| POST | `/api/workspaces/:workspace_id/actions/:action_id/reject` | `workspace::actions::reject_action_handler` | `src/api_server.rs:4220` | CAP-10 Sign off a claim | human accepts/rejects a proposed action; callers: abw-cli, kask (first pass: `CAP-15`) |
| POST | `/api/workspaces/:workspace_id/actions/migrate_parallelism_to_twin` | `workspace::actions::migrate_parallelism_to_twin_handler` | `src/api_server.rs:4224` | tenant:simops | SimOps data migration action; caller `kask/kask-sim-client.js:10251`; callers: app/agent manifests, kask (first pass: `CAP-15`) |
| GET | `/api/workspaces/:workspace_id/annotations` | `workspace::actions::list_annotations_handler` | `src/api_server.rs:4228` | CAP-15 Read and write state | generic app action protocol; callers: kask |
| DELETE | `/api/workspaces/:workspace_id/annotations/:annotation_id` | `workspace::actions::resolve_annotation_handler` | `src/api_server.rs:4232` | CAP-15 Read and write state | generic app action protocol; callers: kask |
| GET | `/api/workspaces/:workspace_id/outputs` | `workspace::outputs::list_outputs_handler` | `src/api_server.rs:4237` | CAP-15 Read and write state | typed outputs + cross-workspace dependencies; callers: fermi-console |
| PUT/GET/DELETE | `/api/workspaces/:workspace_id/outputs/:key` | `workspace::outputs::set_output_handler, workspace::outputs::get_output_handler, workspace::outputs::delete_output_handler` | `src/api_server.rs:4241` | CAP-15 Read and write state | typed outputs + cross-workspace dependencies; callers: fermi-console, scripts |
| GET/POST | `/api/workspaces/:workspace_id/dependencies` | `workspace::outputs::list_dependencies_handler, workspace::outputs::add_dependency_handler` | `src/api_server.rs:4248` | CAP-15 Read and write state | typed outputs + cross-workspace dependencies; callers: fermi-console |
| DELETE | `/api/workspaces/:workspace_id/dependencies/:upstream_id` | `workspace::outputs::remove_dependency_handler` | `src/api_server.rs:4253` | CAP-15 Read and write state | typed outputs + cross-workspace dependencies; no caller found |
| POST | `/api/workspaces/:workspace_id/resolve` | `workspace::resolution::resolve_workspace_handler` | `src/api_server.rs:4261` | CAP-13 Observe an outcome | generalised workspace resolution; no client caller found; no caller found (first pass: `CAP-15`) |
| POST | `/api/workspaces/:workspace_id/messages` | `workspace::post_workspace_message_handler` | `src/api_server.rs:4266` | CAP-15 Invoke agents | workspace messages; callers: abw-cli, fermi-console, kask, scripts, web-ui |
| GET | `/api/workspaces/:workspace_id/messages` | `workspace::get_workspace_messages_handler` | `src/api_server.rs:4270` | CAP-15 Invoke agents | workspace messages; callers: abw-cli, fermi-console, kask, scripts, web-ui |
| GET | `/api/workspaces/:workspace_id/messages/poll` | `workspace::poll_workspace_messages_handler` | `src/api_server.rs:4274` | CAP-15 Invoke agents | workspace messages; callers: adaptogen-lab |
| GET | `/api/workspaces/:workspace_id/messages/stream` | `workspace::workspace_messages_stream_handler` | `src/api_server.rs:4279` | CAP-15 Invoke agents | workspace messages; callers: kask |
| GET | `/api/workspaces/:workspace_id/workflow` | `workspace::get_workspace_workflow_handler` | `src/api_server.rs:4283` | CAP-15 Spawn a workspace | workspace read; callers: web-ui |
| POST | `/api/workspaces/:workspace_id/fork-to-app` | `workspace::fork_workspace_to_app_draft_handler` | `src/api_server.rs:4289` | CAP-15 Register an app | callers: scripts, web-ui |
| POST | `/api/workspaces/:workspace_id/hire` | `workspace::hire_agent_handler` | `src/api_server.rs:4294` | CAP-02 Hire an agent | callers: kask, web-ui |
| POST | `/api/workspaces/:workspace_id/add` | `workspace::add_agent_handler` | `src/api_server.rs:4298` | CAP-02 Hire an agent | callers: kask, web-ui (first pass: `CAP-15`) |
| DELETE | `/api/workspaces/:workspace_id/agents/:agent_id` | `workspace::remove_workspace_agent_handler` | `src/api_server.rs:4302` | CAP-02 Hire an agent | callers: kask, web-ui (first pass: `CAP-15`) |
| GET | `/api/xaman/sessions` | `xaman::list_xaman_sessions_handler` | `src/api_server.rs:4307` | CAP-15 Be discovered | Xaman Ek navigator sessions; callers: scripts, web-ui (first pass: `CAP-14`) |
| POST | `/api/xaman/sessions` | `xaman::create_xaman_session_handler` | `src/api_server.rs:4311` | CAP-15 Be discovered | Xaman Ek navigator sessions; callers: scripts, web-ui (first pass: `CAP-14`) |
| GET | `/api/xaman/sessions/:id` | `xaman::get_xaman_session_handler` | `src/api_server.rs:4315` | CAP-15 Be discovered | Xaman Ek navigator sessions; callers: web-ui (first pass: `CAP-14`) |
| POST | `/api/xaman/sessions/:id/message` | `xaman::xaman_session_message_handler` | `src/api_server.rs:4319` | CAP-15 Be discovered | Xaman Ek navigator sessions; callers: web-ui (first pass: `CAP-14`) |
| POST | `/api/xaman/sessions/:id/complete` | `xaman::complete_xaman_session_handler` | `src/api_server.rs:4323` | CAP-15 Be discovered | Xaman Ek navigator sessions; no caller found (first pass: `CAP-14`) |
| POST | `/api/xaman/sessions/:id/create-app` | `xaman::create_app_from_session_handler` | `src/api_server.rs:4327` | CAP-15 Register an app | Xaman Ek session → app draft; callers: scripts, web-ui (first pass: `CAP-14`) |
| DELETE | `/api/xaman/sessions/:id` | `xaman::abandon_xaman_session_handler` | `src/api_server.rs:4331` | CAP-15 Be discovered | Xaman Ek navigator sessions; callers: web-ui (first pass: `CAP-14`) |
| POST | `/api/workspaces/:workspace_id/composition/identity` | `workspace::set_composition_identity_handler` | `src/api_server.rs:4336` | CAP-02 Compose teams | composition identity; callers: app/agent manifests, web-ui |
| GET | `/api/workspaces/:workspace_id/composition/versions` | `composition::list_composition_versions_handler` | `src/api_server.rs:4341` | CAP-02 Change composition | owner accepts/rejects composition versions; callers: app/agent manifests, web-ui (first pass: `ops`) |
| POST | `/api/workspaces/:workspace_id/composition/versions/:version_id/accept` | `composition::accept_composition_version_handler` | `src/api_server.rs:4345` | CAP-02 Change composition | owner accepts/rejects composition versions; callers: web-ui (first pass: `ops`) |
| POST | `/api/workspaces/:workspace_id/composition/versions/:version_id/reject` | `composition::reject_composition_version_handler` | `src/api_server.rs:4349` | CAP-02 Change composition | owner accepts/rejects composition versions; callers: web-ui (first pass: `ops`) |
| POST | `/api/workspaces/:workspace_id/composition/propose` | `composition::propose_composition_version_handler` | `src/api_server.rs:4353` | CAP-02 Change composition | strategist-side propose; no caller found |
| POST | `/api/workspaces/:workspace_id/composition/dream` | `composition::composition_dream_handler` | `src/api_server.rs:4357` | CAP-02 Change composition | Loop 3.B dream → proposal; callers: web-ui (first pass: `CAP-06`) |
| GET | `/api/wallet` | `wallet::get_wallet_handler` | `src/api_server.rs:4362` | replaced:acn | callers: fermi-console, rabble, scripts, web-ui |
| GET | `/api/wallet/transactions` | `wallet::get_transactions_handler` | `src/api_server.rs:4363` | replaced:acn | callers: rabble, web-ui |
| POST | `/api/wallet/transfer` | `wallet::transfer_credits_handler` | `src/api_server.rs:4367` | replaced:acn | callers: rabble |
| GET | `/api/billing/tiers` | `billing::billing_tiers_handler` | `src/api_server.rs:4372` | replaced:acn | callers: rabble, web-ui |
| POST | `/api/billing/checkout` | `billing::billing_checkout_handler` | `src/api_server.rs:4376` | replaced:acn | callers: rabble, web-ui |
| POST | `/api/billing/dev-topup` | `billing::billing_dev_topup_handler` | `src/api_server.rs:4380` | replaced:acn | callers: rabble |
| GET | `/api/users/search` | `users::search_users_handler` | `src/api_server.rs:4385` | CAP-14 Isolate tenants | user discovery/profile (accounts); callers: rabble, web-ui |
| GET | `/api/users/collaborators` | `users::get_collaborators_handler` | `src/api_server.rs:4389` | CAP-14 Isolate tenants | user discovery/profile (accounts); callers: web-ui |
| GET | `/api/users/lookup` | `users::lookup_user_by_email_handler` | `src/api_server.rs:4396` | CAP-14 Isolate tenants | user discovery/profile (accounts); callers: fermi-console |
| GET | `/api/me/workspace` | `rabble_workspace::get_personal_workspace_handler` | `src/api_server.rs:4401` | tenant:rabble | personal 'menagerie' workspace; no caller found |
| GET | `/api/profile` | `profile::get_profile_handler` | `src/api_server.rs:4406` | CAP-14 Isolate tenants | user profile (accounts); callers: rabble, web-ui |
| PUT | `/api/profile` | `profile::update_profile_handler` | `src/api_server.rs:4407` | CAP-14 Isolate tenants | user profile (accounts); callers: rabble, web-ui |
| GET | `/api/notifications` | `profile::list_notifications_handler` | `src/api_server.rs:4412` | CAP-10 Involve humans | notifications [I]; callers: rabble, scripts, web-ui (first pass: `CAP-14`) |
| PUT | `/api/notifications/:id/read` | `profile::mark_notification_read_handler` | `src/api_server.rs:4416` | CAP-10 Involve humans | notifications [I]; callers: rabble (first pass: `CAP-14`) |
| PUT | `/api/notifications/read-all` | `profile::mark_all_notifications_read_handler` | `src/api_server.rs:4420` | CAP-10 Involve humans | notifications [I]; callers: rabble, web-ui (first pass: `CAP-14`) |
| POST | `/api/secrets` | `profile::create_secret_handler` | `src/api_server.rs:4425` | CAP-14 Keep secrets | callers: scripts, web-ui |
| GET | `/api/secrets` | `profile::list_secrets_handler` | `src/api_server.rs:4429` | CAP-14 Keep secrets | callers: scripts, web-ui |
| GET | `/api/secrets/audit` | `profile::secret_audit_handler` | `src/api_server.rs:4430` | CAP-14 Keep secrets | callers: web-ui |
| DELETE | `/api/secrets/:name` | `profile::delete_secret_handler` | `src/api_server.rs:4434` | CAP-14 Keep secrets | callers: web-ui |
| POST | `/api/workspaces/:workspace_id/coherence/evaluate` | `workspace::evaluate_coherence_handler` | `src/api_server.rs:4439` | CAP-04 Measure coherence | callers: web-ui |
| GET | `/api/workspaces/:workspace_id/coherence` | `workspace::get_coherence_handler` | `src/api_server.rs:4443` | CAP-04 Measure coherence | callers: app/agent manifests, web-ui |
| GET | `/api/workspaces/:workspace_id/coherence/history` | `workspace::get_coherence_history_handler` | `src/api_server.rs:4447` | CAP-04 Measure coherence | callers: web-ui |
| GET | `/api/workspaces/:workspace_id/ontology` | `workspace::get_workspace_ontology_handler` | `src/api_server.rs:4452` | CAP-01 Remember | workspace ontology; no caller found (first pass: `CAP-15`) |
| GET | `/api/workspaces/:workspace_id/dpp/qr` | `qr_codes::dpp_passport_qr_handler` | `src/api_server.rs:4460` | tenant:dpp | passport QR image; no caller found |
| GET/POST/DELETE | `/api/workspaces/:workspace_id/dpp/publish` | `workspace::dpp_publish::publication_status_handler, workspace::dpp_publish::publish_dpp_handler, workspace::dpp_publish::unpublish_dpp_handler` | `src/api_server.rs:4464` | CAP-14 Publish an artifact | released product passport (frozen snapshot); DPP is the named example in CAP-14; callers: adaptogen-lab (first pass: `tenant:dpp`) |
| GET | `/api/workspaces/:workspace_id/files` | `workspace::list_workspace_files_handler` | `src/api_server.rs:4470` | CAP-15 Read and write state | workspace files; callers: abw-cli, app/agent manifests, kask, web-ui |
| GET | `/api/workspaces/:workspace_id/files/*path` | `workspace::read_workspace_file_handler` | `src/api_server.rs:4474` | CAP-15 Read and write state | workspace files; callers: abw-cli, adaptogen-lab, kask, web-ui |
| PUT | `/api/workspaces/:workspace_id/files/*path` | `workspace::write_workspace_file_handler` | `src/api_server.rs:4478` | CAP-15 Read and write state | workspace files; callers: abw-cli, adaptogen-lab, kask, web-ui |
| GET | `/api/workspaces/:workspace_id/files-raw/*path` | `workspace::read_workspace_file_raw_handler` | `src/api_server.rs:4482` | CAP-15 Read and write state | workspace files; callers: rabble |
| POST | `/api/workspaces/:workspace_id/upload` | `workspace::upload_workspace_file_handler` | `src/api_server.rs:4487` | CAP-15 Read and write state | workspace files; callers: rabble, web-ui |
| GET | `/api/workspaces/:workspace_id/git/log` | `workspace::workspace_git_log_handler` | `src/api_server.rs:4492` | CAP-15 Read and write state | workspace git history; callers: kask, web-ui |
| GET | `/api/workspaces/:workspace_id/git/diff` | `workspace::workspace_git_diff_handler` | `src/api_server.rs:4496` | CAP-15 Read and write state | workspace git history; callers: kask, web-ui |
| GET | `/api/notebooks` | `notebooks::list_notebooks_handler` | `src/api_server.rs:4501` | drop? | persisted notebooks: no caller in console, kask, web-ui or tools; stateless `/api/fpl/execute` is the live path (first pass: `CAP-15`) |
| POST | `/api/notebooks` | `notebooks::create_notebook_handler` | `src/api_server.rs:4505` | drop? | persisted notebooks: no caller in console, kask, web-ui or tools; stateless `/api/fpl/execute` is the live path (first pass: `CAP-15`) |
| GET | `/api/notebooks/:notebook_id` | `notebooks::get_notebook_handler` | `src/api_server.rs:4509` | drop? | persisted notebooks: no caller in console, kask, web-ui or tools; stateless `/api/fpl/execute` is the live path (first pass: `CAP-15`) |
| PUT | `/api/notebooks/:notebook_id` | `notebooks::update_notebook_handler` | `src/api_server.rs:4513` | drop? | persisted notebooks: no caller in console, kask, web-ui or tools; stateless `/api/fpl/execute` is the live path (first pass: `CAP-15`) |
| DELETE | `/api/notebooks/:notebook_id` | `notebooks::delete_notebook_handler` | `src/api_server.rs:4517` | drop? | persisted notebooks: no caller in console, kask, web-ui or tools; stateless `/api/fpl/execute` is the live path (first pass: `CAP-15`) |
| POST | `/api/notebooks/:notebook_id/execute` | `notebooks::execute_notebook_handler` | `src/api_server.rs:4521` | drop? | persisted notebooks: no caller in console, kask, web-ui or tools; stateless `/api/fpl/execute` is the live path (first pass: `CAP-01`) |
| POST | `/api/fpl/execute` | `notebooks::fpl_execute_handler` | `src/api_server.rs:4526` | CAP-06 FPL Monte Carlo executor | stateless FPL run; callers: console, kask; callers: kask (first pass: `tenant:fermi-console`) |
| GET | `/api/fpl/health` | `notebooks::fpl_health_handler` | `src/api_server.rs:4530` | ops | FPL health; no caller found |
| POST | `/api/forecasts` | `forecasts::create_forecast_handler` | `src/api_server.rs:4535` | tenant:fermi-console | core primitive underneath: CAP-07 Bind a commitment; callers: fermi-console, kask, scripts |
| GET | `/api/forecasts` | `forecasts::list_forecasts_handler` | `src/api_server.rs:4539` | tenant:fermi-console | callers: fermi-console, kask, scripts |
| GET | `/api/forecasts/my-stats` | `forecasts::my_stats_handler` | `src/api_server.rs:4543` | tenant:fermi-console | callers: fermi-console |
| GET | `/api/forecasts/:forecast_id` | `forecasts::get_forecast_handler` | `src/api_server.rs:4547` | tenant:fermi-console | callers: fermi-console, scripts |
| PUT | `/api/forecasts/:forecast_id` | `forecasts::update_forecast_handler` | `src/api_server.rs:4551` | tenant:fermi-console | callers: fermi-console, scripts |
| DELETE | `/api/forecasts/:forecast_id` | `forecasts::delete_forecast_handler` | `src/api_server.rs:4555` | tenant:fermi-console | callers: fermi-console, scripts |
| POST | `/api/forecasts/:forecast_id/resolve` | `forecasts::resolve_forecast_handler` | `src/api_server.rs:4559` | tenant:fermi-console | core primitive underneath: CAP-13 Observe an outcome; callers: fermi-console, scripts |
| POST | `/api/forecasts/:forecast_id/void` | `forecasts::void_forecast_handler` | `src/api_server.rs:4563` | tenant:fermi-console | core primitive underneath: CAP-13 Observe an outcome; callers: fermi-console |
| POST | `/api/forecasts/:forecast_id/update-probability` | `forecasts::update_probability_handler` | `src/api_server.rs:4567` | tenant:fermi-console | core primitive underneath: CAP-07 Bind a commitment; callers: fermi-console |
| GET/PUT | `/api/forecasts/:forecast_id/schedules` | `forecasts::list_forecast_schedules_handler, forecasts::upsert_forecast_schedule_handler` | `src/api_server.rs:4572` | tenant:fermi-console | callers: fermi-console |
| DELETE | `/api/forecasts/:forecast_id/schedules/:schedule_id` | `forecasts::delete_forecast_schedule_handler` | `src/api_server.rs:4577` | tenant:fermi-console | callers: fermi-console |
| POST | `/api/forecasts/:forecast_id/schedules/:schedule_id/run` | `forecasts::record_schedule_run_handler` | `src/api_server.rs:4581` | tenant:fermi-console | callers: fermi-console |
| GET | `/api/forecasts/:forecast_id/spacetime` | `forecast_benchmark::forecast_spacetime_handler` | `src/api_server.rs:4586` | tenant:fermi-console | no caller found; `timeline` appears to have replaced it [I] |
| GET | `/api/forecasts/:forecast_id/timeline` | `forecast_benchmark::forecast_timeline_handler` | `src/api_server.rs:4594` | tenant:fermi-console | callers: fermi-console |
| GET | `/api/forecasts/:forecast_id/cascade-provenance` | `forecast_benchmark::forecast_cascade_provenance_handler` | `src/api_server.rs:4604` | tenant:fermi-console | callers: fermi-console |
| POST | `/api/forecasts/:forecast_id/commit` | `forecast_benchmark::commit_forecast_handler` | `src/api_server.rs:4608` | CAP-07 Bind a commitment | tamper-evident commitment anchor; HTTP route has no caller (anchoring also runs internally) (first pass: `tenant:fermi-console`) |
| POST | `/api/benchmark/anchor-sweep` | `forecast_benchmark::anchor_sweep_handler` | `src/api_server.rs:4612` | CAP-07 Bind a commitment | tamper-evident commitment anchor; HTTP route has no caller (anchoring also runs internally) (first pass: `tenant:fermi-console`) |
| GET/POST | `/api/forecasts/:forecast_id/shares` | `shares::list_forecast_shares_handler, shares::create_forecast_share_handler` | `src/api_server.rs:4621` | tenant:fermi-console | per-target forecast/portfolio shares; callers: fermi-console |
| DELETE | `/api/forecasts/:forecast_id/shares/:share_id` | `shares::revoke_forecast_share_handler` | `src/api_server.rs:4626` | tenant:fermi-console | per-target forecast/portfolio shares; callers: fermi-console |
| GET | `/api/forecasts/:forecast_id/access` | `collab::forecast_access_handler` | `src/api_server.rs:4637` | tenant:fermi-console | Spec 26 collaboration read models; callers: fermi-console |
| GET | `/api/forecasts/:forecast_id/activity` | `collab::forecast_activity_handler` | `src/api_server.rs:4641` | tenant:fermi-console | Spec 26 collaboration read models; callers: fermi-console |
| GET | `/api/forecasts/:forecast_id/history` | `forecast_git::forecast_history_handler` | `src/api_server.rs:4661` | tenant:fermi-console | callers: fermi-console |
| GET | `/api/forecasts/:forecast_id/history/:sha` | `forecast_git::forecast_diff_handler` | `src/api_server.rs:4665` | tenant:fermi-console | callers: fermi-console |
| POST | `/api/forecasts/:forecast_id/revert` | `forecast_git::forecast_revert_handler` | `src/api_server.rs:4669` | tenant:fermi-console | callers: fermi-console |
| GET/POST | `/api/forecasts/:forecast_id/annotations` | `annotations::list_annotations_handler, annotations::create_annotation_handler` | `src/api_server.rs:4689` | tenant:fermi-console | callers: fermi-console |
| POST | `/api/forecasts/:forecast_id/annotations/:annotation_id/resolve` | `annotations::resolve_annotation_handler` | `src/api_server.rs:4694` | tenant:fermi-console | callers: fermi-console |
| DELETE | `/api/forecasts/:forecast_id/annotations/:annotation_id` | `annotations::delete_annotation_handler` | `src/api_server.rs:4698` | tenant:fermi-console | callers: fermi-console |
| POST/GET | `/api/forecasts/:forecast_id/invites` | `invites::invite_to_forecast_handler, invites::list_forecast_invites_handler` | `src/api_server.rs:4705` | tenant:fermi-console | forecast/portfolio invites; callers: fermi-console |
| POST/GET | `/api/forecast-relationships` | `relationships::legacy::create_relationship_handler, relationships::legacy::list_relationships_handler` | `src/api_server.rs:4711` | tenant:fermi-console | mig-150 model, self-described legacy; superseded by groups but console still calls it; callers: fermi-console, scripts |
| DELETE | `/api/forecast-relationships/:rel_id` | `relationships::legacy::delete_relationship_handler` | `src/api_server.rs:4716` | tenant:fermi-console | mig-150 model, self-described legacy; superseded by groups but console still calls it; callers: fermi-console |
| POST | `/api/forecast-relationships/:rel_id/propagate` | `relationships::legacy::propagate_relationship_handler` | `src/api_server.rs:4720` | tenant:fermi-console | mig-150 model, self-described legacy; superseded by groups but console still calls it; callers: fermi-console |
| POST/GET | `/api/relationship-groups` | `relationships::groups::create_group_handler, relationships::groups::list_groups_handler` | `src/api_server.rs:4725` | tenant:fermi-console | relationship groups (Spec 25); callers: fermi-console (first pass: `CAP-06`) |
| GET/PATCH/DELETE | `/api/relationship-groups/:group_id` | `relationships::groups::get_group_handler, relationships::groups::patch_group_handler, relationships::groups::delete_group_handler` | `src/api_server.rs:4730` | tenant:fermi-console | relationship groups (Spec 25); callers: fermi-console (first pass: `CAP-06`) |
| GET | `/api/relationship-groups/:group_id/members` | `relationships::groups::get_group_members_handler` | `src/api_server.rs:4736` | tenant:fermi-console | relationship groups (Spec 25); no caller found (first pass: `CAP-06`) |
| POST | `/api/relationship-groups/:group_id/propagate` | `relationships::groups::preview_group_propagation_handler` | `src/api_server.rs:4745` | tenant:fermi-console | relationship groups (Spec 25); callers: fermi-console (first pass: `CAP-10`) |
| GET/PUT | `/api/forecasts/:forecast_id/groups` | `relationships::membership::get_forecast_groups_handler, relationships::membership::set_forecast_groups_handler` | `src/api_server.rs:4757` | tenant:fermi-console | relationship groups (Spec 25); callers: fermi-console |
| POST/DELETE | `/api/forecasts/:forecast_id/groups/:group_id` | `relationships::membership::add_forecast_to_group_handler, relationships::membership::remove_forecast_from_group_handler` | `src/api_server.rs:4762` | tenant:fermi-console | relationship groups (Spec 25); callers: fermi-console |
| GET | `/api/pending-cascades` | `pending_cascades::list_pending_cascades_handler` | `src/api_server.rs:4768` | tenant:fermi-console | operator-gated cascade queue (CAP-10 pattern); callers: fermi-console (first pass: `CAP-06`) |
| POST | `/api/pending-cascades/:cascade_id/apply` | `relationships::apply::apply_pending_cascade_handler` | `src/api_server.rs:4772` | tenant:fermi-console | operator-gated cascade (CAP-10 pattern); callers: fermi-console (first pass: `CAP-06`) |
| POST | `/api/pending-cascades/:cascade_id/dismiss` | `pending_cascades::dismiss_pending_cascade_handler` | `src/api_server.rs:4776` | tenant:fermi-console | operator-gated cascade queue (CAP-10 pattern); callers: fermi-console (first pass: `CAP-06`) |
| POST | `/api/pending-cascades/:cascade_id/undo` | `relationships::undo::undo_pending_cascade_handler` | `src/api_server.rs:4780` | tenant:fermi-console | operator-gated cascade (CAP-10 pattern); no caller found (first pass: `CAP-06`) |
| POST | `/api/pending-cascades/requeue` | `relationships::requeue::requeue_cascade_handler` | `src/api_server.rs:4784` | tenant:fermi-console | Loop 2 cascade queue; no caller found (first pass: `CAP-06`) |
| GET | `/api/forecasts/:forecast_id/cascade-history` | `pending_cascades::cascade_history_handler` | `src/api_server.rs:4788` | tenant:fermi-console | no caller found |
| POST | `/api/portfolios` | `forecasts::create_portfolio_handler` | `src/api_server.rs:4793` | tenant:fermi-console | callers: fermi-console, scripts |
| GET | `/api/portfolios` | `forecasts::list_portfolios_handler` | `src/api_server.rs:4797` | tenant:fermi-console | callers: fermi-console, scripts |
| GET | `/api/portfolios/:portfolio_id/stats` | `forecasts::portfolio_stats_handler` | `src/api_server.rs:4801` | tenant:fermi-console | callers: fermi-console |
| DELETE/PATCH | `/api/portfolios/:portfolio_id` | `forecasts::delete_portfolio_handler, forecasts::patch_portfolio_handler` | `src/api_server.rs:4805` | tenant:fermi-console | callers: fermi-console, scripts |
| GET/POST | `/api/portfolios/:portfolio_id/forecasts` | `forecasts::list_portfolio_forecasts_handler, forecasts::add_forecast_to_portfolio_handler` | `src/api_server.rs:4810` | tenant:fermi-console | callers: fermi-console, scripts |
| DELETE | `/api/portfolios/:portfolio_id/forecasts/:forecast_id` | `forecasts::remove_forecast_from_portfolio_handler` | `src/api_server.rs:4815` | tenant:fermi-console | callers: fermi-console |
| GET/POST | `/api/portfolios/:portfolio_id/shares` | `shares::list_portfolio_shares_handler, shares::create_portfolio_share_handler` | `src/api_server.rs:4820` | tenant:fermi-console | per-target forecast/portfolio shares; callers: fermi-console (first pass: `CAP-14`) |
| DELETE | `/api/portfolios/:portfolio_id/shares/:share_id` | `shares::revoke_portfolio_share_handler` | `src/api_server.rs:4825` | tenant:fermi-console | per-target forecast/portfolio shares; callers: fermi-console (first pass: `CAP-14`) |
| GET | `/api/portfolios/:portfolio_id/access` | `collab::portfolio_access_handler` | `src/api_server.rs:4833` | tenant:fermi-console | Spec 26 collaboration read models; callers: fermi-console (first pass: `CAP-14`) |
| GET | `/api/portfolios/:portfolio_id/activity` | `collab::portfolio_activity_handler` | `src/api_server.rs:4837` | tenant:fermi-console | Spec 26 collaboration read models; callers: fermi-console (first pass: `CAP-05`) |
| POST/GET | `/api/portfolios/:portfolio_id/invites` | `invites::invite_to_portfolio_handler, invites::list_portfolio_invites_handler` | `src/api_server.rs:4842` | tenant:fermi-console | forecast/portfolio invites; callers: fermi-console (first pass: `CAP-14`) |
| GET | `/api/leaderboard` | `forecasts::leaderboard_handler` | `src/api_server.rs:4848` | tenant:fermi-console | callers: fermi-console |
| POST | `/api/shares` | `teams::share_object_handler` | `src/api_server.rs:4853` | drop? | generic share that 'performs zero authorization' (`src/handlers/shares.rs:13`); superseded by per-target shares; no caller (first pass: `CAP-02`) |
| DELETE | `/api/shares/:share_id` | `teams::revoke_share_handler` | `src/api_server.rs:4854` | drop? | generic share that 'performs zero authorization' (`src/handlers/shares.rs:13`); superseded by per-target shares; no caller (first pass: `CAP-02`) |
| POST | `/api/polymarket/search` | `polymarket::search_handler` | `src/api_server.rs:4859` | tenant:fermi-console | callers: fermi-console |
| POST | `/api/polymarket/snapshot` | `polymarket::snapshot_handler` | `src/api_server.rs:4863` | tenant:fermi-console | callers: fermi-console |
| POST | `/api/polymarket/link` | `polymarket::link_handler` | `src/api_server.rs:4867` | tenant:fermi-console | callers: fermi-console, scripts |
| POST | `/api/polymarket/import` | `polymarket::import_handler` | `src/api_server.rs:4871` | tenant:fermi-console | no caller found |
| GET | `/api/polymarket/observations` | `polymarket::observations_handler` | `src/api_server.rs:4875` | tenant:fermi-console | no caller found |
| POST | `/api/polymarket/check-resolutions` | `polymarket::check_resolutions_handler` | `src/api_server.rs:4879` | tenant:fermi-console | oracle; core primitive CAP-13 Observe an outcome; callers: fermi-console |
| POST | `/api/admin/impersonate` | `impersonation::start_impersonation_handler` | `src/api_server.rs:4889` | ops | admin read-only 'view as user'; callers: web-ui |
| POST | `/api/admin/impersonate/end` | `impersonation::end_impersonation_handler` | `src/api_server.rs:4893` | ops | admin read-only 'view as user'; callers: web-ui |
| GET | `/api/admin/impersonate/sessions` | `impersonation::list_impersonation_sessions_handler` | `src/api_server.rs:4897` | ops | admin read-only 'view as user'; no caller found |
| GET | `/api/me/impersonation-history` | `impersonation::my_impersonation_history_handler` | `src/api_server.rs:4903` | CAP-14 Isolate tenants | user sees who viewed as them; no caller found |
| GET | `/api/admin/economics/platform` | `economics::platform_economics_handler` | `src/api_server.rs:4910` | replaced:acn | callers: web-ui |
| GET | `/api/admin/stats` | `admin::admin_stats_handler` | `src/api_server.rs:4915` | ops | admin; callers: rabble, web-ui |
| GET | `/api/admin/users` | `admin::admin_list_users_handler` | `src/api_server.rs:4919` | ops | admin; callers: rabble, web-ui |
| POST | `/api/admin/users/:user_id/grant` | `admin::admin_grant_credits_handler` | `src/api_server.rs:4923` | replaced:acn | credit grants; callers: rabble, web-ui |
| POST | `/api/admin/workspaces/:workspace_id/grant` | `admin::admin_grant_workspace_credits_handler` | `src/api_server.rs:4927` | replaced:acn | credit grants; callers: web-ui |
| GET | `/api/admin/agents` | `admin::admin_list_agents_handler` | `src/api_server.rs:4931` | ops | admin; callers: web-ui |
| PUT | `/api/admin/agents/:agent_id/flag` | `admin::admin_flag_agent_handler` | `src/api_server.rs:4935` | ops | admin; callers: web-ui |
| GET/POST | `/api/admin/agents/legacy-slugs` | `admin::admin_legacy_agent_slugs_handler` | `src/api_server.rs:4945` | drop? | one-off v0.10.20 rename of pre-slug agent names; caller `abw-cli admin` only; callers: abw-cli (first pass: `ops`) |
| GET/POST | `/api/admin/agents/cleanup-test-cruft` | `admin::admin_cleanup_test_cruft_handler` | `src/api_server.rs:4957` | drop? | one-off v0.10.25 delete of test-fixture agents; caller `abw-cli admin` only; callers: abw-cli (first pass: `ops`) |
| GET | `/api/orchestras` | `orchestras::list_orchestras_handler` | `src/api_server.rs:4966` | CAP-02 Hire an agent | orchestra registry with admin-gated joins (hire rule) [I]; callers: scripts, web-ui (first pass: `CAP-04`) |
| GET | `/api/orchestras/:name/members` | `orchestras::list_orchestra_members_handler` | `src/api_server.rs:4970` | CAP-02 Hire an agent | orchestra registry with admin-gated joins (hire rule) [I]; callers: fermi-console (first pass: `CAP-04`) |
| DELETE | `/api/orchestras/:name/members/:agent_id` | `orchestras::revoke_orchestra_member_handler` | `src/api_server.rs:4976` | CAP-02 Hire an agent | orchestra registry with admin-gated joins (hire rule) [I]; no caller found (first pass: `CAP-04`) |
| GET/POST | `/api/orchestras/:name/requests` | `orchestras::list_orchestra_requests_handler, orchestras::submit_orchestra_request_handler` | `src/api_server.rs:4980` | CAP-02 Hire an agent | orchestra registry with admin-gated joins (hire rule) [I]; callers: web-ui (first pass: `CAP-04`) |
| POST | `/api/orchestras/:name/requests/:request_id/approve` | `orchestras::approve_orchestra_request_handler` | `src/api_server.rs:4985` | CAP-02 Hire an agent | orchestra registry with admin-gated joins (hire rule) [I]; callers: web-ui (first pass: `CAP-04`) |
| POST | `/api/orchestras/:name/requests/:request_id/reject` | `orchestras::reject_orchestra_request_handler` | `src/api_server.rs:4989` | CAP-02 Hire an agent | orchestra registry with admin-gated joins (hire rule) [I]; callers: web-ui (first pass: `CAP-04`) |
| POST | `/api/orchestras/:name/requests/:request_id/withdraw` | `orchestras::withdraw_orchestra_request_handler` | `src/api_server.rs:4993` | CAP-02 Hire an agent | orchestra registry with admin-gated joins (hire rule) [I]; callers: web-ui (first pass: `CAP-04`) |
| GET | `/api/agents/:agent_id/orchestras` | `orchestras::agent_orchestras_handler` | `src/api_server.rs:4997` | CAP-02 Hire an agent | orchestra registry with admin-gated joins (hire rule) [I]; callers: web-ui (first pass: `CAP-04`) |
| GET | `/api/orchestras/:name/manager-effect` | `orchestras::orchestra_manager_effect_handler` | `src/api_server.rs:5004` | CAP-02 Hire an agent | orchestra registry with admin-gated joins (hire rule) [I]; callers: web-ui (first pass: `CAP-04`) |
| GET | `/api/admin/apps` | `admin::admin_list_apps_handler` | `src/api_server.rs:5012` | ops | admin; callers: web-ui |
| PUT | `/api/admin/apps/:slug/visibility` | `admin::admin_set_app_visibility_handler` | `src/api_server.rs:5016` | ops | admin; callers: web-ui |
| GET | `/api/admin/agent-ownership-audit` | `admin::admin_agent_ownership_audit_handler` | `src/api_server.rs:5020` | ops | admin; callers: web-ui |
| POST | `/api/admin/agent-ownership-reassign` | `admin::admin_agent_ownership_reassign_handler` | `src/api_server.rs:5024` | ops | admin; callers: web-ui |
| GET | `/api/admin/rbac/orphans` | `admin_rbac::admin_rbac_orphans_handler` | `src/api_server.rs:5031` | ops | RBAC orphan repair; no caller found |
| POST | `/api/admin/rbac/reassign` | `admin_rbac::admin_rbac_reassign_handler` | `src/api_server.rs:5035` | ops | RBAC orphan repair; no caller found |
| POST | `/api/admin/rbac/heal` | `admin_rbac::admin_rbac_heal_handler` | `src/api_server.rs:5039` | ops | RBAC orphan repair; no caller found |
| GET | `/api/rbac/self-check` | `rbac_self_check::rbac_self_check_handler` | `src/api_server.rs:5048` | ops | RBAC self-diagnostic; callers: fermi-console, other-crates (first pass: `CAP-14`) |
| POST | `/api/admin/wipe-fermi-forecasts` | `admin::admin_wipe_fermi_forecasts_handler` | `src/api_server.rs:5056` | drop? | 'Spec 23 demo cleanup — one-shot wipe' (comment at registration); caller `scripts/world_cup/respawn_aligned.py` only; callers: scripts (first pass: `ops`) |
| POST | `/api/admin/recompose-mutex-groups` | `admin::admin_recompose_mutex_groups_handler` | `src/api_server.rs:5065` | drop? | 'one-shot admin recompose' after a bug fix; no caller found (first pass: `ops`) |
| GET | `/api/admin/schema-health` | `admin::admin_schema_health_handler` | `src/api_server.rs:5074` | ops | admin; callers: scripts, web-ui |
| GET | `/api/admin/liveness` | `admin::admin_liveness_handler` | `src/api_server.rs:5081` | ops | admin; no caller found |
| GET/POST | `/api/admin/waitlist` | `admin::admin_list_waitlist_handler, admin::admin_add_waitlist_handler` | `src/api_server.rs:5085` | ops | admin; callers: web-ui |
| POST | `/api/admin/waitlist/invite` | `admin::admin_invite_waitlist_handler` | `src/api_server.rs:5090` | ops | admin; callers: web-ui |
| DELETE | `/api/admin/waitlist/:entry_id` | `admin::admin_delete_waitlist_handler` | `src/api_server.rs:5094` | ops | admin; callers: web-ui |
| GET | `/api/admin/creatures` | `admin::admin_list_creatures_handler` | `src/api_server.rs:5099` | tenant:rabble | Rabble moderation (admin); callers: rabble (first pass: `ops`) |
| PUT | `/api/admin/creatures/:creature_id/flag` | `admin::admin_flag_creature_handler` | `src/api_server.rs:5103` | tenant:rabble | Rabble moderation (admin); callers: rabble (first pass: `ops`) |
| GET | `/api/admin/swarms` | `admin::admin_list_swarms_handler` | `src/api_server.rs:5107` | tenant:rabble | Rabble moderation (admin); callers: rabble (first pass: `ops`) |
| PUT | `/api/admin/swarms/:swarm_id/status` | `admin::admin_update_swarm_status_handler` | `src/api_server.rs:5111` | tenant:rabble | Rabble moderation (admin); callers: rabble (first pass: `ops`) |
| POST | `/api/marketplace/match` | `marketplace::marketplace_match_handler` | `src/api_server.rs:5116` | tenant:shopping | embedding marketplace; paid matching moves to ACN; callers: web-ui |
| GET | `/api/marketplace/listings` | `marketplace::list_marketplace_listings_handler` | `src/api_server.rs:5120` | tenant:shopping | embedding marketplace; paid matching moves to ACN; callers: web-ui |
| POST | `/api/marketplace/listings` | `marketplace::create_marketplace_listing_handler` | `src/api_server.rs:5124` | tenant:shopping | embedding marketplace; paid matching moves to ACN; callers: web-ui |
| GET | `/api/marketplace/history` | `marketplace::marketplace_history_handler` | `src/api_server.rs:5128` | tenant:shopping | embedding marketplace; paid matching moves to ACN; callers: web-ui |
| GET | `/api/shopping/profile` | `marketplace::get_shopping_profiles_handler` | `src/api_server.rs:5132` | tenant:shopping | embedding marketplace; paid matching moves to ACN; no caller found (first pass: `CAP-14`) |
| PUT | `/api/shopping/profile/:listing_id/listing` | `marketplace::update_listing_handler` | `src/api_server.rs:5136` | tenant:shopping | embedding marketplace; paid matching moves to ACN; no caller found (first pass: `CAP-14`) |
| GET/POST | `/api/collections` | `creatures::list_collections_handler, creatures::create_collection_handler` | `src/api_server.rs:5141` | tenant:rabble | callers: rabble |
| PUT | `/api/collections/:collection_id` | `creatures::update_collection_handler` | `src/api_server.rs:5146` | tenant:rabble | callers: rabble |
| POST | `/api/flights` | `creatures::record_flight_handler` | `src/api_server.rs:5150` | tenant:rabble | callers: rabble |
| PUT | `/api/flights/:flight_id/end` | `creatures::end_flight_handler` | `src/api_server.rs:5154` | tenant:rabble | callers: rabble |
| POST | `/api/flights/:flight_id/telemetry` | `creatures::append_telemetry_handler` | `src/api_server.rs:5158` | tenant:rabble | callers: rabble |
| GET | `/api/flights/:flight_id/export` | `creatures::export_flight_handler` | `src/api_server.rs:5162` | tenant:rabble | callers: rabble |
| POST | `/api/flights/import` | `creatures::import_flight_handler` | `src/api_server.rs:5166` | tenant:rabble | callers: rabble |
| GET | `/api/creatures/:creature_id/stream` | `streams::creature_stream_handler` | `src/api_server.rs:5171` | tenant:rabble | callers: rabble |
| POST | `/api/creatures/:creature_id/perch` | `creatures::perch_handler` | `src/api_server.rs:5176` | tenant:rabble | callers: rabble |
| POST | `/api/creatures/:creature_id/host` | `creatures::host_rabble_handler` | `src/api_server.rs:5180` | tenant:rabble | callers: rabble |
| POST | `/api/creatures/:creature_id/fly` | `creatures::fly_handler` | `src/api_server.rs:5184` | tenant:rabble | callers: rabble |
| POST/DELETE | `/api/creatures/:creature_id/tether` | `creatures::tether_handler, creatures::untether_handler` | `src/api_server.rs:5189` | tenant:rabble | callers: rabble |
| POST | `/api/creatures/:creature_id/enemy-sensor` | `creatures::enemy_sensor_handler` | `src/api_server.rs:5194` | tenant:rabble | callers: rabble |
| POST | `/api/creatures/:creature_id/genome-profiler` | `creatures::genome_profiler_handler` | `src/api_server.rs:5199` | tenant:rabble | callers: rabble |
| POST | `/api/creatures/:creature_id/prey-locator` | `creatures::prey_locator_handler` | `src/api_server.rs:5204` | tenant:rabble | callers: rabble |
| POST | `/api/creatures/:creature_id/forage` | `creatures::forage_handler` | `src/api_server.rs:5209` | tenant:rabble | callers: rabble |
| GET/POST | `/api/creatures/:creature_id/goals` | `creatures::list_goals_handler, creatures::create_goal_handler` | `src/api_server.rs:5214` | tenant:rabble | callers: rabble |
| PATCH | `/api/creatures/:creature_id/goals/:goal_id` | `creatures::update_goal_handler` | `src/api_server.rs:5219` | tenant:rabble | no caller found |
| POST | `/api/creatures/:creature_id/telemetry` | `creatures::push_telemetry_handler` | `src/api_server.rs:5223` | tenant:rabble | callers: rabble |
| GET | `/api/creatures/:creature_id/track` | `creatures::get_track_handler` | `src/api_server.rs:5227` | tenant:rabble | callers: rabble |
| POST | `/api/creatures/mint` | `creatures::mint_creature_handler` | `src/api_server.rs:5232` | tenant:rabble | callers: rabble |
| GET | `/api/my/rabbles` | `creatures::my_rabbles_handler` | `src/api_server.rs:5236` | tenant:rabble | callers: rabble |
| POST | `/api/swarms/create` | `creatures::create_swarm_handler` | `src/api_server.rs:5240` | tenant:rabble | only the compiled bundle `static/rabble/main.dart.js` calls it; Rabble source does not [I]; callers: rabble(compiled-bundle) |
| POST | `/api/swarms/:swarm_id/join` | `creatures::join_swarm_handler` | `src/api_server.rs:5244` | tenant:rabble | callers: rabble |
| PATCH | `/api/swarms/:swarm_id` | `creatures::update_swarm_handler` | `src/api_server.rs:5248` | tenant:rabble | callers: rabble |
| GET | `/api/feed` | `creatures::feed_handler` | `src/api_server.rs:5253` | tenant:rabble | callers: kask, rabble |
| GET | `/api/dashboard/my-rabbles` | `dashboard::my_rabbles_handler` | `src/api_server.rs:5256` | drop? | duplicate of `/api/my/rabbles` (which Rabble calls, `api_client.dart:415`); only `scripts/patch_dashboard.py` names it; callers: scripts (first pass: `tenant:rabble`) |
| GET | `/api/dashboard/nearby` | `dashboard::nearby_rabbles_handler` | `src/api_server.rs:5260` | tenant:rabble | callers: rabble, scripts |
| GET | `/api/dashboard/creatures` | `dashboard::creatures_handler` | `src/api_server.rs:5264` | tenant:rabble | callers: rabble, scripts |
| GET | `/api/dashboard/boundary-violations` | `dashboard::boundary_violations_handler` | `src/api_server.rs:5268` | drop? | no caller in Rabble or anywhere (first pass: `crud:misc`) |
| GET | `/api/dashboard/nearby-creatures` | `dashboard::nearby_creatures_handler` | `src/api_server.rs:5272` | tenant:rabble | callers: rabble |
| GET/POST | `/api/locations` | `dashboard::list_locations_handler, dashboard::save_location_handler` | `src/api_server.rs:5277` | tenant:rabble | callers: rabble (first pass: `crud:misc`) |
| PATCH/DELETE | `/api/locations/:id` | `dashboard::update_location_handler, dashboard::delete_location_handler` | `src/api_server.rs:5282` | tenant:rabble | callers: rabble (first pass: `crud:misc`) |
| POST/DELETE | `/api/creatures/:creature_id/favourite` | `creatures::favourite_creature_handler, creatures::unfavourite_creature_handler` | `src/api_server.rs:5287` | tenant:rabble | callers: rabble |
| POST | `/api/creatures/:creature_id/generate-art` | `creatures::generate_art_handler` | `src/api_server.rs:5293` | tenant:rabble | callers: rabble |
| POST | `/api/creatures/generate-art-batch` | `creatures::generate_art_batch_handler` | `src/api_server.rs:5297` | tenant:rabble | no caller found (batch/admin) |
| PUT | `/api/creatures/:creature_id/sosa-opt-in` | `creatures::sosa_opt_in_handler` | `src/api_server.rs:5301` | tenant:rabble | callers: rabble |
| PUT | `/api/creatures/:creature_id/update` | `creatures::update_creature_handler` | `src/api_server.rs:5306` | tenant:rabble | callers: rabble |
| PUT | `/api/creatures/:creature_id/status` | `creatures::update_creature_status_handler` | `src/api_server.rs:5310` | tenant:rabble | callers: rabble |
| PUT | `/api/creatures/:creature_id/presence` | `creatures::update_creature_presence_handler` | `src/api_server.rs:5314` | tenant:rabble | callers: rabble |
| POST | `/api/creatures/:creature_id/transfer` | `creatures::transfer_creature_handler` | `src/api_server.rs:5318` | tenant:rabble | callers: rabble |
| POST | `/api/creatures/:creature_id/animate` | `creatures::animate_creature_handler` | `src/api_server.rs:5323` | tenant:rabble | callers: rabble |
| GET | `/api/creatures/:creature_id/level` | `creatures::creature_level_handler` | `src/api_server.rs:5328` | tenant:rabble | callers: rabble |
| POST | `/api/creatures/:creature_id/dream` | `creatures::creature_dream_handler` | `src/api_server.rs:5332` | tenant:rabble | callers: rabble |
| GET | `/api/creatures/:creature_id/activity` | `creatures::creature_activity_handler` | `src/api_server.rs:5336` | tenant:rabble | callers: rabble |
| PUT | `/api/creatures/:creature_id/visibility` | `creatures::update_creature_visibility_handler` | `src/api_server.rs:5341` | tenant:rabble | callers: rabble |
| GET | `/api/flights/visible` | `creatures::list_visible_flights_handler` | `src/api_server.rs:5346` | tenant:rabble | callers: rabble, rabble(compiled-bundle) |
| GET/POST | `/api/creatures/:creature_id/devices` | `creatures::list_devices_handler, creatures::pair_device_handler` | `src/api_server.rs:5351` | tenant:rabble | callers: rabble |
| PUT/DELETE | `/api/devices/:device_id` | `creatures::update_device_handler, creatures::unpair_device_handler` | `src/api_server.rs:5356` | tenant:rabble | callers: rabble |
| POST | `/api/devices/:device_id/location` | `creatures::report_device_location_handler` | `src/api_server.rs:5361` | tenant:rabble | callers: rabble |
| GET/POST | `/api/contacts` | `social::list_contacts_handler, social::add_contact_handler` | `src/api_server.rs:5366` | tenant:rabble | also web-ui; callers: rabble, web-ui (first pass: `crud:misc`) |
| PUT/DELETE | `/api/contacts/:contact_id` | `social::update_contact_handler, social::remove_contact_handler` | `src/api_server.rs:5371` | tenant:rabble | also web-ui; callers: rabble, web-ui (first pass: `crud:misc`) |
| POST | `/api/creature-friendships` | `social::send_friendship_request_handler` | `src/api_server.rs:5377` | tenant:rabble | callers: rabble, scripts (first pass: `crud:misc`) |
| GET | `/api/creature-friendships/pending` | `social::pending_friendships_handler` | `src/api_server.rs:5381` | tenant:rabble | callers: rabble, scripts (first pass: `crud:misc`) |
| POST | `/api/creature-friendships/:id/accept` | `social::accept_friendship_handler` | `src/api_server.rs:5385` | tenant:rabble | callers: rabble, scripts (first pass: `crud:misc`) |
| POST | `/api/creature-friendships/:id/decline` | `social::decline_friendship_handler` | `src/api_server.rs:5389` | tenant:rabble | callers: rabble, scripts (first pass: `crud:misc`) |
| DELETE | `/api/creature-friendships/:id` | `social::remove_friendship_handler` | `src/api_server.rs:5393` | tenant:rabble | callers: rabble, scripts (first pass: `crud:misc`) |
| GET | `/api/creatures/:creature_id/friends` | `social::list_creature_friends_handler` | `src/api_server.rs:5397` | tenant:rabble | callers: rabble, scripts |
| POST | `/api/creature-invites` | `social::send_creature_invite_handler` | `src/api_server.rs:5402` | tenant:rabble | callers: rabble, scripts (first pass: `CAP-14`) |
| GET | `/api/creature-invites/pending` | `social::list_pending_creature_invites_handler` | `src/api_server.rs:5406` | tenant:rabble | callers: rabble, scripts (first pass: `CAP-14`) |
| POST | `/api/creature-invites/:id/accept` | `social::accept_creature_invite_handler` | `src/api_server.rs:5410` | tenant:rabble | callers: rabble, scripts (first pass: `CAP-14`) |
| POST | `/api/creature-invites/:id/decline` | `social::decline_creature_invite_handler` | `src/api_server.rs:5414` | tenant:rabble | callers: rabble, scripts (first pass: `CAP-14`) |
| GET | `/api/rabble/:id/recap/:creature_id` | `social::rabble_recap_handler` | `src/api_server.rs:5419` | tenant:rabble | callers: kask, rabble, scripts |
| POST | `/api/rabble/:id/co-presence` | `social::record_co_presence_handler` | `src/api_server.rs:5423` | tenant:rabble | callers: rabble, scripts |
| POST/PUT/DELETE | `/api/rabbles/:id/follow` | `social::follow_rabble_handler, social::update_follow_handler, social::unfollow_rabble_handler` | `src/api_server.rs:5428` | tenant:rabble | callers: rabble |
| GET | `/api/my/following` | `social::list_following_handler` | `src/api_server.rs:5434` | tenant:rabble | callers: rabble (first pass: `crud:misc`) |
| PUT | `/api/users/social-visibility` | `social::update_social_visibility_handler` | `src/api_server.rs:5439` | tenant:rabble | callers: rabble, scripts (first pass: `crud:misc`) |
| GET | `/api/feed/events` | `social::activity_feed_handler` | `src/api_server.rs:5444` | tenant:rabble | callers: rabble, scripts (first pass: `CAP-05`) |
| GET | `/api/feed/stream` | `social::activity_feed_stream_handler` | `src/api_server.rs:5448` | tenant:rabble | callers: rabble, scripts (first pass: `CAP-05`) |
| GET/POST | `/api/rabble/:id/messages` | `rabble_chat::get_rabble_messages, rabble_chat::post_rabble_message` | `src/api_server.rs:5453` | tenant:rabble | callers: rabble |
| GET | `/api/rabble/:id/stream` | `rabble_chat::rabble_stream` | `src/api_server.rs:5458` | tenant:rabble | callers: rabble |
| POST | `/api/rabble/:id/invite` | `rabble_chat::invite_to_rabble` | `src/api_server.rs:5463` | tenant:rabble | callers: rabble |
| DELETE | `/api/rabble/:id/invite/:user_id` | `rabble_chat::revoke_rabble_invite` | `src/api_server.rs:5467` | tenant:rabble | callers: rabble |
| GET | `/api/rabble/:id/members` | `rabble_chat::list_rabble_members` | `src/api_server.rs:5471` | tenant:rabble | callers: rabble |
| GET/POST | `/api/rabble/join/:qr_token` | `qr_codes::resolve_qr_token_handler, creatures::join_by_qr_token_handler` | `src/api_server.rs:5475` | tenant:rabble | join-by-QR; callers: rabble |
| POST | `/api/rabble/:id/flock` | `rabble_workspace::flock_tick_handler` | `src/api_server.rs:5481` | tenant:rabble | callers: rabble |
| GET | `/api/rabble/:id/flock-history` | `rabble_workspace::flock_history_handler` | `src/api_server.rs:5485` | tenant:rabble | callers: rabble |
| POST | `/api/rabble/:id/end` | `creatures::end_rabble_handler` | `src/api_server.rs:5490` | tenant:rabble | callers: rabble |
| POST | `/api/rabble/:id/leave` | `creatures::leave_rabble_handler` | `src/api_server.rs:5494` | tenant:rabble | callers: rabble |
| POST | `/api/rabble/:id/eject` | `governance::eject_creature_handler` | `src/api_server.rs:5498` | tenant:rabble | callers: rabble |
| DELETE | `/api/rabble/:id/eject/:creature_id` | `governance::lift_ejection_handler` | `src/api_server.rs:5502` | tenant:rabble | callers: rabble |
| POST | `/api/creatures/:creature_id/block` | `governance::block_creature_handler` | `src/api_server.rs:5507` | tenant:rabble | callers: rabble |
| DELETE | `/api/creatures/:creature_id/block/:blocked_creature_id` | `governance::unblock_creature_handler` | `src/api_server.rs:5511` | tenant:rabble | callers: rabble |
| POST | `/api/users/block` | `governance::block_user_handler` | `src/api_server.rs:5515` | tenant:rabble | callers: rabble (first pass: `CAP-14`) |
| DELETE | `/api/users/block/:blocked_user_id` | `governance::unblock_user_handler` | `src/api_server.rs:5519` | tenant:rabble | callers: rabble (first pass: `CAP-14`) |
| GET | `/api/my/blocks` | `governance::list_blocks_handler` | `src/api_server.rs:5523` | tenant:rabble | callers: rabble (first pass: `CAP-14`) |
| POST | `/api/reports` | `governance::create_report_handler` | `src/api_server.rs:5527` | tenant:rabble | callers: rabble (first pass: `CAP-14`) |
| GET | `/api/push/vapid-key` | `push::get_vapid_key_handler` | `src/api_server.rs:5532` | tenant:rabble | callers: rabble |
| POST/DELETE | `/api/push/subscribe` | `push::subscribe_handler, push::unsubscribe_handler` | `src/api_server.rs:5536` | tenant:rabble | callers: rabble |
| POST | `/api/push/proximity` | `push::proximity_check_handler` | `src/api_server.rs:5540` | tenant:rabble | callers: rabble |
| POST | `/api/rabble/:id/transfer-anchor` | `rabble_workspace::transfer_anchor_handler` | `src/api_server.rs:5544` | tenant:rabble | callers: rabble |
| POST | `/api/rabble/:id/update-anchor-position` | `rabble_workspace::update_anchor_position_handler` | `src/api_server.rs:5548` | tenant:rabble | callers: rabble |
| POST | `/api/swarms/:swarm_id/join-batch` | `rabble_workspace::join_batch_handler` | `src/api_server.rs:5553` | tenant:rabble | callers: rabble |
| GET | `/api/rabble/:id/attraction-leaderboard` | `rabble_workspace::attraction_leaderboard_handler` | `src/api_server.rs:5557` | tenant:rabble | callers: rabble |
| POST/GET | `/api/swarm/sessions` | `swarm_telemetry::create_session_handler, swarm_telemetry::list_sessions_handler` | `src/api_server.rs:5562` | drop? | superseded by universal `/api/observe/*` (`src/handlers/observations.rs:7`); no caller (first pass: `CAP-14`) |
| GET | `/api/swarm/sessions/:session_id` | `swarm_telemetry::get_session_handler` | `src/api_server.rs:5567` | drop? | superseded by universal `/api/observe/*` (`src/handlers/observations.rs:7`); no caller (first pass: `CAP-14`) |
| PUT | `/api/swarm/sessions/:session_id/end` | `swarm_telemetry::end_session_handler` | `src/api_server.rs:5571` | drop? | superseded by universal `/api/observe/*` (`src/handlers/observations.rs:7`); no caller (first pass: `CAP-14`) |
| POST/GET | `/api/swarm/sessions/:session_id/telemetry` | `swarm_telemetry::ingest_telemetry_handler, swarm_telemetry::query_telemetry_handler` | `src/api_server.rs:5575` | drop? | superseded by universal `/api/observe/*` (`src/handlers/observations.rs:7`); no caller (first pass: `CAP-14`) |
| GET | `/api/swarm/sessions/:session_id/summary` | `swarm_telemetry::session_summary_handler` | `src/api_server.rs:5580` | drop? | superseded by universal `/api/observe/*` (`src/handlers/observations.rs:7`); no caller (first pass: `CAP-14`) |
| GET | `/api/swarm/sessions/:session_id/experience` | `swarm_telemetry::experience_export_handler` | `src/api_server.rs:5584` | drop? | superseded by universal `/api/observe/*` (`src/handlers/observations.rs:7`); no caller (first pass: `CAP-14`) |
| GET | `/api/swarm-algorithms` | `swarm_algorithms::list_algorithms_handler` | `src/api_server.rs:5589` | tenant:rabble | callers: rabble |
| POST | `/api/swarm-algorithms/activate` | `swarm_algorithms::activate_algorithm_handler` | `src/api_server.rs:5593` | tenant:rabble | callers: rabble |
| GET | `/api/swarm-algorithms/activations/:swarm_id` | `swarm_algorithms::list_activations_handler` | `src/api_server.rs:5597` | tenant:rabble | callers: rabble |
| GET | `/api/swarm-algorithms/:algorithm_id` | `swarm_algorithms::get_algorithm_handler` | `src/api_server.rs:5601` | tenant:rabble | callers: rabble, rabble(compiled-bundle) |
| POST/GET | `/api/observe/platforms` | `observations::create_platform_handler, observations::list_platforms_handler` | `src/api_server.rs:5606` | CAP-13 Observe an outcome | SOSA observations (ground truth for projections); caller `kask/abw-client.js:712`; callers: kask (first pass: `CAP-05`) |
| POST/GET | `/api/observe/sessions` | `observations::create_observation_session_handler, observations::list_observation_sessions_handler` | `src/api_server.rs:5611` | CAP-13 Observe an outcome | SOSA observations (ground truth for projections); caller `kask/abw-client.js:712`; callers: kask (first pass: `CAP-14`) |
| PUT | `/api/observe/sessions/:session_id/end` | `observations::end_observation_session_handler` | `src/api_server.rs:5616` | CAP-13 Observe an outcome | SOSA observations (ground truth for projections); caller `kask/abw-client.js:712`; no caller found (first pass: `CAP-14`) |
| POST/GET | `/api/observe/sessions/:session_id/observations` | `observations::ingest_observations_handler, observations::query_observations_handler` | `src/api_server.rs:5620` | CAP-13 Observe an outcome | SOSA observations (ground truth for projections); caller `kask/abw-client.js:712`; callers: kask (first pass: `CAP-14`) |
| GET | `/api/observe/sessions/:session_id/summary` | `observations::observation_summary_handler` | `src/api_server.rs:5625` | CAP-13 Observe an outcome | SOSA observations (ground truth for projections); caller `kask/abw-client.js:712`; no caller found (first pass: `CAP-14`) |
| GET | `/api/observe/sessions/:session_id/experience` | `observations::observation_experience_handler` | `src/api_server.rs:5629` | CAP-13 Observe an outcome | SOSA observations (ground truth for projections); caller `kask/abw-client.js:712`; no caller found (first pass: `CAP-14`) |
| GET | `/rabble/` | `|| async { Redirect::permanent("/rabble")` | `src/api_server.rs:5686` | tenant:rabble | redirect `/rabble/` → `/rabble` for the Rabble SPA nest; no caller found |
| GET | `/health` | `health_check` | `src/api/server.rs:20` | drop? | legacy `agent-server` router; its only binary is commented out (`Cargo.toml:188` "agent-server archived — superseded by api-server") (first pass: `ops`) |
| GET | `/agents` | `list_agents` | `src/api/server.rs:22` | drop? | legacy `agent-server` router; its only binary is commented out (`Cargo.toml:188` "agent-server archived — superseded by api-server") |
| POST | `/agents` | `register_agent` | `src/api/server.rs:23` | drop? | legacy `agent-server` router; its only binary is commented out (`Cargo.toml:188` "agent-server archived — superseded by api-server") |
| GET | `/agents/:id` | `get_agent` | `src/api/server.rs:24` | drop? | legacy `agent-server` router; its only binary is commented out (`Cargo.toml:188` "agent-server archived — superseded by api-server") |
| POST | `/agents/:id/execute` | `execute_agent` | `src/api/server.rs:25` | drop? | legacy `agent-server` router; its only binary is commented out (`Cargo.toml:188` "agent-server archived — superseded by api-server") (first pass: `CAP-01`) |
| POST | `/agents/:id/save` | `save_agent` | `src/api/server.rs:26` | drop? | legacy `agent-server` router; its only binary is commented out (`Cargo.toml:188` "agent-server archived — superseded by api-server") |
