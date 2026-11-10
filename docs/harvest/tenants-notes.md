# Tenant notes

Narrative companion to [`tenants.md`](tenants.md) (endpoints, regex-extracted). This file replaces the first draft. Written 2026-11-10 from a read-only pass over every tenant's client and server sources; nothing was built or run.

**Conventions.**
- Paths: `fermi/…` is `/home/ilabra/fermi`, `kask/…` is `/home/ilabra/kask`, `rabble/…` is `/home/ilabra/rabble`. Legacy server line numbers are from the working tree, as in `routes.md`.
- `[I]` marks an inference that hasn't been checked at run time.
- Endpoint counts are **method + path** pairs, so `GET` and `POST /api/forecasts` count as two.
- Capability numbers (`CAP-n`) follow `docs/design/capabilities.md` §1–15: 1 Run agents, 2 Compose teams, 4 Coordinate, 5 Observe, 6 Learn, 7 Make claims, 8 Verify claims, 9 Provenance, 10 Involve humans, 11 Gate effects, 12 Account for money, 13 Score fidelity, 14 Connect and protect, 15 Build and ship apps.
- **Last commit dates are scoped to the tenant's path** (`git log -1 --format=%ci -- <path>`). Running `git -C <dir> log -1` gives the whole repo's latest commit instead: fermi 2026-10-08, kask 2026-06-25, rabble 2026-06-06.

---

## 1. Decisions for the owner

**Scope per tenant**

1. **Fermi Console at M3: the core loop or full parity?** The UI calls 93 endpoints. 60 of them form the forecasting loop: agents, forecasts, portfolios, Polymarket, BayesOps and Loop 2 cascades. The other 33 are collaboration: teams, shares, invites, annotations and activity. *Recommendation:* ship the core at M3 and hide the collaboration panels behind a capability check until M5 (compositions and workspaces in full). §3.1 lists both sets.
2. **Fermi Console links the legacy `fermi` crate.** It runs FPL and Monte Carlo locally through `fermi::{ast,lexer,parser,executor,sensitivity,polymarket,agent_backend}` (`fermi/crates/fermi-console/Cargo.toml:65`, 64 `fermi::ast` uses). Legacy can't be archived while the console still depends on it. *Decide:* the new core publishes its oracle-certified kernel crate, and the console depends on that, so client and server share one executor.
3. **Triangular semantics now has a tenant attached.** The console labels and validates driver estimates as true percentiles (`p5 ≤ p50 ≤ p95`, `crates/fermi-console/src/mutations.rs:235`). The executor treats p5/p95 as min/max (`src/distributions.rs:30-40`). This is open decision 1 in `modules.md:448`. Every console forecast shifts when it's settled, so settle it before M3.
4. **DPP Studio and Regulatory Lens are one tenant, not two.** Both are the `adaptogen_lab_regulatory` app (`apps/adaptogen-lab/regulatory-lens/manifest.json`). Its workspaces are named "DPP Studio — {date}", and `static/adaptogen-lab/index.html:1241` spawns them. Treat them as one M6 tenant. The ruleset, claims and composition YAMLs should become workspace seed data. Today the server falls back to reading them from its own working directory (`src/handlers/workspace/lens_actions.rs:53-63,428-472`, `bom_pricing.rs:486`).
5. **DPP passport signing.** The passport isn't signed or hashed today: there's no signing code in `src/handlers/workspace/dpp_publish.rs`, and `tables.md` agrees ("Not hashed/signed"). Signed, hashed publication is a *rebuild* target (G20, `fermi/docs/plans/ABW Refactor-in-Place Plan.md:246`). The previous draft described it as current. *Decide* whether G20 is in the DPP MVP. Recommended yes: it's the strictest suite, and a public passport is where an unsigned snapshot does the most harm.
6. **Rabble: own service, recommended.** The client calls **123 distinct endpoints** (not the 73 in `tenants.md`). Legacy has 119 `tenant:rabble` routes (`routes.md` counts) and 36 Rabble tables (`tables.md` §1g). Only about 10 endpoints touch core capabilities: creature agents, geocode, GBIF-grounded mint, observations and allowance. *Recommendation:* Rabble runs as its own service with its own database, and calls the core for auth, agent execution, observations or grounding, and allowance.
7. **Is Rabble active?** Almost all client work predates 2026-02-28. One commit followed on 2026-06-06, and an untracked bug report dated 2026-05-31 says all three creature agents return `Success` with empty payloads (`rabble/docs/ABW_ISSUE_empty_agent_payloads.md`). No legacy commit clearly fixes it [I]. If Rabble is frozen, move it behind SimOps and DPP within M6, or past M6.
8. **Kask Wild calls two actions that don't exist.** `update_goal` and `annotate_location` are posted to `/api/workspaces/:id/actions/<type>` (`kask/kask-wild-client.js:252,284`). Legacy has no generic action route and no route for either (`src/api_server.rs:4132-4214`), so they return 404 [I]. *Decide:* implement them as typed actions in the new core, or remove them from the client.
9. **Kask pages that don't use ABW are out of scope.** `serum-maker`, `tonic-builder`, `evidence-heatmap`, `pathway-mapper` and `adaptogen-guide` read a static `/assets/data/adaptogen-data.json`. `simops-tool` is superseded. `adc-pk-intake` talks to `localhost:8742` (`kask/projects/adc-pk-intake.html:271`). *Recommendation:* no core work for these.
10. **Twelve kask pages use ABW only for sign-in and a chat widget.** `kask/hooks.js` posts to `/mcp/agents/:id` with a per-page agent. The default agent, `kask_advisor` (`hooks.js:54`), exists only in `kask/agents/kask_advisor/`, not in `fermi/agents/curated/`. Unless that card is imported in M2 with the curated set, the widget breaks on 10 pages. *Decide:* keep the widget (5 endpoints, §3.7) or drop it.
11. **Glasses/HUD: is it a tenant?** Its README says "the endpoint already exists; nothing needs building server-side" (`glasses/hud_field_scout/README.md:68`). Two things contradict that. The shell sends no credentials (`pages/index/index.ink:177-181`), but `execute` is on the protected router (`src/api_server.rs:3654` onward), so it would get a 401 [I]. And `hud_contract::enforce` has **no production caller** (`src/gate_trust.rs:35-36`), so the markers aren't actually stamped. *Recommendation:* define the card as a typed output contract of `hud_field_scout`, plus a device-token auth story, and defer the device itself until it has rendered once.

