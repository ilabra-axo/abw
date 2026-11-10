# Tenant notes

Narrative companion to `tenants.md` (endpoints, mechanically extracted). Written Nov 10, 2026 from the client sources and the other harvest files. Claims marked [I] are inferred and need confirming with the owner.

## Where the tenants are

| Tenant | Client source | Server side (legacy) | Notes |
| --- | --- | --- | --- |
| Fermi Console | `fermi/crates/fermi-console` (Rust desktop, 75k lines) | `handlers/forecasts.rs`, `polymarket.rs`, portfolios, teams, workspaces, BayesOps, relationship groups and cascades | 88 distinct endpoints. The largest and most complete tenant |
| SimOps | `~/kask/simops-*.js`, `kask-sim-client.js`, `~/kask/dynamics` (repo `ilabra-axo/kask`) | `/api/simops/*` plus generic workspaces | Uses the shared `~/kask/abw-client.js` (52 endpoints across the kask pages) |
| DPP Studio / Adaptogen Lab | `fermi/static/adaptogen-lab/{index,passport,embed-example}.html`; `~/kask/adaptogen` | `handlers/workspace/carbon.rs`, `bom_pricing.rs`, passport publication | Served by the legacy server itself. App definition in `fermi/apps/adaptogen-lab/dpp` |
| Regulatory Lens | `fermi/apps/adaptogen-lab/regulatory-lens` (manifest, rulesets, ontology, SKUs) | Generic app and workspace routes | Data-only app [I] |
| Rabble | `~/rabble/lib` (Flutter, 35k lines); compiled build at `fermi/rabble-web` | `handlers/rabble_workspace.rs`, `creatures/*`, swarms, flights, push | 73 endpoints, including `/api/wallet/transfer` and `/api/billing/*` (replaced by ACN) |
| Kask Wild | `~/kask/kask-wild-client.js` | Forecasts | Small |
| Glasses / HUD | `fermi/glasses` (2 files) | `hud_contract.rs`, `glasses_shell.rs` | A probe, not yet an app |
| ABW web UI | `fermi/templates`, `fermi/static/js` | Askama pages | Platform UI, not a tenant: 197 endpoints. The new core is API-first; this UI isn't ported as-is |
| abw CLI | `fermi/crates/abw-cli` | Apps, workspaces, messages | Developer tool; port when the API stabilises |
| Pulse, Arborea, architect lens, cow-tipping game | **Not found** on this machine | — | The owner should say where they live, or whether they're still planned |

## What each tenant claims (what the Lattice must check)

| Tenant | Claim | Ground truth | Lattice dimensions it exercises |
| --- | --- | --- | --- |
| Fermi Console | Probability forecasts with a condition and deadline; per-agent driver estimates (p5/p50/p95); Shapley credit | Polymarket resolution or manual resolution | Binding (native), Grounding (data tools), Truth (claim against the FPL output artifact) |
| SimOps | Process projections (dynamics, rheology, cascade) | Batch observations from the process | Grounding via `derivation`, Truth, Binding on projections |
| DPP Studio | Carbon footprint, bill of materials (BOM), signed passport | Supplier data and carbon-factor sources; external audit | Presence and Grounding end to end; the strictest suite (compliance weight) |
| Rabble | Species identity and ecology (GBIF) | GBIF records and observations | Grounding on external sources. Most state is game state (CRUD) |

## Minimum core surface per tenant (MVP)

**Common to all:** auth (`/api/auth/me`), agents (list, detail, execute and stream), apps (list, detail, spawn workspace), workspaces (detail, messages and stream, files, agents, hire), and health.

**Fermi Console (M3):**
- forecasts (CRUD, resolve, void, history and revert, timeline);
- portfolios;
- Polymarket (search, link, snapshot, check resolutions);
- the Fermi orchestra roster;
- BayesOps (state, pending accept/reject, refit);
- relationship groups and pending cascades (Loop 2 queue);
- shares, invites and annotations;
- the leaderboard and calibration.

`/api/wallet` is replaced by allowance.

**SimOps (M6):** `/api/simops/{dynamics,rheology,cascade,project}` and their models; workspace actions (`fork_state`, `compare`, `mutate_document`, `invoke_member`, accept/reject); observation sessions.

**DPP Studio (M6):** carbon and BOM pricing in the workspace, passport publication and verification (signed, hashed), and the embed.

**Rabble (M6):** creatures, swarms, flights, presence and push, and the feed. The 33 Rabble tables are tenant-owned CRUD; consider giving Rabble its own service that calls ABW for agents only. Billing and wallet are replaced by ACN.

## Needs owner decision

1. **Pulse, Arborea, the architect lens and the cow-tipping game:** where are they, and are they in MVP scope?
2. **Rabble:** a tenant inside the core, or its own service that uses ABW agents? It's mostly game CRUD (33 tables, 101 routes).
3. **The legacy web UI (197 endpoints):** which pages does the new core need a UI for at MVP? The agent catalogue, workspace view and claim review packets are likely [I].
4. **73 legacy `/api` routes that no client calls** (listed in `tenants.md`): each is either reached by agents and tools, external, or dead.
5. **Shopping and marketplace agents and routes:** a tenant, or dropped? Selling between owners moves to ACN either way.
