# Owner decisions from the harvest

One checklist of every decision the harvest raised, ordered by when it blocks work. Each item links to its evidence. Tick an item by writing the answer next to it (or say it in chat); decisions that change the design become ADRs.

Recommendations come from the reviewing agents and are marked *Rec.*

## Before M1 (event store): none block

M1 is pure substrate (`abw-canon`, `abw-ledger`) and needs no tenant or kernel decision.

## Before M2 (agents, episodes, allowance)

- [x] **D1. Agent cards to drop.** **Decided 2026-11-11: drop all seven** (each is archived, superseded, demo-only or replaced by ACN). `coherence_consultant`, `intention_coordinator` (port its code, not its card), `performance_coach`, `daily_puzzle`, `micro_patron_template`. `stripe_billing` and `stripe_connect_advisor` are replaced by ACN. *Rec.* drop all seven. [agents.md](agents.md#needs-owner-decision)
- [x] **D2. Archived flags.** **Decided 2026-11-11: port both as active.** `coherence_evaluator` and `dream_narrator` are marked archived but are live dependencies. *Rec.* port both as active. [agents.md](agents.md#unknowns--decisions) item 1
- [ ] **D3. Agents with no card.** **Decided 2026-11-11: port `forage_identify`, `goal_tracker`, `kask_advisor`, `label_generator`, `info_card_generator`, `cold_chain_forecast`.** What exists to port:
  - `kask_advisor`: a full card at `~/kask/agents/kask_advisor/` (M2, with the curated import).
  - `forage_identify`: a prompt embedded in legacy `src/handlers/wild.rs:246`, plus a grounding contract (`src/grounding_trust.rs:2706-2740`). It becomes a card with the Wild tenant (M6).
  - `goal_tracker`, `label_generator`, `info_card_generator`, `cold_chain_forecast`: **name only, no prompt or card anywhere**. They appear in `apps/kask_wild.json:191`, the DPP composition comment and the regulatory-lens manifest's `demos` list. Each needs a new card written from its tenant's spec, with that tenant (M6).
  - Still open: `efra_forensic` (appears only as a test fixture name in legacy) and the four retired names (`notebook_runner`, `polymarket_analyst`, `swarm_telemetry_analyst`, `trend_scout`). *Rec.* drop all five. `forage_identify`, `goal_tracker`, `efra_forensic`, `kask_advisor` (default chat agent on 10 kask pages, lives in `~/kask/agents`), `label_generator`, `info_card_generator`, `cold_chain_forecast`, plus four retired names. Port, write a card, or drop? [agents.md](agents.md#unknowns--decisions) item 2; [tenants-notes.md](tenants-notes.md#1-decisions-for-the-owner) item 10
- [ ] **D4. Duplicate cards.** **`coherence_evaluator` decided 2026-11-11 (ADR-0007): the curated card is canonical;** the bestiary card's theory section is kept as documentation. Still open: whether `agents/templates` or `docs/templates` is the source. [agents.md](agents.md#unknowns--decisions) item 3
- [x] **D5. Phantom tools.** **Decided 2026-11-11: strip the declarations when porting; re-add each tool with its tenant.** Cards that declare tools which never ran (adaptogen_curator ×12, Bluesky and Instagram publishers, the AR renderer, bioreactor actuation, ...). Build the tools, or strip the declarations when porting? *Rec.* strip now and re-add with each tenant. [agents.md](agents.md#unknowns--decisions) item 6
- [x] **D6. Allowance unit and grants.** **Decided 2026-11-11 (ADR-0006): compute units, not money. Grants: onboarding, a monthly free tier topped up to a cap, and ad-hoc operator grants.** Policy values (amounts, cap, CU prices) are still to be set before M2 closes.
- [x] **D6a. Coherence agents.** **Decided 2026-11-11 (ADR-0007):** `cohere_and_coordinate` is the only coherence agent; there is no separate coach; the platform delivers coordination observations to current members' episodic memory.

## Before M3 (Lattice and Fermi Console)

- [ ] **D7. Triangular semantics.** The console treats p5/p95 as true percentiles, but the executor uses them as min/max. Every forecast's numbers change with this choice. [modules.md](modules.md#needs-owner-decision) item 1; [tenants-notes.md](tenants-notes.md#1-decisions-for-the-owner) item 3
- [ ] **D8. One Sobol implementation, seeded.** `src/sensitivity.rs` (unseeded) or `posterior-reg/src/whatif.rs` (seeded Saltelli)? *Rec.* the seeded one. [modules.md](modules.md#needs-owner-decision) item 3
- [ ] **D9. DBSCAN `min_samples`:** keep legacy's off-by-one, or match the standard definition? [modules.md](modules.md#needs-owner-decision) item 2
- [ ] **D10. Shapley:** keep the 16-player cap; should `reconstruction_error` refuse rather than only record? Require `agent_id` on claims instead of roster prefix matching? [modules.md](modules.md#needs-owner-decision) items 9–11
- [ ] **D11. Fermi Console scope at M3.** The 60-endpoint forecasting core with collaboration panels hidden (*Rec.*), or all 93. [tenants-notes.md](tenants-notes.md#1-decisions-for-the-owner) item 1
- [ ] **D12. The console links the legacy `fermi` crate** to run simulations locally. *Rec.* the console depends on the new certified kernel crate instead. [tenants-notes.md](tenants-notes.md#1-decisions-for-the-owner) item 2
- [ ] **D13. `grounding_trust.rs` field contracts** (8,979 lines): port them as data, or discard them with the ladder? [modules.md](modules.md#needs-owner-decision) item 4
- [ ] **D14. Routes to drop** (38 routes in 9 groups: the old agent-server router, the debug probe, the knowledge-graph read API, ontology history, notebooks, generic shares, swarm telemetry sessions, Rabble dashboard duplicates, one-off admin repairs). *Rec.* drop all; the underlying data and stores are kept. [routes.md](routes.md#decisions-for-the-owner)
- [ ] **D15. Tables to drop or reclassify** (AKP ×4, `voice_assets`, `flight_telemetry`, `ar_choreographies`, marketplace ×3, views; trust-vs-CRUD seams for `composition_versions`, semantic rules, BayesOps fits, security audit logs). [tables.md](tables.md#3-needs-owner-decision)

## Before M5 (coordination, strategists, loops)

- [ ] **D16. Valence-homophily guard:** compute the 0.25 spread in code, use the tension band, or drop it? [modules.md](modules.md#needs-owner-decision) item 7
- [ ] **D17. Γ band calibration:** production Γ is 0.97, above the 0.45–0.75 band. Recalibrate, or treat it as an observer defect? [modules.md](modules.md#needs-owner-decision) item 8
- [ ] **D18. Loop 2:** the consensus audit defect and the persona-version deadlock. [modules.md](modules.md#needs-owner-decision) items 5–6
- [ ] **D19. Dead crates and modules:** `coherence-api`, `coherence-protocols`, `agent-bestiary/consolidate`, `src/report/`, the REPL, `evaluator-sotopia` and `evaluator-character`? [modules.md](modules.md#needs-owner-decision) item 14
- [ ] **D20. Shared experts:** `supply_chain_oracle`, `comparator`, `monte_carlo_sim` and `ar_beacon` are used by several tenants. Core or per-tenant? [agents.md](agents.md#unknowns--decisions) item 4

## Before M6 (tenants)

- [ ] **D21. Rabble:** its own service calling the core for auth, agents and allowance (*Rec.*), or inside the core. Is Rabble active? It has an open "empty agent payloads" bug from May 31. [tenants-notes.md](tenants-notes.md#1-decisions-for-the-owner) items 6–7
- [ ] **D22. DPP and Regulatory Lens are one tenant.** Is signed, hashed passport publication in its MVP? *Rec.* yes. Items 4–5
- [ ] **D23. Kask Wild** calls two actions that don't exist (`update_goal`, `annotate_location`): implement or remove? Item 8
- [ ] **D24. Kask pages:** the pages that don't use ABW are out of scope (*Rec.*). Keep the chat widget on the 12 that do? Items 9–10
- [ ] **D25. Glasses/HUD:** a tenant now, or deferred until it has rendered once (*Rec.* defer)? Item 11
- [ ] **D26. Workspace deletion** under append-only: archive or tombstone. Item 16
- [ ] **D27. Rabble peer-to-peer transfers** move to ACN in M4; confirm Rabble's MVP has no transfers. Item 15

## Missing tenants

- [ ] **D28. Pulse, Arborea and Architect Lens** exist as design documents only (`fermi/docs/folio`, `~/Downloads/*.pdf`). *Rec.* post-MVP, but record their substrate needs now. **The cow-tipping game wasn't found:** confirm it exists or remove it. "Pulse" already names an agent message hop in legacy, so pick one meaning. Items 12–14

## Harness hygiene

- [ ] **D29.** `tenants.md` (regex) undercounts. Rabble calls 123 endpoints, not 73; DPP about 22, not 2. `tenants-notes.md` is authoritative for counts; fix `harvest_tenants.py` or retire it? Item 17