**Missing tenants**

12. **Pulse, Arborea and Architect Lens exist as design documents only.** Pulse has a folio page. Arborea has a concept paper (`~/Downloads/Arborea — Concept Paper.pdf`, 2026-09-19). Architect Lens has a tenant spec (`~/Downloads/Architect Lens — ABW Tenant Spec (v0).pdf`, 2026-10-01). Folio pages are in `fermi/docs/folio/site/projects/`. No code exists for any of them, and `docs/folio/TBD_REGISTER.md:43` already notes that "No heart-rate overlay code [was] found" for Pulse. *Recommendation:* post-MVP. Still record what they'll need from the substrate in `core-delta.md` now (§3.10): large content-addressed binary artefacts (Gaussian splats), forkable event streams with inherited provenance, evidence tiers on content, and sensor-data provenance.
13. **The cow-tipping game wasn't found anywhere.** I searched `fermi/docs`, `kask`, `uffp*`, `labrastudiogit*`, `abw`, `rabble`, `silat`, `nuria` and 18 other project dirs, plus `~/Documents`, `~/Desktop` and `~/Downloads` filenames. The only hit is an unrelated SUMO ontology file. Confirm it exists, or remove it from `PLAN.md:206` and the Refactor plan's tenant row (`…Refactor-in-Place Plan.md:248`).
14. **"Pulse" already means something else.** In legacy ABW, `Pulse` is an agent message hop: `src/episode_boundary.rs:85`, `static/js/widgets/pulse.js`, and `Pulse::grade` in `bom_pricing.rs:21`. Pick one meaning before naming API resources.

**Cross-cutting**

15. **Allowance replaces four money surfaces.** The console's `/api/wallet` (`crates/fermi-console/src/api/client.rs:2256`), DPP's `/api/workspaces/:id/budget` (`static/adaptogen-lab/index.html:2488,4363`), Rabble's wallet, transfer, billing and grant routes, and the per-app `initial_budget` (`apps/*.json` `workspace_template`). Rabble's peer-to-peer credit transfer (`rabble/lib/screens/send_credits.dart`) is a sale between owners, which belongs to ACN at M4. Confirm Rabble's MVP has no transfers.
16. **Workspace deletion versus append-only.** SimOps deletes a workspace with `DELETE /api/teams/:id` (`kask/abw-client.js:490-491`), which is a bug. *Decide* the new semantics: archive or tombstone, given append-only from M1.
17. **`tenants.md` undercounts badly.** Its regex missed `Uri.parse('$baseUrl/…')` (Rabble), `${base()}` template literals (DPP), `ABW.*` SDK calls (SimOps, Wild), wildcard routes (`files/*path`) and literal-versus-`:param` matches. It also counted comment text as calls. §2.1 lists the corrections. *Decide* whether to fix `harvest_tenants.py` or treat the counts in this file as authoritative. About 30 routes on `routes.md`'s "no caller" list are called by Rabble.
18. **There are two copies of this file.** `/home/ilabra/abw/docs/harvest/tenants-notes.md`, in the new-core repo `ilabra-axo/abw`, still holds the old draft. Only the staging copy was changed. Copy this one across.

---

## 2. Summary

| Tenant | Where (client · server) | Size | Last commit (path) | Health | MVP endpoints | Milestone |
| --- | --- | --- | --- | --- | --- | --- |
| Fermi Console | `fermi/crates/fermi-console` · `handlers/{forecasts,polymarket,bayesops,relationships,…}` | 40 `.rs`, 75,585 lines; 416 unit tests | 2026-08-28 | **Usable.** v0.18.0, 0 TODOs, no user-facing breakage at last handover; 14 client methods dead | **60** core (+33 collab) | **M3** |
| SimOps | `kask/simops-*.js`, `kask-sim-client.js`, `kask/dynamics`, `kask/projects/simops*.html` · `crates/{simops,dynamics,projections}`, `handlers/{simops,observations,simops_benchmark}.rs` | client 58,982 lines (21 + 1 JS, 2 dynamics, 4 pages); server ≈ 10,100 | client 2026-06-12; `crates/simops` 2026-08-07 | **Dormant, probably usable [I].** Verification plan not executed; delete bug | **35** | M6 |
| DPP Studio (incl. Regulatory Lens) | `fermi/static/adaptogen-lab`, `fermi/apps/adaptogen-lab` · `handlers/workspace/{carbon,bom_pricing,dpp_publish,lens_actions,claim_evaluation}.rs` | client 5,630; app data 1,657; server 6,862 | 2026-10-08 | **Most active, partly unproven.** Carbon fixes not yet seen in a live run; citations unverified | **29** | M6 |
| Regulatory Lens | `fermi/apps/adaptogen-lab/regulatory-lens` | 6 files, 1,492 lines (data) | 2026-09-16 | Data plus manifest; merged into DPP (decision 4) | (in DPP) | M6 |
| Rabble | `rabble/lib` (Flutter); build in `fermi/rabble-web` · `handlers/creatures/*`, `rabble_workspace.rs`, `rabble_chat.rs`, `push.rs`, `swarm_algorithms.rs` | 71 `.dart`, 34,944 lines; server ≈ 15,240 | 2026-06-06 (before that, 2026-02-28) | **Dormant; open agent bug** | **47** (of 123 called) | M6 |
| Kask Wild | `kask/kask-wild-client.js`, `kask/projects/wild*.html` · `handlers/wild.rs`, `workspace/actions.rs:1174+` | 2,239 lines; server 464 + action handler | 2026-06-14 | Phase 1 shipped 2026-06-05; 2 of 4 actions 404 | **16** | M6 |
| Kask project pages | `kask/projects/*.html`, `kask/hooks.js` | 23 pages, 21,967 lines | 2026-06-14 | Static site; deploy fixes 2026-06-25; default chat agent not in curated set | **5** | M6 |
| Glasses / HUD | `fermi/glasses` · `src/hud_contract.rs`, `src/glasses_shell.rs` | 15 files, 773 lines; server 2,036 | 2026-08-20 | **Never rendered.** `STUB = true`; would 401 [I]; contract has no production caller | **1** (+ device auth) | M6 |
| Pulse | `fermi/docs/folio/site/projects/pulse.html` | design only | — | Concept; no code | 0 | post-MVP |
| Arborea | folio page; concept paper PDF in `~/Downloads` | design only | — | Concept paper | 0 | post-MVP |
| Architect Lens | folio page; tenant spec v0 PDF in `~/Downloads` | design only | — | Spec v0 with Lattice mapping | 0 | post-MVP |
| Cow-tipping game | not found | — | — | — | — | ? |

### 2.1 Corrections to `tenants.md`

| `tenants.md` says | Ground truth |
| --- | --- |
| Rabble: 73 endpoints | **123** distinct paths in `rabble/lib`. Missed examples: `/api/creatures/:id/{enemy-sensor,genome-profiler,prey-locator,forage,dream}` (`services/api_client.dart:1300-1371`), `/api/rabble/:id/stream` (`services/rabble_stream_service.dart:232`), friendships, blocks, push subscribe |
| DPP: 2 endpoints | **≈ 22 paths** in `static/adaptogen-lab/index.html`, e.g. `actions/{calculate_carbon,price_bom,evaluate_claims,compare_lenses}` (`:2887,2216,2685,1915`), `dpp/publish` (`:1107`), `budget`, `messages/poll` |
| SimOps: 2 endpoints | About 30, called through `ABW.*` in `kask/abw-client.js` (listed under kask-shared) |
| Wild: 1 endpoint | 16, through `ABW.*` plus `_postAction` |
| Glasses: 0 endpoints, 2 files | 1 endpoint (`index.ink:177`); 15 files |
| `/api/workspaces/:p/files/:p`: no legacy route | Exists as `files/*path` (`src/api_server.rs:4475-4479`) |
| `/api/workspaces/:p/actions:p` and `actions/pending.`: no route | Query-string form of `actions` and `actions/pending` (`:4133,4137`); the second match is comment text (`kask-sim-client.js:976`) |
| Console: `/api/apps/fermi_forecast/workspaces` and `/api/orchestras/fermi/members` unmatched | Match `:slug` and `:name` (`:3961,4971`) |
| Console: `/api/agents/fermi/messages` | Comment only (`crates/fermi-console/src/chat.rs:209`, "Slice 3 … currently unused") |
| Console: `/api/agents/:p/calibration` called | Comment only (`api/client.rs:2337`); the console uses `/api/agents/contributions` instead |

---

## 3. Tenants

### 3.1 Fermi Console (M3)

- **Location and size:** `fermi/crates/fermi-console`, a Rust/GPUI desktop app, v0.18.0 (`Cargo.toml:3`). 40 source files, 75,585 lines, 416 `#[test]`s in the GPUI-free lib target (`Cargo.toml:8-13`), plus `tests/invocation_envelope.rs`. Last commit 2026-08-28 ("migrate the agreement check to Claim").
- **What it does:** a forecasting command centre. You write or import a question (Polymarket search, `README.md:113-123`) and decompose it into FPL drivers. Agents research each driver and suggest distributions, Monte Carlo runs locally, and the forecast is published, versioned, shared and finally resolved and scored (Brier, leaderboard).
- **Claims and ground truth:**

  | Claim | Ground truth | Caveat found |
  | --- | --- | --- |
  | P(question) by deadline, from an FPL model | Polymarket resolution (`/api/polymarket/check-resolutions`) or manual resolve | Driver percentiles are interpreted as min/max by the executor (decision 3) |
  | Per-driver p5/p50/p95 from a named agent | Same outcome, through attribution | Per-agent calibration copies the team Brier to every agent (`modules.md:341`); Shapley credit has probably never run on real data (`README.md:17`) |
  | Base-rate frequency with a reference class | Platform-measured frequency (commit 2026-08-21, "compare the declared historical_frequency…") | `mutations.rs:172` refuses a frequency without a class |
  | Crowd divergence (pp) | Polymarket snapshot | "Costs 1 credit per search" (`README.md:153`) means allowance pricing |
- **Core capabilities:** CAP-01 (agent list, execute and stream, roster), CAP-04 (MoE routing reads `/api/agents/contributions`, `wire.rs:134`), CAP-07/08 (forecast claims), CAP-05 (Polymarket outcome observer), CAP-06 Loop 2 (pending cascades) and Loop 5, CAP-10 (BayesOps accept/reject; cascade apply/dismiss), CAP-13 (Brier, leaderboard), CAP-12 (wallet, which becomes allowance), CAP-14 (OAuth, sharing, teams), kernels (FPL, MC, Sobol, BayesOps, Shapley).
- **Health:** usable. 0 TODO/FIXME in `src/`. The last legacy handover reports "None user-facing today" (`fermi/docs/HANDOVER_2026-08-01.md:98-100`), and the `eval_brier` bug it lists has since been removed. Smaller findings:
  - 14 client methods have no caller: `get_agent`, `health`, `public_forecasts`, `forecast_activity`, `void_forecast`, `list_my_sent_invites`, `list_portfolio_invites`, `list_teams`, `add_team_member`, `update_team_member_role`, `add_workspace_dependency`, `list_workspace_outputs`, `refit_workspace`, `post_workspace_action`. Their endpoints don't need porting for the console.
  - The README says "SQLite cache" (`README.md:231`), but `rusqlite` (`Cargo.toml:24`) has no use in `src/`, and `README.md:159` says the backend is the only store. The README is stale.
  - The console depends on the legacy rate-limit contract: 10 LLM calls per 60 s, with the retry delay in the 429 *body* because there's no `Retry-After` header (`src/abw_pacing.rs:8-13`). The new core should send `Retry-After` and keep a per-user bucket.
  - Sign-in uses loopback OAuth: `{base}/auth/{provider}?redirect=<localhost>&app=fermi_console` (`src/main.rs:1546`). API keys (`ferm_*`) also work (`README.md:79`).
- **MVP surface: core loop, 60 endpoints.**
  - *Auth and identity (CAP-14), 2:* `GET /auth/{provider}` (loopback redirect), `GET /api/auth/me`.
  - *Allowance (CAP-12), 1:* `GET` balance (replaces `GET /api/wallet`).
  - *Agents (CAP-01/04), 5:* `GET /api/agents`, `POST /api/agents/:id/execute`, `POST /api/agents/:id/execute/stream` (SSE; `cockpit.rs:4066`), `GET /api/agents/contributions`, `GET /api/orchestras/:name/members`.
  - *Forecast workspaces (CAP-15), 5:* `POST` and `GET /api/apps/fermi_forecast/workspaces`, `POST /api/workspaces/:id/messages`, `GET` and `PUT /api/workspaces/:id/outputs/:key`.
  - *Forecasts (tenant; CAP-07), 17:*
    - `POST` and `GET /api/forecasts`; `GET`, `PUT` and `DELETE /api/forecasts/:id`;
    - `POST …/:id/update-probability`, `POST …/:id/resolve`, `GET /api/forecasts/my-stats`;
    - `GET …/:id/history`, `GET …/:id/history/:rev`, `POST …/:id/revert`;
    - `GET …/:id/timeline`, `GET …/:id/cascade-provenance`;
    - `GET` and `PUT …/:id/schedules`, `DELETE …/schedules/:sid`, `POST …/schedules/:sid/run`.
  - *Portfolios (tenant), 8:* `POST` and `GET /api/portfolios`; `PATCH` and `DELETE /api/portfolios/:id`; `GET` and `POST …/:id/forecasts`; `DELETE …/:id/forecasts/:fid`; `GET …/:id/stats`.
  - *Outcome observer (CAP-05/08), 4:* `POST /api/polymarket/{search,link,snapshot,check-resolutions}`.
  - *Scoring (CAP-13), 1:* `GET /api/leaderboard`.
  - *BayesOps (CAP-10 kernel), 3:* `GET /api/workspaces/:id/bayesops/state`, `POST /api/bayesops/pending/:id/accept`, `POST /api/bayesops/pending/:id/reject`.
  - *Relationships and Loop 2 queue (CAP-06/10), 14:*
    - `POST` and `GET /api/forecast-relationships`, `DELETE …/:id`, `POST …/:id/propagate`;
    - `POST` and `GET /api/relationship-groups`, `GET …/:id`, `POST …/:id/propagate`;
    - `GET /api/forecasts/:id/groups`, `POST` and `DELETE …/groups/:gid`;
    - `GET /api/pending-cascades`, `POST …/:id/apply`, `POST …/:id/dismiss`.
    - `PUT …/groups` isn't called, so it's excluded.
  - *Deferred collaboration (CAP-14/02), 33:*
    - Forecasts: `access`, `shares` (×3), `invites` (×2), `annotations` (×4).
    - Portfolios: `access`, `activity`, `invites POST`, `shares` (×3).
    - Teams: `GET`/`POST /api/teams`, `GET`/`DELETE /api/teams/:id`, `activity`, `contributions`, `invites` (×2), `DELETE members/:uid`, `PUT members/:uid/capabilities`, `ops`, `shared`.
    - Invites: `/api/invites/:id` (`DELETE`, `accept`, `decline`), `GET /api/me/invites`, `GET /api/users/lookup`.
  - *Drop:* `GET /api/rbac/self-check`. It diagnoses legacy dual-identity and owner-type drift (`api/client.rs:2810-2816`); the new core's tenancy-by-ID (T17) replaces it [I].

### 3.2 SimOps (M6, first)

- **Location and size:**
  - Client: `kask/simops-*.js` (21 files; `simops-page.js` alone is 30,056 lines), `kask/kask-sim-client.js` (12,809), `kask/dynamics/{identity,linear_decay}.js`, and `kask/projects/simops{,-compose,-wizard}.html` (`simops-tool.html` is superseded). 58,982 lines in total. Uses the shared SDK `kask/abw-client.js` (1,345 lines).
  - Server: `fermi/crates/simops` (5,501 lines, 59 tests, `modules.md:49`), `crates/dynamics` (2,557), `crates/projections` (1,009), and `src/handlers/{simops,observations,simops_benchmark}.rs` (2,176).
  - App: `fermi/apps/kask_simops.json`, composition `simops_fleet`, schema `kask-simops/2`.
  - Specs: `kask/adaptogen/simops-v2/` (capability brief, app manifest, specs).
- **Last commit:** client 2026-06-12. The last SimOps-specific commit was 2026-06-06 ("cascade 400 — root-cause at the writer"). `crates/simops` changed on 2026-08-07.
- **What it does:** agent-led process modelling for bio-manufacturing. An interview designs a process (`simops-wizard`, `simops_advisor`), which you edit as `simops/process.yaml` in the workspace. You then run the cascade (mass, carbon, NER, opex per stage), dynamics and rheology models to project outcomes, bind live sensors, and compare projections with measured batches.
- **Claims and ground truth:**
  - Claims: stage outputs, carbon delta, NER, opex (cascade); time-series projections (dynamics); rheology predictions.
  - Ground truth: measured batch observations ingested as SOSA observations (`/api/observe/sessions/:id/observations`).
  - Legacy has the right anchor: `projection_commit.rs` proves a projection pre-dated its measurement, and `process_spacetime` records a `committed_before_measured` flag (`tables.md` §1h). But projection-accuracy scoring has never had input (`README.md:19`).
  - The verification plan (`fermi/docs/PLAN_simops_verification.md`) isn't done. Its target test `tests/simops_composition.rs` doesn't exist. `agents/curated/simops_cascade/` and `comparator/` have only `agent_card.json`, with no output contract (step 1 and step 2 incomplete).
- **Core capabilities:** CAP-15 (app spawn, workspace files as the canonical document, git log and diff, messages and stream, `mutate_document`), CAP-02 (`auto_hire` of 12 agents, hire, add), CAP-01 (execute), CAP-05 (SOSA platforms, sessions, observations), kernels (cascade_v2, dynamics, rheology, FPL), CAP-07/08 (projections as claims with commitment), CAP-10 (pending actions).
- **Health:** dormant since June, probably usable [I].
  - Workspace delete hits the teams route (decision 16).
  - The client polls arbitrary user-supplied sensor URLs directly from the browser (`kask-sim-client.js:7574`, `simops-page.js:28136`). Those readings bypass server provenance until they're ingested.
  - `PredictorEngine::Conditional` was never built (`modules.md:49`).
  - The kask site's last commits are Railway deploy fixes (2026-06-25). Whether `kask.bio` is live now wasn't checked [I].
- **MVP surface: 35 endpoints.** These are the `ABW.*` calls the SimOps sources actually make.
  - *Auth (CAP-14), 3:* `GET /auth/{provider}`, `GET /auth/logout`, `GET /api/auth/me`.
  - *App (CAP-15), 2:* `POST` and `GET /api/apps/kask_simops/workspaces`.
  - *Workspaces (CAP-15), 3:* `GET /api/workspaces`, `GET /api/workspaces/:id`, and a real delete or archive (decision 16).
  - *Documents (CAP-15), 5:* `GET …/files`, `GET` and `PUT …/files/*path` (`readWorkspaceFile` ×36, `writeWorkspaceFile` ×5), `GET …/git/log`, `GET …/git/diff`.
  - *Messages (CAP-15), 3:* `GET` and `POST …/messages`, `GET …/messages/stream`.
  - *Actions (CAP-10/15), 2:* `POST …/actions/mutate_document`, `GET …/actions` (with `?pending`).
  - *Workspace roster (CAP-02), 5:* `GET` and `POST …/agents`, `POST …/hire`, `POST …/add`, `DELETE …/agents/:id`. Consider collapsing to one "add member" call.
  - *Agents (CAP-01), 2:* `GET /api/agents`, `POST /api/agents/:id/execute`.
  - *SimOps kernels, 4:* `POST /api/simops/cascade`, `POST /api/simops/cascade/suggest-bindings`, `POST /api/simops/dynamics`, `GET /api/simops/dynamics/models`.
  - *Observations (CAP-05), 4:* `POST /api/observe/platforms`, `POST /api/observe/sessions`, `POST` and `GET /api/observe/sessions/:id/observations`.
  - *FPL and forecast, 2:* `POST /api/fpl/execute`, `POST /api/forecasts`.
  - *Not called by the UI (SDK only), so defer:* `/api/simops/{project,rheology,rheology/models}` and `/api/workspaces/:id/cascade`. Separately, the server-side projection commitment (`process-spacetime`, `sample-config`) is needed by the acceptance suite even though no client calls it.

### 3.3 DPP Studio, including Regulatory Lens (M6, second)

- **Location and size:**
  - Client: `fermi/static/adaptogen-lab/index.html` (5,119 lines), `passport.html` (206), `embed-example.html` (305).
  - App: `fermi/apps/adaptogen-lab/dpp/{claims,composition}.yaml` and `regulatory-lens/*` (§3.4).
  - Server: `src/handlers/workspace/{carbon (2,887), bom_pricing (959), dpp_publish (751), lens_actions (1,247), claim_evaluation (1,018)}.rs`.
  - Agents: `dpp_companion`, `regulatory_lens_translator`, `carbon_accountant`, `supply_chain_oracle`.
  - Last commit 2026-10-08, the most recent of any tenant.
  - `kask/adaptogen` is *not* DPP. It holds the adaptogen reference database (24 tables, seed SQL) and SimOps v2 specs.
- **What it does:** you load a product's bill of materials and label claims. The agents evaluate each claim against EU, US and CN regulation with citations, price the BOM, and compute product carbon from retrieved emission factors (the platform does the multiplication in Rust). You can then publish a public digital product passport with a QR code.
- **Claims and ground truth:**

  | Claim | Ground truth | State |
  | --- | --- | --- |
  | Claim X is permitted in market M (verdict + citations) | Primary registers: EFSA, 21 CFR 101, SAMR (`manifest.json` `data_honesty.verification_sources`) | Citations are `tool_verified`, but cited URLs aren't cross-checked: "a cited URL is currently unverified" (`data_honesty.what_would_make_a_verdict_stronger`). Endorsement by a qualified reviewer outranks the agent |
  | Product carbon kg CO2e/kg | Emission factor (retrieved, cited) × BOM quantity (Rust) | `carbon.rs:22-37`. Non-mass-basis factors are refused (`COORDINATION_DPP_STUDIO_PANELS_AND_PASSPORT.md:48-58`). A material or reference-flow mismatch is still open (`:60-65`). The fixes haven't been seen in a live run (`:24,504-508`) |
  | BOM price | `supply_chain_oracle` with the grounding gate | Server owns the %→qty conversion (`bom_pricing.rs:30-36`) |
  | The passport is what was released | Allowlisted snapshot behind a token (`dpp_publish.rs:12-32`) | Not signed or hashed (decision 5) |
- **Core capabilities:** CAP-15 (app, files, typed actions, public publication), CAP-01 (agent runs inside actions; `dispatch_rabble_action` needs no hire, `bom_pricing.rs:25-26`), CAP-08/09 (grounding enforce, provenance stamps, `needs_expert`), CAP-10 (endorsement on stored evaluations), CAP-12 (budget, which becomes allowance), external tools (web search; it refuses with 503 without `BRAVE_SEARCH_API_KEY`, `manifest.json` `evaluate_claims`), kernels (carbon arithmetic, unit basis).
- **Health:** under active development until 2026-10-08, with careful honesty labelling (`composition.yaml:93-112` is still `mode: synthetic`, 0.41 typed by a person). The reload-visibility bug for carbon output was diagnosed as client-only (`COORDINATION…PASSPORT.md:93-122`), and later commits address it. No TODOs in the client.
- **MVP surface: 29 endpoints.**
  - *Auth (CAP-14), 3:* `GET /auth/{provider}`, `GET /auth/logout`, `GET /api/auth/me`.
  - *App (CAP-15), 3:* `POST` and `GET /api/apps/adaptogen_lab_regulatory/workspaces`, and `GET /api/apps/adaptogen_lab_regulatory/schema` (public; used by the third-party embed).
  - *Documents, 2:* `GET` and `PUT /api/workspaces/:id/files/*path`.
  - *Roster (CAP-02), 3:* `GET …/agents`, `POST …/hire`, `POST …/add`. The client tries `hire` and then `add` (`index.html:2085-2102`); one idempotent call would replace both.
  - *Typed actions (CAP-08/15), 7:* `GET …/actions`, then `POST …/actions/{evaluate_claims, calculate_carbon, price_bom, compare_lenses, render_lens, flag_divergence}`.
  - *Allowance (CAP-12), 2:* `GET` and `POST …/budget`, which become per-workspace allowance.
  - *Messages, 2:* `POST …/messages`, `GET …/messages/poll?since=`.
  - *Publication (CAP-15/14), 6:* `POST`, `GET` and `DELETE …/dpp/publish`; public `GET /api/dpp/p/:token`, `GET /api/dpp/p/:token/qr`, `GET /dpp/:token`.
  - *Ops, 1 (deferrable):* `GET /api/agents/:id/metrics`.

### 3.4 Regulatory Lens (folded into DPP)

- **Location and size:** `fermi/apps/adaptogen-lab/regulatory-lens/`: `manifest.json` (329 lines), `rulesets/{eu_reg_1924,us_fda_ftc,cn_gb_samr}.yaml` (898), `ontology/patterns.yaml` (95), `sku/precision_kombucha_hibiscus_f2.yaml` (170). 6 files, 1,492 lines. Last commit 2026-09-16.
- **What it does:** this is the manifest and data for the DPP app. It renders one product's claims as compliant labels for three markets and shows where the regulatory *philosophies* diverge (positive list, risk-based, categorical).
- **Claims:** the rulesets are explicitly "SYNTHETIC-BUT-STRUCTURALLY-REPRESENTATIVE" (`rulesets/eu_reg_1924.yaml:10`) and off by default (`include_synthetic_seed`). Rows served from them are tagged `synthetic_seed`, and `not_evaluated` never means permitted (`manifest.json` `data_honesty.caveat`).
- **Health:** fine as data. The server's filesystem fallback (decision 4) is a deployment coupling.
- **MVP surface:** none of its own. Its 6 action types are in §3.3. Its roadmap (JP and KR lenses, a live camera label renderer) is post-MVP.

### 3.5 Rabble (M6, third, or its own service)

- **Location and size:**
  - Client: `rabble/lib`, Flutter, 71 files and 34,944 lines; the web build is in `fermi/rabble-web`. `README.md` is the Flutter template.
  - Server: `src/handlers/creatures/*` (11,754 lines), `rabble_workspace.rs` (1,648), `rabble_chat.rs` (927), `push.rs` (540), `swarm_algorithms.rs` (371).
  - Data: 36 tables (`tables.md` §1g), 119 routes. Agents: `enemy_sensor`, `genome_profiler`, `prey_locator`, `rabble_*` (3).
  - Last commit 2026-06-06. Before that, 2026-02-28 (Brilliant Frame AR prototype, rabble SSE stream).
- **What it does:** a multiplayer AR game. You mint a digital-twin insect from a real GBIF species, fly or perch or tether it at real locations, and gather with others' creatures in "rabbles" with chat, presence and flocking. Creature agents scan for threats or prey and profile the genome.
- **Claims and ground truth:** species identity and taxonomy (minted from `gbif_key`, `creatures/identity.rs:34`) checked against GBIF records; ecological agent outputs (threat level, prey targets, genome profile) checked against GBIF and literature. The `genome_profiler` once produced a fabricated value, which motivated `grounding_trust` (`src/hud_contract.rs:18-21`). Everything else (flights, presence, chat, collections) is game state, not claims.
- **Core capabilities:** CAP-01 (creature agents, geocode), CAP-05 (telemetry and flights write `sosa_*` tables, `creatures/flights.rs:522-573`), CAP-09 (GBIF grounding), CAP-14 (auth, profile, notifications, push, blocks and reports), CAP-12 (wallet, billing and transfer, which go to allowance and ACN), CAP-15 (`workspaces/:id/upload`, `files-raw` for art). Most of the remainder is tenant CRUD.
- **Health:** dormant. Open bug: all creature agents return empty `summary` and `key_findings` (`rabble/docs/ABW_ISSUE_empty_agent_payloads.md`, dated 2026-05-31), with a fix not confirmed [I]. Two TODOs in `lib`. There's a stray `screens/explore_screen.dart.oTiDJbV` temp file. The test checklist (`fermi/docs/RABBLE_TEST_CHECKLIST.md`) was last touched 2026-02-17.
- **MVP surface: 47 endpoints** (of 123 called).
  - *Auth and profile (CAP-14), 4:* `GET /auth/{provider}`, `GET /api/auth/me`, `GET /api/users/:id`, `GET /api/profile`.
  - *Creatures (tenant), 9:* `GET /api/creatures`, `GET /api/creatures/:id`, `POST /api/creatures/mint`, `PUT …/:id/update`, `PUT …/:id/status`, `PUT …/:id/presence`, `GET …/:id/image`, `GET …/:id/animation/:layer`, `POST …/:id/generate-art`.
  - *Creature agents (CAP-01 via tenant), 5:* `POST /api/creatures/:id/{enemy-sensor, genome-profiler, prey-locator, forage, dream}`.
  - *Rabbles (tenant), 12:*
    - `GET /api/swarms`, `GET /api/swarms/:id`, `POST /api/swarms/:id/join`;
    - `POST /api/creatures/:id/host`, `POST /api/rabble/:id/leave`, `POST /api/rabble/:id/end`;
    - `GET` and `POST /api/rabble/:id/messages`, `GET …/members`, `GET …/stream` (SSE);
    - `GET` and `POST /api/rabble/join/:token`.
  - *Movement (tenant), 6:* `POST …/:id/perch`, `POST …/:id/fly`, `POST` and `DELETE …/:id/tether`, `POST …/:id/telemetry`, `GET …/:id/track`.
  - *Flights (CAP-05), 3:* `POST /api/flights`, `PUT /api/flights/:id/end`, `POST /api/flights/:id/telemetry`.
  - *Feed (tenant), 3:* `GET /api/feed`, `GET /api/feed/stream`, `GET /api/dashboard/nearby`.
  - *Push and notifications (CAP-14), 4:* `GET /api/push/vapid-key`, `POST /api/push/subscribe`, `GET /api/notifications`, `PUT /api/notifications/:id/read`.
  - *Grounding (CAP-01), 1:* `GET /api/geocode`.
  - *Deferred:* social (friendships, creature invites, contacts, blocks, reports, follows), swarm algorithms, devices, collections, flight import and export, and admin (which becomes ops).
  - *Replaced:* wallet, transactions, transfer, billing and admin grant move to allowance and ACN.

### 3.6 Kask Wild (M6)

- **Location and size:** `kask/kask-wild-client.js` (549 lines) and `kask/projects/{wild,wild-app}.html` (1,690). App `fermi/apps/kask_wild.json` (composition `wild_forager`, 7 auto-hired agents). Server: `src/handlers/wild.rs` (464 lines) and `log_observation_handler` (`workspace/actions.rs:1174-1206`). Last commit 2026-06-14.
- **What it does:** a foraging journal. You log finds (species, place as an H3 cell, conditions, flavour), keep named spots and goals, and talk to `wild_companion`. The planned scout replies combine iNaturalist, MycoBank, GBIF and weather forecasts (`kask_wild.json` `metadata.data_sources`).
- **Claims and ground truth:** species identification (planned; `identify` exists server-side, `src/api_server.rs:4206-4214`) checked against iNaturalist and GBIF. Condition forecasts checked against later observations; the roadmap wants a Brier badge, which is blocked on ABW (`kask/docs/wild-roadmap.md:70`). Journal entries are user testimony.
- **Core capabilities:** CAP-15 (app, files, typed actions, messages and stream), CAP-05 (observations), CAP-01 (companion), CAP-13 (future calibration). Cross-workspace delegation: "Rabble creatures consume Wild" (`kask_wild.json` description; `src/handlers/wild.rs:3`).
- **Health:** Phase 1 shipped 2026-06-05 (`wild-roadmap.md:35-37`), and Phase 2 is blocked on three ABW features (`:68-70`). `update_goal` and `annotate_location` have no route (decision 8). The gate-lockout bug was fixed 2026-06-12.
- **MVP surface: 16 endpoints.**
  - *Auth, 3:* OAuth, logout, `GET /api/auth/me`.
  - *App, 2:* `POST` and `GET /api/apps/kask_wild/workspaces`.
  - *Workspaces, 3:* `GET /api/workspaces`, `GET /api/workspaces/:id`, `GET …/agents`.
  - *Documents, 1:* `GET …/files/*path`.
  - *Actions, 4:* `POST …/actions/{log_observation, update_goal, annotate_location, mutate_document}` (`mutate_document` is the fallback).
  - *Messages, 3:* `GET` and `POST …/messages`, `GET …/messages/stream`.

### 3.7 Kask project pages (M6)

`kask/projects/`: 23 pages, 21,967 lines, last commit 2026-06-14. The site is served by `kask/server.js` on Railway (`railway.toml`). "Chat" means the `hooks.js` widget: `POST /mcp/agents/:id`, `method: execute` (`abw-client.js:354-371`).

| Page | What it is | Uses ABW? |
| --- | --- | --- |
| `adc-pk.html` | ADC-PK: Chemprop v2 model for antibody–drug conjugate pharmacokinetics (T2X); architecture doc that plans an `adc_pk_oracle` ABW agent | Sign-in + chat (`kask_advisor`) only; the agent is planned, not built [I] |
| `adc-pk-intake.html` | Training-data intake form | Yes: `GET /api/workspaces`, `PUT files` (`:473-477`). Its validator is `localhost:8742` (`:271`), so it's a dev tool |
| `ambu.html` | AMBU: autopoietic bio-robotic distributed biomanufacturing (landing) | Sign-in + chat (`bioreactor_modeler`) |
| `elm.html` | ELM: engineered living materials, adaptive wellness R&D (landing) | Sign-in + chat (`kask_advisor`) |
| `fb-fpt.html` | Full Brew: precision fermentation, terroir × microbiome (landing) | Sign-in + chat |
| `konstanz.html` | EU incubation node, German–Swiss border (landing) | Sign-in + chat |
| `monterey.html` | Monterrey node with Tec de Monterrey (landing) | Sign-in + chat |
| `fermi.html` | Fermi landing page | Sign-in + chat |
| `rabble.html`, `agent-bestiary.html` | Landing pages | Sign-in + chat (`xaman_ek`) |
| `adaptogenic-wellness.html`, `adaptogen-browser.html` | Adaptogen platform landing page and species browser (static JSON) | Sign-in + chat (`adaptogen_curator`) |
| `simops.html`, `simops-compose.html`, `simops-wizard.html` | SimOps (§3.2) | Yes |
| `simops-tool.html` | Old digital-twin tool, superseded 2026-05-16 | No |
| `wild.html`, `wild-app.html` | Wild (§3.6) | Yes |
| `serum-maker`, `tonic-builder`, `evidence-heatmap`, `pathway-mapper`, `adaptogen-guide` | Static adaptogen tools over `/assets/data/adaptogen-data.json` | **No** (meta tag only) |

- **Health:** static and working [I]. `kask_advisor` isn't in the curated agent set (decision 10). `ABW.follows` is localStorage only (`abw-client.js:1240-1280`), so it has no server dependency.
- **MVP surface: 5 endpoints.** `GET /auth/{provider}`, `GET /auth/logout`, `GET /api/auth/me`, `GET /api/health`, `POST /mcp/agents/:id` (execute). Add 2 more if `adc-pk-intake` is kept.

### 3.8 Glasses / HUD (M6)

- **Location and size:** `fermi/glasses/{minimal_probe, hud_field_scout}`. These are Rokid AIUI packages: 15 files and 773 lines, including docs (`RUNNING_IN_CRAFT.md`, 138 lines). Server side: `src/hud_contract.rs` (1,169), `src/glasses_shell.rs` (867), and `tests/{glasses_shell_parity,hud_contract}.rs`. Agent: `agents/curated/hud_field_scout`. Last commit 2026-08-20.
- **What it does:** you ask a question aloud on display glasses and get a glanceable card back. Each line carries a provenance marker (verified, inferred, unknown), green on black (`hud_field_scout/README.md:1-21`).
- **Claims:** the card's markers are the claim: "computed by `src/hud_contract.rs` server-side and arrive already stamped" (`README.md:6-8`). Ground truth is whatever the agent's grounding resolved. As noted in decision 11, nothing on the execute path calls `enforce` today (`gate_trust.rs:35-36`).
- **Core capabilities:** CAP-01 (execute), CAP-09 (provenance vocabulary), CAP-11 (display gate), CAP-14 (device auth, which is missing).
- **Health:** "Nothing here has rendered" (`README.md:23-33`). `STUB = true` (`pages/index/index.ink:62`). The README points to `pages/card/index.ink` (`README.md:40,63`), but the file is `pages/index/index.ink`. Camera input isn't wired (`README.md:70-74`).
- **MVP surface: 1 endpoint plus auth.** `POST /api/agents/hud_field_scout/execute`, returning the HUD card contract, with a device or API-token credential.

### 3.9 Pulse (post-MVP)

- **Found:** design only. `fermi/docs/folio/site/projects/pulse.html` and `docs/folio/FOLIO_DRAFT.md:44,83-87`: "AR overlay of a person's live heart rate … with consent", stage "Prototype".
- **Ground truth versus the folio:** `TBD_REGISTER.md:43` says "No heart-rate overlay code found". What exists is the Brilliant Frame prototype in Rabble (`rabble/lib/services/frame_service.dart`, `screens/frame_ar_screen.dart`) and the Rokid HUD shell (§3.8). A grep for heart rate and bpm across `rabble`, `glasses`, `src`, `kask`, `butterly` and `uffp_mobile` found nothing.
- **Core needs, if built:** consent-gated publication of a private sensor stream (CAP-14, CAP-05), and sensor-data provenance (Refactor plan:248).

### 3.10 Arborea and Architect Lens (post-MVP)

- **Arborea:** design only. Folio page `docs/folio/site/projects/arborea.html`; concept paper `~/Downloads/Arborea — Concept Paper.pdf` (2026-09-19).
  - It's a slow world-building game where trees are the primitive. Players publish rival atlases that can be checked against sensors.
  - It has three data tiers (instrumented, inferred, testimonial) plus "witnessed" readings from a Dendrometer device.
  - It reuses GBIF grounding as Rabble does, and wants the five-gate Lattice applied to narrative content.
  - *Core needs:* sensor observations, tier-stamped provenance, forkable published artefacts (atlases).
- **Architect Lens:** design only. Folio page `architect-lens.html`; tenant spec `~/Downloads/Architect Lens — ABW Tenant Spec (v0).pdf` (2026-10-01).
  - It's an AR walk through unbuilt, past and doctrinal states of a real place.
  - Its entities (Site, Layer, Surface, Promenade, Reading) are event-sourced, with ΞPROV provenance and forks that inherit their parent's provenance.
  - Its surfaces have evidence tiers: documented, inferred, invented.
  - It maps all five Lattice dimensions, and Truth means machine-checked doctrine rules. It has an LLM critic that "never counts as provenance".
  - Its MVP is one Unbuilt promenade at one site.
  - *Core needs:* large binary artefacts (splats), streams with forking, rule-check claims, a judge that isn't evidence. All four are already in the target design [I].
- **Cow-tipping game:** not found (decision 13).

---

## 4. Cross-tenant capabilities

Abbreviations: **FC** Fermi Console · **SO** SimOps · **DPP** DPP Studio (incl. Regulatory Lens) · **RB** Rabble · **WD** Wild · **KP** kask pages · **HUD** Glasses · **PU/AR/AL** Pulse / Arborea / Architect Lens (design only).

| Capability (CAP) | Needed by | Notes |
| --- | --- | --- |
| Browser OAuth start and logout, `GET /api/auth/me` (14) | FC, SO, DPP, RB, WD, KP | FC uses a loopback redirect (`main.rs:1546`); web clients use cookies or `ferm_*` tokens |
| Device or API-token auth without a browser (14) | HUD, FC (paste key), DPP embed | HUD has none today |
| Allowance balance and per-workspace budget (12) | FC, DPP, SO and WD (`initial_budget`), RB | Replaces wallet, budget and billing |
| Owner-to-owner transfer and sale (12 → ACN, M4) | RB | `send_credits`, transfer, swarm-algorithm purchases |
| Agent catalogue and execute (1) | FC, SO, KP (via MCP), HUD, RB (wrapped) | One execute contract; KP uses JSON-RPC `/mcp/agents/:id` |
| Streaming execute (SSE) and rate-limit semantics (1) | FC | Send `Retry-After`; FC's pacing depends on it |
| Rosters: orchestra, auto-hire, hire/add (2/4) | FC, SO, DPP, WD | Merge hire, add and `POST agents` into one idempotent call |
| Apps: manifest, spawn and list workspaces, public schema (15) | FC, SO, DPP, WD | Schemas: `fermi-forecast/1`, `kask-simops/2`, `adaptogen-lab-regulatory/1`, `kask-wild/1` |
| Workspace documents as files, with history (15) | SO, DPP, WD, KP (adc-pk), FC (outputs, history, revert) | Canonical YAML documents per app |
| Messages: post, list, stream or poll (15) | FC, SO, DPP, WD | SSE (SO, WD) and polling (DPP) both in use |
| Typed actions plus action log; accept and reject (10/15) | SO, DPP, WD | DPP has 6 types, WD 4, SO uses `mutate_document` |
| Human decision queues (10) | FC (BayesOps, Loop 2 cascades), SO (pending actions), DPP (endorsement) | — |
| FPL and Monte Carlo kernel, client and server (13) | FC (local, via crate), SO (`/api/fpl/execute`) | Decision 2 and decision 3 |
| BayesOps, Shapley, Brier, calibration (6/13) | FC; WD (planned Brier badge) | — |
| Domain kernels (tenant-owned) | SO (cascade, dynamics, rheology), DPP (carbon arithmetic, unit basis) | `modules.md:459` asks whether to certify SO under the kernel process |
| Observations, SOSA ingestion (5) | SO, RB (flights, telemetry), WD (`log_observation`), PU, AR | The `sosa_*` tables are shared between SO and RB |
| Outcome observer (5/8) | FC (Polymarket), SO (measured batches) | — |
| Claims plus Lattice evaluation (7/8) | FC, SO, DPP, RB (species), HUD (card), AL (doctrine rules), AR | DPP is the strictest suite |
| Provenance stamps and evidence tiers (9) | DPP, HUD, RB, WD, AR, AL | One vocabulary; HUD and AL render it to people |
| External grounding tools (1/9) | FC (Polymarket), DPP (web search), RB and WD (GBIF, iNaturalist), RB (geocode) | Keys required (`BRAVE_SEARCH_API_KEY`) |
| Public unauthenticated publication (14/15) | DPP (passport + QR), RB (join token) | G20 signing (decision 5) |
| Sharing, teams, invites (2/14) | FC; RB (rabble invites, contacts) | FC deferred set; RB's are tenant-local |
| Push and notifications (14) | RB | Web push (VAPID) |
| Large binary artefacts (15) | RB (art, upload), AL (splats), HUD (camera frame, not wired) | Content-addressed artefacts (M1) |
| Cross-workspace delegation (4) | WD ↔ RB | Declared in `kask_wild.json`; `src/handlers/wild.rs:3` |
