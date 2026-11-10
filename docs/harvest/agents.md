# Agent harvest — legacy Fermi / Agent Bestiary platform

| | |
|---|---|
| Source repo | `/home/ilabra/fermi` |
| Source commit | `a3cea516` (committed Thu Oct 8 11:40:15 2026 +0200) |
| Harvest date | 2026-11-10 |
| Scope | Read-only. The only write was this file. |
| Working-tree caveat | At harvest time the tree had **uncommitted changes in 35 files** that are not part of this harvest. They include `src/api_server.rs` (+46 net lines, hunks at 1455/2762/2995/3351) and `agent-bestiary/memory/src/store.rs`. Line numbers were read from the **working tree**. For `src/api_server.rs` lines above about 3351 (seeder `5790-6041`, routes `4141-4213`, `seed_cep_entities` `6346`), subtract roughly 46 to get the HEAD `a3cea516` position; between 2762 and 3351, subtract 18–28. Every other cited file, and all of `agents/curated/`, matches HEAD. |

## Method

1. **Cards.** I parsed all 104 `agents/curated/*/agent_card.json` with a throwaway script (kept outside the repo). For each card it pulled `agent_type`, `tier`, `status`, `system_prompt` size, `capabilities.mcp_tools[].name`, `mcp_servers`, `data_sources`, `requires_secrets`, the contract fields (`output_contract`, `input_contract`, `fermi_contract`, `simops_contract`), `dependencies`, `workflow_template.stages` and the sketch files next to each card.
2. **Taxonomy and port JSONs.** I cross-checked `agents/port_binding_expected.json` (103 keys), `agents/taxonomy_derived_expected.json` (101 keys) and `agents/taxonomy_proposals.json` (50 proposals) against the curated directory.
3. **Seeding.** I grepped for `seed`, `agent_card`, `INSERT INTO agents` in `src/`, `crates/`, `agent-bestiary/` and `migrations/`. **No migration inserts agent rows.** Agents reach the DB only through `seed_agents_to_database`, which upserts every card under `$AGENTS_DIR` (default `agents/curated`, `src/api_server.rs:2441`) on each boot (`src/api_server.rs:2461-2462`, `5790-6041`). Other agent definitions turned up as code or manifests:
   - test fixtures in `agent-bestiary/memory/src/seed.rs:145,208,274`, deleted from prod by `migrations/123_remove_test_seed_agents.sql`;
   - an embedded agent with no card, `forage_identify` (`src/handlers/wild.rs:246`, `:367`);
   - `TYPED_TIER_EXEMPT` names with no card (`src/workflows/agent_contract.rs:185-273`);
   - app manifests `apps/*.json` that auto-hire agents into tenant workspaces;
   - templates under `agents/templates/` (mirrored, with differences, in `docs/templates/`);
   - a second `coherence_evaluator` card at `agent-bestiary/coherence/agent_card.json`.
4. **Vocabulary.** Types come from `docs/architecture/ABW — Logical Architecture in Capability Terms.md` §"Agent types" (lines 54-67). I added two types: **Vote strategist**, and **Domain strategist** for app companions that act as domain-constrained MoE strategists. Tenants were assigned from tags (`fermi-orchestra`, `simops-orchestra`, `dpp-orchestra`, `kask-wild`, `rabble`, `fleet:weather`), from `apps/*.json` `auto_hire` lists, and from descriptions.
5. **Orchestration logic.** For every strategist and compound agent I grepped the slug and its tool names in `src/`, `crates/`, `agent-bestiary/` and `fermi-auth/`, then read the dispatch code. Each behaviour is tagged **[code]** (enforced or executed by Rust) or **[prompt-only]** (exists only as instructions to the LLM).
6. **Tool reality.** `src/agent_backend/weather_tools.rs:3804-3990` (`no_curated_card_declares_a_phantom_tool`) carries a ratchet list, `known_debt` (`:3897-3971`), of declared tools that have **no dispatch arm**. Agents marked "phantom" below draw on that list.

Conventions: `file:line` paths are relative to the repo root. **[I]** marks an inference. **"Prompt size"** is the `system_prompt` string only, as newline-delimited lines and characters. It excludes `prompt_template` and `workflow_template`.

### Counts

**By type (104 curated cards):**

| Type | n | Slugs |
|---|---|---|
| Expert | 71 | (see table) |
| Compound agent | 13 | ar_card_producer, ar_choreographer, bioreactor_modeler, delivery, dream_coordinator (platform), flight_coordinator, observability_coordinator, rabble_anchor_manager, rabble_curator, rabble_lifecycle_coordinator, shopping_assistant, social_media_studio, weather_oracle |
| Coherence strategist | 3 | cohere_and_coordinate (current), coherence_consultant (archived, superseded), intention_coordinator (Stage-0 sub-strategist, absorbed) |
| Pipeline strategist | 1 | pipeline_strategist |
| MoE routing strategist | 2 | moe_router_strategist (general), fermi (domain-constrained, fermi orchestra) |
| Debate strategist | 1 | debate_strategist |
| Vote strategist (new) | 1 | vote_strategist |
| Domain strategist / app companion (new) | 3 | simops_companion, dpp_companion, wild_companion |
| Platform agent | 9 | coherence_evaluator, companion_builder_coach, embedding_projector_guide, ontologist, performance_coach, publish_coach, stripe_billing, stripe_connect_advisor, xaman_ek |

That is 15 strategist-class agents including the 3 domain strategists, and 13 compound agents.

**By disposition:** `port` 17 · `port:tenant-*` 80 (simops 19, rabble 16, fermi 13, social-media 6, wild 6, ar 5, shopping 4, weather 4, dpp 2, adaptogen 1, regulatory-lens 1, glasses 1, kask-biotech 1, video 1) · `replaced:acn` 2 · `drop?` 5.

**Card `agent_type` (raw) vs. this classification.** The raw field is not a reliable type signal. 50 cards say `research`, and the rest are spread over `creative`, `meta`, `coordination`, `coherence`, `strategist`, `compound`, `composition`, `commerce`, `observability`, `companion`, `game` and `analytics`. Three examples:
- `weather_oracle` is `composition`.
- The three companions are `strategist`.
- `pipeline_strategist`, `moe_router_strategist`, `debate_strategist` and `vote_strategist` are `coordination`, not `strategist`.

The derived taxonomy (`scripts/taxonomy.py:181-187`) calls an agent *Composita* if it has any `dependencies.required`. That over-includes `simops_optimizer` and `specimen_minter` and misses most strategists, which have no required deps.

---

## Full inventory (all 104 curated agents)

Column notes:
- **Output contract?** `y` means the card has a `capabilities.output_contract` or an `output_contract.sketch.json` file. `partial` means only a `fermi_contract` or `simops_contract` (a domain finding shape, not a full output schema). The parenthesis says which.
- **External services.** The first entry is always the LLM provider and model. Search, image, data and publish services are derived from the tools and are in-process platform tools unless marked `MCP:`. `secret:` is a per-user `requires_secrets` entry. `ds:` is a card-declared data source; many are aspirational, with no tool behind them [I].

| # | slug | type | tenant/app | tools declared | external services / model | output contract? | prompt size | disposition | note |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `adaptogen_curator` | Expert | dpp/adaptogen | adaptogen_species_search, adaptogen_species_detail, adaptogen_compound_search, adaptogen_evidence_query, adaptogen_safety_check, adaptogen_drug_interaction_check, adaptogen_traditional_use_query, adaptogen_indication_search, adaptogen_genomic_markers, adaptogen_population_variants, adaptogen_compare_species, adaptogen_medicine_system_browse, execute_agent, search_knowledge, read_workspace_file, write_workspace_file, list_agents, gbif_species_search | anthropic/claude-sonnet-4-5-20250929, GBIF API, ds:kask_adaptogen_db, ds:PubMed E-utilities, ds:ClinicalTrials.gov API, ds:PubChem PUG-REST, ds:NCBI Taxonomy & GenBank, ds:GBIF Species API, ds:DrugBank Open Data | n | 77 L / 5855 ch | port:tenant-adaptogen | Tags 'compound-agent' but no required deps; 12 adaptogen_* tools are phantom (no dispatch arm, weather_tools.rs:3897 known_debt). Needs kask_adaptogen_db (Neon PG). card agent_type=research, tier=curated. |
| 2 | `adc_pk_oracle` | Expert | other (kask biotech/ADC) | execute_agent, web_search | anthropic/claude-sonnet-4-6, Brave Search (BRAVE_SEARCH_API_KEY) | n | 79 L / 3617 ch | port:tenant-kask-biotech | Phase-1 prior-knowledge reasoning; Phase-2 Chemprop model not wired [I]. card agent_type=research, tier=curated. |
| 3 | `anomaly_triager` | Expert | platform (observability) | query_anomalies, query_hitl_queue, classify_anomaly, route_to_hitl | anthropic/claude-sonnet-4-5-20250929 | n | 35 L / 2527 ch | port | Member of observability composition. card agent_type=observability, tier=curated. |
| 4 | `ar_avatar_renderer` | Expert | glasses/hud (AR) | scene_planner, interaction_designer, avatar_profile_loader | anthropic/claude-sonnet-4-5-20250929 | n | 54 L / 2722 ch | port:tenant-ar | All 3 tools phantom (known_debt). card agent_type=creative, tier=curated. |
| 5 | `ar_beacon` | Expert | glasses/hud (AR) + rabble | generate_image, edit_image, write_workspace_file, read_workspace_file, list_workspace_agents, execute_agent | anthropic/claude-sonnet-4-5-20250929, Gemini (GEMINI_API_KEY) | n | 123 L / 5688 ch | port:tenant-ar | H3 placement; image gen (Gemini). card agent_type=creative, tier=curated. |
| 6 | `ar_card_producer` | Compound agent | glasses/hud (AR) | generate_image, edit_image, write_workspace_file, read_workspace_file, list_workspace_agents, execute_agent | anthropic/claude-sonnet-4-5-20250929, Gemini (GEMINI_API_KEY) | n | 103 L / 5597 ch | port:tenant-ar | Declares 5-stage workflow_template (Intake→…→Delivery); no runtime executes stages. card agent_type=creative, tier=curated. |
| 7 | `ar_cartographer` | Expert | glasses/hud (AR) | write_workspace_file, read_workspace_file, list_workspace_agents, execute_agent, search_knowledge | anthropic/claude-sonnet-4-5-20250929 | n | 162 L / 6706 ch | port:tenant-ar | H3 grid naming. card agent_type=creative, tier=curated. |
| 8 | `ar_choreographer` | Compound agent | glasses/hud (AR) | write_workspace_file, read_workspace_file, list_workspace_agents, execute_agent, search_knowledge | anthropic/claude-sonnet-4-5-20250929 | n | 187 L / 7472 ch | port:tenant-ar | required dep ar_beacon (taxonomy=Composita); 3-stage workflow_template, prompt-only orchestration. card agent_type=creative, tier=curated. |
| 9 | `bioreactor_modeler` | Compound agent | simops (kask/Ambu) | get_observations, get_latest_observation, send_actuation, simops_cascade_forward, simops_kpi_compute, describe_session, list_active_sessions, execute_agent | anthropic/claude-sonnet-4-6 | n | 67 L / 2655 ch | port:tenant-simops | required deps simops_cascade+simops_predictor; send_actuation/get_latest_observation/list_active_sessions phantom (known_debt) — the actuation half cannot run. card agent_type=research, tier=curated. |
| 10 | `biotech_analyst` | Expert | fermi | execute_agent, search_ontology_terms, search_ontology_properties, get_ontology_analytics | anthropic/claude-sonnet-4-5-20250929, BioPortal, MCP:bioportal | partial (fermi_contract) | 78 L / 4807 ch | port:tenant-fermi | fermi_contract; BioPortal remote MCP (BIOPORTAL_API_KEY). card agent_type=research, tier=curated. |
| 11 | `bluesky_publisher` | Expert | social-media | create_post, upload_blob, resolve_handle, get_post, create_thread, fetch_og_metadata, read_workspace_file, list_workspace_agents | anthropic/claude-haiku-4-5-20251001, Bluesky AT Proto, secret:bluesky_handle, secret:bluesky_app_password | n | 42 L / 2251 ch | port:tenant-social-media | All 6 Bluesky tools phantom (known_debt). requires_secrets bluesky_handle/app_password. card agent_type=creative, tier=curated. |
| 12 | `carbon_accountant` | Expert | dpp/adaptogen (DPP Studio) | web_search, read_workspace_file | anthropic/claude-sonnet-4-6, Brave Search (BRAVE_SEARCH_API_KEY) | y (card, sketch) | 182 L / 10073 ch | port:tenant-dpp | dpp-orchestra; output contract sketch; arithmetic done in Rust (handlers/workspace/carbon.rs). card agent_type=research, tier=curated. |
| 13 | `cohere_and_coordinate` | Coherence strategist | platform (default strategist all workspaces) | solicit_agent_plan, evaluate_coherence, coherence_snapshot, get_workspace_messages, list_workspace_agents, execute_agent, read_workspace_file, write_workspace_file, propose_composition_change, record_coordination_observation, get_intention_map, declare_intention, check_conflicts, clear_intention, suggest_differentiation, emit_coherence_signal | anthropic/claude-sonnet-4-5-20250929 | n | 110 L / 8197 ch | port | THE default strategist (fermi-auth/src/teams.rs:17). See strategist section — most important card. card agent_type=coherence, tier=curated. |
| 14 | `coherence_consultant` | Coherence strategist (legacy) | platform | coherence_snapshot, get_workspace_messages, ontology_reader | anthropic/claude-sonnet-4-5-20250929 | n | 46 L / 2781 ch | drop? | [card status=archived] status=archived; superseded by cohere_and_coordinate (coherence.rs:415-417 'replaces the former coherence_consultant sub-call'). Keep freemium tier pricing idea. card agent_type=coherence, tier=curated. |
| 15 | `coherence_evaluator` | Platform agent | platform | evaluate_coherence, coherence_snapshot | anthropic/deterministic | n | 0 L / 0 ch | port | [card status=archived] executor=mcp, model 'deterministic', no prompt — a card wrapping the Rust TEC engine (agent-bestiary/coherence). status=archived but required dep of cohere_and_coordinate. A divergent duplicate card exists at agent-bestiary/coherence/agent_card.json. card agent_type=coherence, tier=system. |
| 16 | `companion_builder_coach` | Platform agent | platform | agent_template_loader, design_checklist | anthropic/claude-sonnet-4-5-20250929 | n | 39 L / 2442 ch | port | Onboarding; both tools phantom (known_debt). card agent_type=companion, tier=curated. |
| 17 | `comparator` | Expert | simops (also auto-hired by fermi_forecast) | — | anthropic/claude-sonnet-4-6 | partial (simops_contract) | 54 L / 2677 ch | port:tenant-simops | No tools; simops_contract extension:comparison. card agent_type=research, tier=curated. |
| 18 | `condition_forecaster` | Expert | other (kask-wild) | openweather_forecast, inat_observations, execute_agent | anthropic/claude-haiku-4-5-20251001, OpenWeather (OPENWEATHER_API_KEY), iNaturalist API | y (card) | 54 L / 2384 ch | port:tenant-wild | Brier-scored microclimate; OPENWEATHER_API_KEY. card agent_type=research, tier=curated. |
| 19 | `daily_puzzle` | Expert | other (demo/game) | puzzle_generator, streak_tracker | anthropic/claude-haiku-4-5-20251001 | n | 38 L / 2112 ch | drop? | Demo/template; both tools phantom; no tenant. card agent_type=game, tier=curated. |
| 20 | `deal_finder` | Expert | other (shopping) | — | anthropic/claude-sonnet-4-5-20250929 | n | 46 L / 1559 ch | port:tenant-shopping | Text-only sub-agent of shopping_assistant. card agent_type=research, tier=curated. |
| 21 | `debate_strategist` | Debate strategist | platform | list_workspace_agents, execute_agent, get_agent_calibration, search_knowledge, web_search | anthropic/claude-sonnet-4-5-20250929, Brave Search (BRAVE_SEARCH_API_KEY) | n | 59 L / 3840 ch | port | See strategist section. Orchestration prompt-only. card agent_type=coordination, tier=curated. |
| 22 | `delivery` | Compound agent | social-media (artist deck) | — | anthropic/claude-haiku-4-5-20251001 | n | 10 L / 750 ch | port:tenant-social-media | No tools declared but prompt tells it to read_workspace_file/execute_agent — prompt/tool mismatch. card agent_type=creative, tier=curated. |
| 23 | `dpp_companion` | Domain strategist (app companion / domain-constrained MoE) | dpp/adaptogen (DPP Studio) | list_workspace_agents, execute_agent, read_workspace_file, get_workspace_messages | anthropic/claude-sonnet-4-6 | n | 73 L / 5065 ch | port:tenant-dpp | agent_type=strategist; tag-based dispatch to dpp-orchestra; __ACTION__ grammar executed by client + workspace action endpoints. Not in port_binding/taxonomy expected json (newest card). card agent_type=strategist, tier=curated. |
| 24 | `dream_coordinator` | Compound agent (platform) | platform (Loop 1 dreaming) | execute_agent, consolidation_reader | openai/gpt-4o-mini | n | 12 L / 1235 ch | port | [card status=published] Orchestration is code: handlers/consolidation.rs resolves members from its deps; prompt is descriptive. card agent_type=coordination, tier=system. |
| 25 | `dream_narrator` | Expert | platform (Loop 1) | consolidation_reader, agent_profile_loader | anthropic/claude-haiku-4-5-20251001 | n | 36 L / 2901 ch | port | [card status=archived] status=archived on card but is the NARRATE member of dream_coordinator and is invoked by code (consolidation.rs:1298) — archived flag is wrong or ignored. card agent_type=meta, tier=curated. |
| 26 | `dyad_observer` | Expert | platform (observability) | query_timeline, query_dyad_state, query_episodes, query_persona_history | anthropic/claude-sonnet-4-5-20250929 | n | 35 L / 2337 ch | port | 2 of 4 tools phantom (query_episodes, query_persona_history). card agent_type=observability, tier=curated. |
| 27 | `embedding_broker` | Expert | other (shopping/embedding commerce) | get_shopping_profile, create_listing, list_marketplace, search_knowledge | anthropic/claude-haiku-4-5-20251001 | n | 41 L / 2179 ch | port:tenant-shopping | Marketplace sub-agent. card agent_type=commerce, tier=curated. |
| 28 | `embedding_projector_guide` | Platform agent | platform | projection_api, temporal_analysis, cluster_interpreter | anthropic/claude-sonnet-4-5-20250929 | n | 13 L / 1830 ch | port | All 3 tools phantom. card agent_type=research, tier=curated. |
| 29 | `enemy_sensor` | Expert | rabble | scan_nearby_creatures, gbif_species_search, execute_agent | anthropic/claude-haiku-4-5-20251001, GBIF API | n | 39 L / 2105 ch | port:tenant-rabble | Rabble system agent (auto-hired). card agent_type=research, tier=curated. |
| 30 | `energy_advisor` | Expert | simops | web_search | anthropic/claude-sonnet-4-6, Brave Search (BRAVE_SEARCH_API_KEY) | partial (simops_contract) | 345 L / 16716 ch | port:tenant-simops | Largest SimOps expert prompt. card agent_type=research, tier=curated. |
| 31 | `entity_investigator` | Expert | fermi | execute_agent | anthropic/claude-sonnet-4-5-20250929, ds:osint_sources | partial (fermi_contract) | 111 L / 5899 ch | port:tenant-fermi | OSINT; fermi_contract. card agent_type=research, tier=curated. |
| 32 | `equity_analyst` | Expert | fermi | execute_agent, fmp_company_profile, fmp_income_statement, fmp_balance_sheet, fmp_cash_flow, fmp_ratios, fmp_key_metrics, fmp_dcf, fmp_analyst_estimates, fmp_historical_price | anthropic/claude-sonnet-4-5-20250929, Financial Modeling Prep (FMP_API_KEY), ds:Financial Modeling Prep | y (card, sketch, fermi_contract) | 183 L / 9730 ch | port:tenant-fermi | 9 fmp_* tools dispatchable but not declarable (weather_tools.rs:3823-3827). FMP_API_KEY. card agent_type=research, tier=curated. |
| 33 | `eval_runner` | Expert | platform (observability) | run_evaluator_registry, query_eval_signals, query_eval_runs | anthropic/claude-sonnet-4-5-20250929 | n | 28 L / 2072 ch | port | Member of observability composition. card agent_type=observability, tier=curated. |
| 34 | `fermi` | MoE routing strategist (domain-constrained, fermi orchestra) | fermi | execute_agent, get_agent_calibration | anthropic/claude-sonnet-4-5-20250929 | n | 71 L / 6659 ch | port:tenant-fermi | Orchestra strategist (orchestras.rs:72); roster injected at runtime (orchestras.rs:1171-1185). See strategist section. card agent_type=meta, tier=system. |
| 35 | `fixture_context_agent` | Expert | fermi (sports/football) | — | anthropic/claude-sonnet-4-5-20250929, ds:weather_api, ds:venue_geodata, ds:tournament_fixture_list | partial (fermi_contract) | 44 L / 3588 ch | port:tenant-fermi | Factor X6; no tools. card agent_type=research, tier=curated. |
| 36 | `flavor_profiler` | Expert | other (kask-wild) | gbif_species_search, execute_agent | anthropic/claude-haiku-4-5-20251001, GBIF API | n | 56 L / 2835 ch | port:tenant-wild |  card agent_type=research, tier=curated. |
| 37 | `flight_coordinator` | Compound agent | rabble | execute_agent, write_workspace_file, read_workspace_file, search_knowledge | anthropic/claude-sonnet-4-5-20250929 | n | 69 L / 3753 ch | port:tenant-rabble | required navigator+naturalist; invoked by code (creatures/flights.rs:986,1550). card agent_type=creative, tier=system. |
| 38 | `football_analyst` | Expert | sports (fermi orchestra) | execute_agent, call_football_api | anthropic/claude-sonnet-4-5-20250929, API-Football (FOOTBALL_API_KEY), ds:api_football | y (card, sketch, fermi_contract) | 244 L / 14366 ch | port:tenant-fermi | Has per-field output contract prototype; FOOTBALL_API_KEY (API-Football v3). card agent_type=research, tier=curated. |
| 39 | `football_institution_agent` | Expert | sports (fermi orchestra) | execute_agent | anthropic/claude-sonnet-4-5-20250929, ds:fifa_big_count, ds:deloitte_football_money_league, ds:uefa_country_coefficient | partial (fermi_contract) | 41 L / 3245 ch | port:tenant-fermi | Factor X2; no tools. card agent_type=research, tier=curated. |
| 40 | `forage_scout` | Expert | other (kask-wild) | inat_observations, openweather_forecast, mycobank_lookup, gbif_species_search, execute_agent | anthropic/claude-haiku-4-5-20251001, iNaturalist API, OpenWeather (OPENWEATHER_API_KEY), MycoBank (MYCOBANK_API_KEY), GBIF API | n | 97 L / 5259 ch | port:tenant-wild | iNat, OpenWeather, MycoBank (MYCOBANK_API_KEY), GBIF. card agent_type=research, tier=curated. |
| 41 | `genome_profiler` | Expert | rabble | gbif_species_search, gbif_taxonomy_tree, ncbi_genome_search, execute_agent | anthropic/claude-haiku-4-5-20251001, GBIF API, NCBI E-utils (NCBI_API_KEY opt.) | y (card, sketch) | 69 L / 6208 ch | port:tenant-rabble | GBIF+NCBI (NCBI_API_KEY); in/out contract sketches. card agent_type=research, tier=curated. |
| 42 | `harvest_advisor` | Expert | other (kask-wild) | mycobank_lookup, gbif_species_search, execute_agent | anthropic/claude-haiku-4-5-20251001, MycoBank (MYCOBANK_API_KEY), GBIF API | n | 126 L / 6340 ch | port:tenant-wild | Safety layer; note edibility refusal now enforced in code in wild.rs. card agent_type=research, tier=curated. |
| 43 | `hud_field_scout` | Expert | glasses/hud | gbif_species_search, mycobank_lookup, inat_observations | anthropic/claude-sonnet-4-5-20250929, GBIF API, MycoBank (MYCOBANK_API_KEY), iNaturalist API | y (card) | 99 L / 8669 ch | port:tenant-glasses | Paired with glasses/hud_field_scout shell (generated by src/glasses_shell.rs). card agent_type=research, tier=curated. |
| 44 | `instagram_publisher` | Expert | social-media | create_media_container, check_container_status, publish_media, get_media_insights, get_account_info, list_recent_media, read_workspace_file, list_workspace_agents | anthropic/claude-haiku-4-5-20251001, Instagram Graph API, secret:instagram_token | n | 35 L / 1812 ch | port:tenant-social-media | All 6 Graph API tools phantom; requires_secrets instagram_token. card agent_type=creative, tier=curated. |
| 45 | `intention_coordinator` | Coherence strategist (Stage-0 sub-strategist, legacy) | platform | declare_intention, check_conflicts, get_intention_map, clear_intention, suggest_differentiation, emit_coherence_signal, list_workspace_agents, read_workspace_file, write_workspace_file | anthropic/claude-haiku-4-5-20251001 | n | 74 L / 3849 ch | drop? | Function absorbed into cohere_and_coordinate Stage 0; tools now real (coordination.rs) but its PROMPT is stale (file-based map, TTL, BUDGET_GATE, contradiction detection — none exist in code). Port the code, not this prompt. card agent_type=coherence, tier=system. |
| 46 | `keeper` | Expert | rabble | list_agents | anthropic/claude-sonnet-4-5-20250929 | n | 13 L / 872 ch | port:tenant-rabble | Rabble system agent; 13-line prompt. card agent_type=creative, tier=system. |
| 47 | `macro_data_agent` | Expert | fermi | web_search | anthropic/claude-sonnet-4-5-20250929, Brave Search (BRAVE_SEARCH_API_KEY), ds:world_bank, ds:undp_hdr | y (card, sketch, fermi_contract) | 119 L / 6092 ch | port:tenant-fermi | Factor X1; World Bank/UNDP via web_search (BRAVE_SEARCH_API_KEY). card agent_type=research, tier=curated. |
| 48 | `macro_forecaster` | Expert | fermi | execute_agent | anthropic/claude-sonnet-4-5-20250929, ds:macro_indicators | partial (fermi_contract) | 102 L / 5484 ch | port:tenant-fermi |  card agent_type=research, tier=curated. |
| 49 | `market_research` | Expert | fermi | execute_agent | anthropic/claude-sonnet-4-5-20250929 | partial (fermi_contract) | 52 L / 3747 ch | port:tenant-fermi | Also a template example of the same name with different content (agents/templates/examples/market_research). card agent_type=research, tier=curated. |
| 50 | `marketing_composer` | Expert | simops (kask marketing) | execute_agent | anthropic/claude-sonnet-4-6 | n | 63 L / 3541 ch | port:tenant-simops |  card agent_type=research, tier=curated. |
| 51 | `micro_patron_template` | Expert | other (creator-economy template) | pricing_calculator, agent_card_generator | anthropic/claude-sonnet-4-5-20250929 | n | 48 L / 2613 ch | drop? | Template/teaching card; both tools phantom; monetization pitch overlaps ACN billing. card agent_type=creative, tier=curated. |
| 52 | `moe_router_strategist` | MoE routing strategist | platform | select_agent, list_workspace_agents, execute_coordination_graph, execute_agent, search_knowledge, get_agent_calibration | anthropic/claude-sonnet-4-5-20250929 | y (card, sketch) | 87 L / 5654 ch | port | See strategist section. card agent_type=coordination, tier=curated. |
| 53 | `monte_carlo_sim` | Expert | fermi | fermi_execute_fpl, fermi_sensitivity_analysis | anthropic/claude-sonnet-4-6, Fermi FPL engine (internal) | n | 96 L / 3879 ch | port:tenant-fermi | Uses fermi_execute_fpl/fermi_sensitivity_analysis (real engine). card agent_type=research, tier=curated. |
| 54 | `naturalist` | Expert | rabble | gbif_species_search, list_agents | anthropic/claude-sonnet-4-5-20250929, GBIF API | n | 14 L / 1145 ch | port:tenant-rabble | Rabble system agent; 14-line prompt. card agent_type=creative, tier=system. |
| 55 | `navigator` | Expert | rabble | list_agents | anthropic/claude-sonnet-4-5-20250929 | n | 12 L / 823 ch | port:tenant-rabble | Rabble flight narrator (not xaman_ek 'navigator' role). Invoked by creatures/flights.rs:244. card agent_type=creative, tier=system. |
| 56 | `nba_analyst` | Expert | sports (fermi orchestra) | execute_agent | anthropic/claude-sonnet-4-5-20250929, ds:nba_api | partial (fermi_contract) | 124 L / 7426 ch | port:tenant-fermi | References nba_api but no tool. card agent_type=research, tier=curated. |
| 57 | `observability_coordinator` | Compound agent | platform (observability shelf) | execute_agent, query_eval_signals, query_anomalies, query_timeline, query_dyad_state, query_hitl_queue, get_workspace_messages, list_workspace_agents | anthropic/claude-sonnet-4-5-20250929 | n | 61 L / 2921 ch | port | required eval_runner+anomaly_triager+dyad_observer; prompt-only delegation. card agent_type=observability, tier=curated. |
| 58 | `observation_analyst` | Expert | simops (SOSA) | — | anthropic/claude-sonnet-4-5-20250929 | n | 37 L / 2635 ch | port:tenant-simops | No tools. card agent_type=research, tier=curated. |
| 59 | `ontologist` | Platform agent | platform (Loop 1) | — | openai/gpt-4o-mini | n | 13 L / 1427 ch | port | [card status=published] Used as bare LLMProvider by consolidation (consolidation.rs:337-352); prompt barely used [I]. card agent_type=meta, tier=system. |
| 60 | `performance_coach` | Platform agent | platform | agent_stats_api, ontology_analyzer, benchmark_comparator | anthropic/claude-sonnet-4-5-20250929 | n | 40 L / 2107 ch | drop? | [card status=archived] status=archived; all 3 tools phantom. card agent_type=meta, tier=curated. |
| 61 | `pipeline_strategist` | Pipeline strategist | platform | get_workflow_template, list_workspace_agents, execute_coordination_graph, execute_agent, search_knowledge | anthropic/claude-sonnet-4-5-20250929 | y (card, sketch) | 62 L / 4220 ch | port | See strategist section. card agent_type=coordination, tier=curated. |
| 62 | `preference_modeler` | Expert | other (shopping) | update_shopping_profile, get_shopping_profile, search_knowledge, write_workspace_file | anthropic/claude-haiku-4-5-20251001 | n | 33 L / 1819 ch | port:tenant-shopping |  card agent_type=analytics, tier=curated. |
| 63 | `prey_locator` | Expert | rabble | scan_nearby_creatures, gbif_species_search | anthropic/claude-haiku-4-5-20251001, GBIF API | n | 18 L / 1628 ch | port:tenant-rabble | Premium; dependencies null. card agent_type=research, tier=curated. |
| 64 | `product_scout` | Expert | simops | execute_agent, web_search | anthropic/claude-sonnet-4-6, Brave Search (BRAVE_SEARCH_API_KEY) | n | 57 L / 2792 ch | port:tenant-simops |  card agent_type=research, tier=curated. |
| 65 | `publish_coach` | Platform agent | platform | — | anthropic/claude-sonnet-4-5-20250929 | n | 19 L / 1328 ch | port | Card-review coach; no tools. card agent_type=meta, tier=curated. |
| 66 | `rabble_anchor_manager` | Compound agent | rabble | execute_agent, read_workspace_file, write_workspace_file, list_workspace_agents | anthropic/claude-sonnet-4-5-20250929 | n | 36 L / 1908 ch | port:tenant-rabble | agent_type=compound; invoked by code on anchor events (rabble_workspace.rs:1024, flights.rs:769). card agent_type=compound, tier=system. |
| 67 | `rabble_curator` | Compound agent | rabble | read_workspace_file, write_workspace_file, execute_agent, delegate_to_agent, query_beacons, h3_resolve, list_workspace_agents, search_knowledge | anthropic/claude-sonnet-4-5-20250929 | n | 89 L / 3783 ch | port:tenant-rabble | 4-stage workflow_template Resolve→Mint→Fly→Choreograph; delegate_to_agent; prompt-only orchestration. card agent_type=commerce, tier=curated. |
| 68 | `rabble_lifecycle_coordinator` | Compound agent | rabble | execute_agent, read_workspace_file, write_workspace_file, list_workspace_agents | anthropic/claude-sonnet-4-5-20250929 | n | 42 L / 2029 ch | port:tenant-rabble | agent_type=compound; invoked by code on lifecycle events (state.rs:1261, rabble_workspace.rs:1034). card agent_type=compound, tier=system. |
| 69 | `regulatory_lens_translator` | Expert | regulatory-lens (dpp/adaptogen) | web_search, read_workspace_file, write_workspace_file | anthropic/claude-sonnet-4-6, Brave Search (BRAVE_SEARCH_API_KEY) | y (card, sketch) | 202 L / 9960 ch | port:tenant-regulatory-lens | Primary agent of adaptogen_lab_regulatory app; dependencies null; grounding enforced in code (claim_evaluation.rs). card agent_type=research, tier=curated. |
| 70 | `regulatory_scanner` | Expert | simops | web_search | anthropic/claude-sonnet-4-6, Brave Search (BRAVE_SEARCH_API_KEY) | y (sketch) | 65 L / 2921 ch | port:tenant-simops | Has output_contract.sketch.json but capabilities.output_contract absent. card agent_type=research, tier=curated. |
| 71 | `reynolds_flock` | Expert | rabble | — | anthropic/claude-haiku-4-5-20251001 | n | 54 L / 2074 ch | port:tenant-rabble | Deterministic-math narration; dispatched by rabble_workspace.rs:826. card agent_type=research, tier=system. |
| 72 | `sensor_advisor` | Expert | simops | execute_agent, web_search | anthropic/claude-sonnet-4-6, Brave Search (BRAVE_SEARCH_API_KEY) | partial (simops_contract) | 319 L / 15944 ch | port:tenant-simops |  card agent_type=research, tier=curated. |
| 73 | `sentiment_analyzer` | Expert | fermi | execute_agent | anthropic/claude-sonnet-4-5-20250929, ds:sentiment_signals | partial (fermi_contract) | 53 L / 3943 ch | port:tenant-fermi | Also a template example of the same name. card agent_type=research, tier=curated. |
| 74 | `shopping_assistant` | Compound agent | other (shopping) | execute_agent, delegate_to_agent, get_shopping_profile, search_knowledge, list_workspace_agents, write_workspace_file, read_workspace_file | anthropic/claude-sonnet-4-5-20250929 | n | 57 L / 2538 ch | port:tenant-shopping | required preference_modeler+deal_finder; 4-stage workflow; no Rust references at all. card agent_type=commerce, tier=curated. |
| 75 | `sidestream_miner` | Expert | simops | web_search | anthropic/claude-sonnet-4-6, Brave Search (BRAVE_SEARCH_API_KEY) | partial (simops_contract) | 72 L / 3399 ch | port:tenant-simops |  card agent_type=research, tier=curated. |
| 76 | `simops_advisor` | Expert | simops | execute_agent, get_observations, describe_session | anthropic/claude-sonnet-4-6 | partial (simops_contract) | 138 L / 9076 ch | port:tenant-simops | 6-turn interview (prompt) — companion prompt says 'no interview' now; overlap. card agent_type=research, tier=curated. |
| 77 | `simops_cascade` | Expert | simops | execute_agent, simops_cascade_forward, simops_cascade_backward, simops_load_process, simops_write_observation | anthropic/claude-sonnet-4-6 | partial (simops_contract) | 49 L / 2575 ch | port:tenant-simops | Tools real (simops_tools.rs). card agent_type=research, tier=curated. |
| 78 | `simops_companion` | Domain strategist (app companion / domain-constrained MoE) | simops | execute_agent, mutate_document, fork_state, compare, invoke_member, annotate_schema, annotate, list_workspace_agents | anthropic/claude-sonnet-4-6 | y (card, sketch) | 284 L / 14708 ch | port:tenant-simops | Orchestra strategist (orchestras.rs:87); action grammar. See strategist section. card agent_type=strategist, tier=curated. |
| 79 | `simops_dynamics_runner` | Expert | simops | execute_agent | anthropic/claude-sonnet-4-6 | partial (simops_contract) | 85 L / 5252 ch | port:tenant-simops |  card agent_type=research, tier=curated. |
| 80 | `simops_narrator` | Expert | simops | execute_agent | anthropic/claude-sonnet-4-6 | partial (simops_contract) | 76 L / 4493 ch | port:tenant-simops |  card agent_type=research, tier=curated. |
| 81 | `simops_narrator_local` | Expert | simops | — | ollama/qwen2.5:7b | partial (simops_contract) | 73 L / 3384 ch | port:tenant-simops | Ollama qwen2.5:7b local-first variant; no tools. card agent_type=research, tier=curated. |
| 82 | `simops_optimizer` | Expert | simops | execute_agent, simops_optimize_scale, simops_optimize_single_input, simops_check_constraints, simops_write_actuation_plan | anthropic/claude-sonnet-4-6 | partial (simops_contract) | 43 L / 2355 ch | port:tenant-simops | required dep simops_predictor → taxonomy Composita, but behaviour is a solver expert. card agent_type=research, tier=curated. |
| 83 | `simops_predictor` | Expert | simops | execute_agent, simops_predictor_train, simops_predictor_forecast, simops_fetch_training_data, simops_write_observation | anthropic/claude-sonnet-4-6 | partial (simops_contract) | 42 L / 2501 ch | port:tenant-simops | OLS training tools. card agent_type=research, tier=curated. |
| 84 | `social_media_studio` | Compound agent | social-media | generate_image, edit_image, write_workspace_file, read_workspace_file, list_workspace_agents, execute_agent, create_media_container, publish_media, get_media_insights, create_bsky_post, upload_bsky_blob | anthropic/claude-sonnet-4-5-20250929, Gemini (GEMINI_API_KEY), Instagram Graph API, Bluesky AT Proto | n | 83 L / 3682 ch | port:tenant-social-media | ABW doc's canonical compound example. 5 publish tools phantom. card agent_type=creative, tier=curated. |
| 85 | `species_resolver` | Expert | rabble | gbif_species_search, search_knowledge, execute_agent, write_workspace_file, read_workspace_file | anthropic/claude-haiku-4-5-20251001, GBIF API | y (card, sketch) | 173 L / 6543 ch | port:tenant-rabble | GBIF; in/out contracts. card agent_type=research, tier=curated. |
| 86 | `specimen_minter` | Expert | rabble | generate_image, edit_image, write_workspace_file, read_workspace_file, execute_agent, mint_creature | anthropic/claude-sonnet-4-5-20250929, Gemini (GEMINI_API_KEY) | n | 80 L / 3404 ch | port:tenant-rabble | required dep species_resolver → Composita by taxonomy; mint_creature tool; Gemini image. card agent_type=creative, tier=curated. |
| 87 | `stripe_billing` | Platform agent | platform (billing) | create_connect_account, check_connect_status, set_pricing, create_checkout_session, charge_usage, record_usage, get_payout_balance, get_usage_summary, generate_client_api_key, list_workspace_agents, read_workspace_file, write_workspace_file | anthropic/claude-sonnet-4-5-20250929, Stripe (STRIPE_SECRET_KEY), secret:stripe_api_key | n | 40 L / 2514 ch | replaced:acn | All 9 Stripe tools phantom (known_debt); STRIPE_SECRET_KEY. card agent_type=meta, tier=system. |
| 88 | `stripe_connect_advisor` | Platform agent | platform (billing) | — | anthropic/claude-sonnet-4-5-20250929, secret:stripe_api_key | n | 47 L / 2585 ch | replaced:acn | Advisory only. card agent_type=meta, tier=curated. |
| 89 | `style_transfer` | Expert | social-media (artist deck) | — | anthropic/claude-haiku-4-5-20251001 | n | 9 L / 836 ch | port:tenant-social-media | Gemini 'Nano Banana' edit; no tools declared. card agent_type=creative, tier=curated. |
| 90 | `supply_chain_oracle` | Expert | simops + dpp | execute_agent, web_search | anthropic/claude-sonnet-4-6, Brave Search (BRAVE_SEARCH_API_KEY) | y (card, sketch, simops_contract) | 130 L / 6135 ch | port:tenant-simops | Shared by simops-orchestra and dpp-orchestra (scro/bom-query/1). Consider core/shared [I]. card agent_type=research, tier=curated. |
| 91 | `swarm_coordinator` | Expert | rabble | activate_formation | anthropic/claude-sonnet-4-5-20250929 | n | 38 L / 2822 ch | port:tenant-rabble | Name says coordinator but is an expert (Onto4MAT telemetry); auto-run by swarm_telemetry.rs:404. card agent_type=research, tier=curated. |
| 92 | `swarm_host` | Expert | rabble | gbif_species_search, list_agents | anthropic/claude-sonnet-4-5-20250929, GBIF API | n | 18 L / 1013 ch | port:tenant-rabble | Welcome narration; state.rs:1234. card agent_type=creative, tier=system. |
| 93 | `valuechain_mapper` | Expert | simops | execute_agent, web_search | anthropic/claude-sonnet-4-6, Brave Search (BRAVE_SEARCH_API_KEY) | n | 68 L / 3583 ch | port:tenant-simops |  card agent_type=research, tier=curated. |
| 94 | `video_analyst` | Expert | other (video/Reduct) | reduct_list_projects, reduct_get_project, reduct_get_transcript, reduct_create_reel, reduct_add_block | anthropic/claude-haiku-4-5-20251001, Reduct.video (Reduct API key) | y (card, sketch) | 93 L / 7360 ch | port:tenant-video | Reduct.video tools (dispatchable, not in known_debt). card agent_type=creative, tier=curated. |
| 95 | `vote_strategist` | Vote strategist | platform | list_workspace_agents, execute_agent, get_agent_calibration, search_knowledge | anthropic/claude-sonnet-4-5-20250929 | n | 59 L / 3841 ch | port | See strategist section. card agent_type=coordination, tier=curated. |
| 96 | `watermark` | Expert | social-media (artist deck) | — | anthropic/claude-haiku-4-5-20251001 | n | 10 L / 742 ch | port:tenant-social-media |  card agent_type=creative, tier=curated. |
| 97 | `weather_calibrator` | Expert | weather | weather_climatology, weather_station_observation, run_monte_carlo, run_sensitivity_analysis | anthropic/claude-sonnet-4-6, Open-Meteo/ERA5, Open-Meteo/station obs | y (card, sketch) | 131 L / 10173 ch | port:tenant-weather |  card agent_type=research, tier=curated. |
| 98 | `weather_ensemble_forecaster` | Expert | weather | weather_settlement_spec, weather_ensemble_forecast, weather_climatology, weather_station_observation | anthropic/claude-sonnet-4-5-20250929, Polymarket gamma API, Open-Meteo ensemble, Open-Meteo/ERA5, Open-Meteo/station obs | y (card, sketch) | 139 L / 9181 ch | port:tenant-weather | Open-Meteo ensembles/ERA5 (no key). card agent_type=research, tier=curated. |
| 99 | `weather_market_analyst` | Expert | weather | polymarket_weather_markets, polymarket_orderbook, weather_settlement_spec, web_search | anthropic/claude-sonnet-4-5-20250929, Polymarket gamma API, Polymarket CLOB, Brave Search (BRAVE_SEARCH_API_KEY) | y (card, sketch) | 139 L / 9480 ch | port:tenant-weather | Polymarket gamma/CLOB (no key). card agent_type=research, tier=curated. |
| 100 | `weather_oracle` | Compound agent | weather (+ fermi entry point) | execute_agent, weather_settlement_spec, polymarket_weather_markets, weather_ensemble_forecast, weather_climatology, weather_dispersion_fit, polymarket_orderbook | anthropic/claude-sonnet-4-6, Polymarket gamma API, Open-Meteo ensemble, Open-Meteo/ERA5, Polymarket CLOB | y (card, sketch, fermi_contract) | 241 L / 18713 ch | port:tenant-weather | agent_type=composition; 4-stage pipeline + adversarial challenge; challenge flags checked post-hoc in grounding_trust.rs:939-958. card agent_type=composition, tier=curated. |
| 101 | `wild_companion` | Domain strategist (app companion / domain-constrained MoE) | other (kask-wild) | execute_agent, inat_observations, mycobank_lookup, openweather_forecast, log_observation | anthropic/claude-sonnet-4-5-20250929, iNaturalist API, MycoBank (MYCOBANK_API_KEY), OpenWeather (OPENWEATHER_API_KEY) | y (card) | 59 L / 2553 ch | port:tenant-wild | agent_type=strategist; hardcoded member routing in prompt; __ACTION__ grammar; log_observation phantom. card agent_type=strategist, tier=curated. |
| 102 | `wild_narrator` | Expert | other (kask-wild) | — | anthropic/claude-haiku-4-5-20251001 | n | 29 L / 2344 ch | port:tenant-wild | No tools. card agent_type=meta, tier=curated. |
| 103 | `wing_segmenter` | Expert | rabble | segment_creature_wings, search_knowledge | anthropic/claude-sonnet-4-5-20250929, Gemini (GEMINI_API_KEY) | n | 16 L / 1148 ch | port:tenant-rabble | Gemini segmentation. card agent_type=creative, tier=curated. |
| 104 | `xaman_ek` | Platform agent | platform (navigator) | list_agents, search_knowledge, execute_agent, list_workspace_agents, read_workspace_file, evaluate_coherence, coherence_snapshot, build_output_contract, validate_agent_card, fleet_map, describe_agent, who_answers, agents_of_type | anthropic/claude-haiku-4-5-20251001 | n | 681 L / 41980 ch | port | 42k-char prompt; fleet digest injected (orchestras.rs:1176; executor.rs FLEET MAP); seeder warns if fleet tools missing (api_server.rs:5825-5846). card agent_type=meta, tier=system. |

---

## Strategists and compound agents: what they do and where the logic lives

This is the core of the harvest. Shared mechanisms first, because every strategist depends on them.

### 0. Shared strategist machinery (applies to every strategist)

- **How a strategist is seated.** `teams.coordination_strategist_id` stores the workspace's strategist. The default is `cohere_and_coordinate`, assigned on creation only when the column is NULL (`fermi-auth/src/teams.rs:17`, `:34-67`). Migration 211 backfilled the default into all 249 workspaces (`migrations/211_assign_default_coordination_strategist.sql:29,55-59`). An owner can change it through `PATCH` handlers using `COALESCE($2, coordination_strategist_id)` (`src/handlers/workspace/core.rs:949`, `src/handlers/teams.rs:87`). **[code]**
- **When the seated strategist runs.** It is invoked automatically only by the coherence shelf: `POST …/coherence` with `depth=recommendations|dream_notes` (`src/handlers/workspace/coherence.rs:365-387`, `:418-663`). That call costs 2 or 5 credits (`:221-248`). Otherwise a strategist runs only when @-mentioned in chat (`src/handlers/workspace/messages.rs:255-298`). **No message router dispatches ordinary turns to the seated strategist.** [code; the absence is inferred from grep — no other reader of `coordination_strategist_id` invokes an agent, I]
- **Strategist runs with tools.** `ToolAwareExecutor` and `PlatformToolRegistry::all()` give the strategist its tools. `current_agent_id` is set so authorisation gates work. The run gets a reserved parent episode, so delegations are parented (`coherence.rs:583-663`, `:601-627`). **[code]**
- **The strategist's own memory (Loop 1).** Before running, its prompt is enriched with KG context (`coherence.rs:520-544`). Afterwards its run is persisted as an episode tagged `coordination_session` and `workspace:<id>` with route `CallerNamed` (`coherence.rs:680-729`). **[code]**
- **Chat attribution.** The `coherence_update` message is attributed to whichever strategist actually ran, title-cased from its slug (`coherence.rs:876-900`). **[code]**
- **`list_workspace_agents`** returns name, type, description, tags, `simops_contract`, accepts, produces, `input_schema_id` and `output_schema_id` (`src/agent_backend/tools/domains/workspace.rs:1147-1197`). **It does not return provider/model or valence**, though vote_strategist's tool description promises provider/model (`agents/curated/vote_strategist/agent_card.json:22`) and cohere_and_coordinate's promises valence. Its SQL joins `agents a ON wa.agent_id = a.id` (`workspace.rs:1167`). The schema I can see defines only `agents.agent_id` (`migrations/010_add_adm_tables_and_dreaming.sql:15-16`), and I found no migration adding `agents.id`. **If that holds, this tool errors for every strategist.** [I — verify against the prod schema; `migrations/195_declare_the_ghost_schema.sql` documents prod-only columns]
- **`select_agent`** ranks candidates by Brier, cost, valence_fit and fidelity, with default weights 0.40/0.20/0.20/0.20 (`workspace.rs:225`, `:291-350`). Per-workspace weights come from `teams.selection_weights` (`:298-321`, mig 231), **but nothing writes that column** [code; the missing writer is grep-verified]. valence_fit rewards distance from the workspace's valence centroid, i.e. anti-homophily (`:415-445`, `:530-555`). Every call is logged to `select_agent_decisions` (`:636-660`). **[code]**
- **`execute_coordination_graph`** is a typed DAG executor with BFS levels, topology detection (pipeline / fan-out / hybrid), synthesis at fan-in and stop-on-first-failure (`src/agent_backend/coordination_graph.rs:114-220`). Open slots are filled by `select_agent` (`:426-479`). Synthesis protocols (`:316-346`):
  - `aggregation` and `cep_weighted` wrap outputs in `{members:[…]}`. **No log-odds extremisation is implemented**; weighting is "upstream".
  - `max_risk` picks the highest `risk`/`severity`.
  - `selection` and the default return `{candidates:[…]}` for the LLM to choose from.

  Without a template argument the tool auto-reads the seated strategist's `workflow_template` (`workspace.rs:760-800`). **No curated card declares `nodes`/`edges`** (all 13 `workflow_template`s carry only `stages`), so on the current corpus this tool always returns "This workflow_template has no typed nodes…" (`coordination_graph.rs:119-135`). **[code — executor exists but has no live input]**
- **`get_agent_calibration`** is a real tool (`src/agent_backend/tools/domains/observability.rs:384-418`) backed by `compute_agent_calibration` (`src/calibration.rs:51-60`). It reads `eval_signals.dimension='forecast_calibration'`, which resolved forecasts write (`src/handlers/forecasts.rs:1820-1830`). **[code]**
- **Phantom-tool ratchet.** A test asserts no card may declare an undispatchable tool except those in `known_debt` (`weather_tools.rs:3804-3990`). `get_workflow_template` is on that list (`:3949`). **[code]**
- **`capability_gates` / `min_tier` are never enforced.** They are "typed, persisted, exposed, and never compared against anything" (`src/command_registry.rs:17-18`). Every "premium-only stage" in a strategist card is therefore **[prompt-only]**.
- **`member_count_min`** (cohere 2, pipeline 2, MoE 2, debate 2, vote 3) is declared on the cards but has **no reader in Rust** (grep-verified). **[prompt-only/metadata]**

### 1. `cohere_and_coordinate` — Coherence strategist (default for every workspace)

**What it does (card).** The card is v2.0.0, a 110-line / 8.2k-char prompt at `agents/curated/cohere_and_coordinate/agent_card.json:6`, with a five-stage `workflow_template` at `:318`. It is the workspace conductor:
- **Stage 0, pre-flight intention coordination:** solicit each member's plan, compare members' `teammate_assignment` views, conflict-check, differentiate, emit IntentionAligns/IntentionConflicts.
- **Stage 1, assess:** coherence snapshot, messages, members with valence.
- **Stage 2, diagnose:** read the TEC principle scores P1–P7 against bands (<0.4 / 0.4–0.7 / >0.7) and an incoherence taxonomy that *protects* productive incoherence.
- **Stage 3, coordinate:** assign roles, resolve destructive conflict, sequence work, flag gaps, raise anti-convergence alerts, write `_coordination/brief.md`, and `record_coordination_observation` for each named agent.
- **Stage 4, tension audit** (only on `[COMPOSITION DREAMING — TENSION AUDIT]`): read consolidated memory, run a valence audit (arousal and valence spread under 0.25 means homophily), `propose_composition_change` without naming replacements, and cascade RSI observations into each member's episodic memory.

Model ladder: premium Sonnet 4.5 runs all stages, standard Haiku runs Stages 0–3, free OpenRouter runs 1–3. Capability gates: `composition_dreaming: premium`, `cascade_rsi: standard` (`:183`). Required dep: `coherence_evaluator` (`:312`).

**Where the logic lives, behaviour by behaviour:**

- **TEC measurement Γ(C) + P1–P7.** `ConversationObserver::observe`, then `SettlingEngine::settle`, then `snapshot` over the last 50 messages (`src/handlers/workspace/coherence.rs:250-290`). The engine lives in `agent-bestiary/coherence/crates/coherence-core`. Stored as `CoherenceEvaluation` (`:321-342`). **[code]**
- **Incoherence taxonomy** (destructive / productive-competitive / -analogical / -contradictory / coherent). Lives in `coherence_core::classify` (`agent-bestiary/coherence/crates/coherence-core/src/incoherence.rs:181-266`), with `is_productive` and `should_remedy` (`:114-126`). It is called before the strategist runs (`coherence.rs:297-300`) and injected as `incoherence_type`, `productive`, `should_remedy` and `homophily_risk` (`:303-319`). The query tells the LLM "do not re-derive it" (`:464-471`). **[code]**
  - ⚠ **Thresholds differ between prompt and code.** The code uses LOW=0.5, HIGH=0.6 and an optimal Γ band of 0.45–0.75 (`incoherence.rs:46-56`). The prompt's bands are 0.4 and 0.7 (card `:6`). The new core should take the code values.
- **Homophily from Γ (above band means homophily risk).** `TensionBand::Above` sets `homophily_risk` (`incoherence.rs:189-190`). **[code]**
- **Homophily from valence spread (arousal or valence spread < 0.25).** Exists only in the card prompt, the dream prompt text (`src/handlers/composition.rs:358-359`) and a tool *description* (`src/agent_backend/tools/domains/coordination.rs:1254`). **No Rust computes the spread.** `composition_dream_handler` only passes raw member valence JSON into the prompt (`composition.rs:311-334`). **[prompt-only]** (Related **[code]**: `select_agent`'s anti-homophily valence_fit, `workspace.rs:415-555`.)
- **Stage 0 floor: the platform solicits plans before the strategist runs.** `run_plan_floor` (`coherence.rs:58-184`) runs only at `depth=recommendations` (`:409-413`). It asks members concurrently with no tools (`src/plan_solicitation.rs:389-391`), capped at `MAX_PER_RUN=8` with `FRESHNESS_SECS=600` (`plan_solicitation.rs:86,99`). Outcomes are folded into prose for the strategist (`coherence.rs:440`, `:474`). **[code]**
- **`solicit_agent_plan` tool.** `coordination.rs:327-403` calls `plan_solicitation::solicit` (`plan_solicitation.rs:282-480`). It checks membership, shows the member its teammates' declared plans (`:487-560`), parses JSON and writes the intention as `Solicited`. **[code]**
  - ⚠ **`teammate_assignment` is requested from members but discarded.** The elicitation prompt asks for it (`plan_solicitation.rs:547`) and the card and shelf query tell the strategist to compare these across members (card `:6`; `coherence.rs:457-461`). But `Solicited::Recorded` carries only `intention_id`, `description` and `signal` (`plan_solicitation.rs:124-130`, `:466-479`), and the tool returns no assignment (`coordination.rs:371-383`). The raw reply survives only inside the member's `plan_solicitation` episode (`plan_solicitation.rs:565-600`). **The "compare division-of-labour views" behaviour is effectively [prompt-only].**
- **Intention provenance (self / solicited / inferred / unattributed) and suppressed inferred-vs-inferred overlap.** Source is derived from caller identity (`coordination.rs:292-297`). Duplication between two second-hand rows is suppressed (`src/intentions.rs:216-224`). Grounding readings come from `intentions::Grounding` (`intentions.rs:283-340`). **[code]**
- **Conflict signals.** Resource maps to `CONFLICT_ALERT`, dependency to `DEPENDENCY_WAIT`, duplication (cosine ≥ 0.82) to `OVERLAP_WARNING`, otherwise `CLEAR` (`intentions.rs:49`, `:126-152`, `:162-280`). **[code]**
  - ⚠ `BUDGET_GATE` (card) has **no implementation** [prompt-only]. Contradiction detection is **explicitly not implemented** (`intentions.rs:9-18`).
- **`suggest_differentiation` with grounding caveat**, structural rather than prescriptive (`coordination.rs:522-600`). **[code]**
- **`clear_intention`.** Completed targets become satisfied dependencies (`coordination.rs:481-520`). **[code]**
- **`emit_coherence_signal`.** Writes `workspace_intention_signals` and posts an `intention_signal` message so the TEC observer sees it (`coordination.rs:602-682`). **[code]**
- **Assign roles / resolve destructive conflict / sequence work / flag gaps / anti-convergence alert.** LLM judgement only. "Anti-convergence: Γ rose fast without P4" has **no rate-of-change computation** anywhere (grep for convergence/delta-Γ found only prompt text, `composition.rs:365`). The nearest code signal is `homophily_risk` (above band). **[prompt-only]**
- **Coordination brief `_coordination/brief.md`.** Written only if the LLM calls `write_workspace_file`; no Rust references `brief.md`. The brief *text*, i.e. the strategist's output, is always posted to chat as a `coherence_update` (`coherence.rs:856-926`). **[prompt-only for the file / code for the chat post]**
- **Cascade to member memory (Loop 3.A).** Two paths share one implementation, `coordination_note::deliver` (`src/coordination_note.rs:88-…`), which checks membership and dedupes on a `coordinator_observation` episode since `run_started_at` (`:115-130`):
  - **Model path:** `record_coordination_observation` with gate 1 (caller must be the seated strategist) and gate 2 (target must be a member) (`coordination.rs:684-778`).
  - **Platform floor:** after the strategist runs, the brief is delivered to every member except the strategist, unless the model already wrote a targeted note this run (`coherence.rs:754-854`).

  **[code]** Key facts for porting: in production the model path produced "0 of 3,576 episodes" (`coherence.rs:756-767`), which is why the platform floor exists. The floor runs only on shelf invocations, **not** on @-mention or composition-dream runs.
- **Composition dreaming trigger.** `POST /api/workspaces/:id/composition/dream` requires owner or admin. It builds a tension-audit prompt with member valence and the pending-proposal count, then posts it as an `@cohere_and_coordinate` agent invocation and charges 5 credits (`composition.rs:304-414`). **[code trigger / prompt-only content]**
  - ⚠ The prompt **hardcodes `@cohere_and_coordinate`** (`composition.rs:350`) and ignores the seated strategist, unlike the shelf (`coherence.rs:344-387`).
- **`propose_composition_change` (Loop 4).** Creates a pending `composition_versions` row and prefixes `[homophily detected]` when flagged (`coordination.rs:780-840`). The HTTP route does the same (`composition.rs:251-300`). **[code]** Quirks:
  - the tool **ignores** `member_agent_ids` / `member_weights` even though the card schema declares them (`coordination.rs:817-818` set `None`);
  - it hardcodes `proposed_by: "cohere_and_coordinate"` (`:820`);
  - it stores the rationale in `rejection_note` (`:823`).

  Accepting a roster-less proposal records agreement and changes nothing (`composition.rs:93-119`).
- **Stage 4 "read consolidated memory".** Works only via KG enrichment on the shelf path (`coherence.rs:502-544`) and the `coordination_session` episodes written there (`:680-729`). On the dream path (an @-invocation) enrichment depends on the messages handler [I]. **[code, partial]**
- **Tier gating** (Stage 4 premium, Stage 0 not on free). Gates are never enforced (`command_registry.rs:17-18`). **[prompt-only]**
- **Skip Stage 0 on analytical invocations.** The platform floor skips at `dream_notes` (`coherence.rs:404-413`). **[code for the floor / prompt for the model]**

### 2. `pipeline_strategist` — Pipeline strategist

**What it does (card,** 62 lines / 4.2k chars, `agents/curated/pipeline_strategist/agent_card.json:6`; output contract `abw/coordination-trace/1` at `:62`; `workflow_template.synthesis=pipeline` at `:38`**):**
1. Read the composition's stages via `get_workflow_template`.
2. Validate each seam as SCHEMA MATCH, LABEL MATCH, UNMATCHED or OPEN SLOT, and halt before spending if any seam is unmatched.
3. Execute stages in order with `execute_agent`, passing only the declared accepts.
4. On failure, stop and report; never fabricate a stage.
5. Report seam check, trace, final output, open slots, and an assessment of fit, including which other strategy would have fit.

**Where the logic lives:**
- **Read the pipeline.** `get_workflow_template` is **phantom**, with no dispatch arm (`weather_tools.rs:3949` known_debt). Step 1 cannot run as written. **[prompt-only / broken]**
- **Seam validation (label / entry-input / unmatched / open slot).** Implemented as a pure planner, `pipeline::plan` (`src/pipeline.rs:128-…`, `SeamStatus` at `:56-79`), with tests including the open-slot vacancy (`:518-540`). **But it has no production caller**: only `examples/plan_probe.rs:63` and tests use it (grep-verified). **[code exists, not wired]**
  - SCHEMA-ID matching (`input_schema_id == output_schema_id`) is **not** in `pipeline::plan`, which is label-only (`pipeline.rs:18-29`). It exists only as data from `list_workspace_agents` for the LLM. **[prompt-only]**
- **Execute in order / stop on failure / open slots / trace.** `execute_coordination_graph` would do this (`coordination_graph.rs:114-220`, stop-on-failure at `:182-193`, open slots at `:174-176`), but **only for `nodes`/`edges` templates, which no card has**. The strategist's own card only has `stages`. In practice ordering, filtering and stopping are **[prompt-only]** via `execute_agent`.
- **Output contract enforcement.** `pipeline_strategist` declares a card-level field contract that the execute path enforces (`src/handlers/loops.rs:785-795`). **[code]**
- **"Say when this is the wrong strategy."** **[prompt-only]**

### 3. `moe_router_strategist` — MoE routing strategist

**What it does (card,** 87 lines / 5.7k chars, `agents/curated/moe_router_strategist/agent_card.json:6`; `workflow_template.synthesis=selection` plus `selection.criteria {brier .4, cost .2, valence_fit .2, fidelity .2}` at `:350-351`; output contract at `:113`; gate `domain_constrained_moe: premium` at `:27`**):**
- **Stage 0, classify:** `list_workspace_agents`, then `select_agent` on the schema id, then `get_agent_calibration` (prefer score ≥ 0.65 with ≥ 10 resolved), then label/skill/history match. Decompose multi-domain queries.
- **Stage 1, route:** `execute_agent`.
- **Stage 2, synthesise:** selection / aggregation (calibration-weighted) / pipeline / max_risk / cep_weighted (log-odds extremisation).
- **Stage 3, record the routing decision** as an episode for Brier learning.
- **Domain-constrained mode:** members must conform to the composition's output contract.

**Where the logic lives:**
- **Candidate ranking by Brier / cost / valence / fidelity.** `select_agent` (`workspace.rs:251-660`) logs to `select_agent_decisions` (`:636-660`). **[code]**
- **Calibration lookup.** `get_agent_calibration` (`observability.rs:384-418`, `calibration.rs:51-60`). **[code]** The thresholds 0.65 / 10 / n<5 cold-start are **[prompt-only]**.
- **Fan-out and synthesis.** `execute_coordination_graph` implements selection, aggregation, max_risk and cep_weighted *shapes* (`coordination_graph.rs:316-346`), but nothing feeds it (no nodes/edges). **Calibration-weighted aggregation and cep_weighted log-odds extremisation exist in no Rust** (grep: no `extremi*`, `log-odds` only in attribution). **[prompt-only]**
- **Recording routing decisions for learning (Loop 4.B).** The reader half exists: on forecast resolution, episodes tagged `moe_routing_decision` from the last 7 days are annotated with `1 − Brier` (`src/handlers/forecasts.rs:1459-1530`). **Nothing writes episodes with the tag `moe_routing_decision`** (grep-verified: the only occurrences are the reader). The prompt tells the LLM to "record … via search_knowledge", which is a read tool. **[prompt-only; loop open]** `select_agent_decisions` is the only persisted routing trace **[code]**.
- **Per-workspace learned weights.** `teams.selection_weights` is read (`workspace.rs:298-321`) and never written. **[code read / no writer]**
- **Domain-constrained output-contract conformance.** Its own field contract is enforced on execute (`loops.rs:785-795`). Member conformance relies on envelope gates in `execute_agent` (`coordination_graph.rs:382-391` reads `envelope.validation`). **[code, partial]** The premium gate is not enforced **[prompt-only]**.

### 4. `debate_strategist` — Debate strategist

**What it does (card,** 59 lines / 3.8k chars, `agents/curated/debate_strategist/agent_card.json:6`; five-stage `workflow_template` at `:122` with `Exchange` stage `agent: null`, an open slot; output contract at `:46`**):**
1. State the crux as a falsifiable proposition.
2. Assign AFFIRMATIVE and NEGATIVE by expertise, not volunteering, and tell members the advocacy is assigned.
3. Run at least two rounds: independent openings, then responses to the strongest opposing point. Stop on repetition.
4. Judge: survived / refuted / unresolved, a verdict with confidence, and a falsification condition.
5. Preserve the strongest surviving objection from the losing side.

Calibration informs claim weight but never decides the debate.

**Where the logic lives:**
- **All five stages, assignment, independence of opening rounds, judging and objection preservation:** no Rust orchestration exists. References in code are only (a) the shelf comment that the strategist is now reachable when seated (`coherence.rs:349-352`, `:876-878`) and (b) a `pipeline::plan` *test fixture* using debate's stages (`pipeline.rs:518-540`). **[prompt-only]**
- **Tools it relies on:** `execute_agent`, `list_workspace_agents` (see the `a.id` caveat above), `get_agent_calibration`, `search_knowledge`, `web_search`. All are dispatchable. **[code]**
- **Isolation between AFF and NEG in round 1.** Not enforced; each `execute_agent` call is independent only because the LLM chooses not to pass the other's text [I]. **[prompt-only]**

### 5. `vote_strategist` — Vote strategist (new type)

**What it does (card,** 59 lines / 3.8k chars, `agents/curated/vote_strategist/agent_card.json:6`; four-stage `workflow_template` at `:106`; output contract at `:42`; `member_count_min: 3` at `:6`**):**
1. Frame one comparable question.
2. Poll members independently with the identical question; no member sees another's answer.
3. Name the aggregation rule *before* the tally — majority, confidence-weighted, calibration-weighted (cited) or median — and never switch after seeing answers.
4. Report the full tally, dissent with reasoning, and an agreement level.
5. Run an **independence check**: flag members sharing provider, model or framing as correlated, not corroborating.
6. State what would resolve a split.

**Where the logic lives:**
- **Polling, rule pre-commitment, aggregation, tally, dissent preservation, what-would-resolve.** No Rust. **[prompt-only]**
- **Correlated-voter detection.** Depends on provider/model from `list_workspace_agents`, which **does not return those fields** (`workspace.rs:1155-1195`). **[prompt-only, and its input data is missing]** The same doctrine is applied to *human* verifiers in code (`src/verification.rs:40-50`).
- **`member_count_min: 3`.** No reader. **[prompt-only]**

### 6. `fermi` — domain-constrained MoE strategist (Fermi orchestra)

**What it does (card,** 71 lines / 6.7k chars, `agents/curated/fermi/agent_card.json:7`**):** the meta-forecaster. It decomposes a question into 3–6 orthogonal drivers using Tetlock discipline (bucket-as-integer-set, broad reference class, recency, effective n, multipliers near 1). It maps each driver to a domain and routes to specialists from a **runtime-injected roster**, calling `get_agent_calibration` when a domain has several candidates. It emits a mandatory `routing_decision` per driver (candidates, selected, reason).

**Where the logic lives:**
- **Roster injection.** `inject_orchestra_context` appends a domain-grouped roster with calibration badges (Brier, n) to the prompt (`src/handlers/orchestras.rs:1171-1185`, `:1260-1390`, `:1399-1439`). It is called from `execution.rs:150` and `execution_stream.rs:110`, with a static fallback when the DB is empty. **[code]**
- **Orchestra membership as governed state.** `orchestra_members` (mig 180, SPEC_29). Curated cards with `fermi_contract` are granted `curated_seed` at boot (`src/api_server.rs:6003-6035`). Orchestra spec at `orchestras.rs:70-95`. **[code]**
- **Driver→specialist routing.** Executed by the **fermi-console** client, not the server: `select_agent_for_driver*`, `route_candidates` and domain detection (`crates/fermi-console/src/routing.rs:552-911`). **[code, client-side]**
- **Calibration feedback.** A resolved forecast writes `forecast_calibration` eval_signals per agent used (`forecasts.rs:1820-1830`). **[code]**
- **`routing_decision` output field.** No Rust parses it (grep: no reader in `src/` or `crates/`). **[prompt-only]**
- **Aggregation of driver multipliers into a forecast.** Done by the FPL Monte Carlo engine (`src/executor.rs`, `src/evaluator.rs`) over the FPL program the console builds [I]. **[code]**

### 7. `simops_companion` — domain strategist (SimOps app)

**What it does (card,** 284 lines / 14.7k chars, `agents/curated/simops_companion/agent_card.json:6`; output contract `:313`; 11 optional deps `:457`**):** a conversational process-design companion. It emits `__ACTION__ … __END_ACTION__` JSON blocks in six types: `edit_process` (auto/ask), `invoke_agent` (with `render_as`), `fork_variation`, `compare_variations` (narrated by comparator), `declare_sosa_contract` and `annotate`. It is told to declare SOSA contracts proactively, keep stage output/input names consistent, and route to specialists by `simops_contract.role` / `extension_task` discovered via `list_workspace_agents` rather than by name.

**Where the logic lives:**
- **Action protocol endpoints.** Append-only `workspace_action_log` and auto/ask/pending confirmation, with `force_ask` overriding auto during alpha (`src/handlers/workspace/actions.rs:1-33`, `:159-261`). Six routes: mutate_document, fork_state, compare, invoke_member, annotate_schema, annotate (`actions.rs:10-22`). Accept/reject at `:744-870`. **[code]**
- **Parsing `__ACTION__` blocks from the companion's reply.** **Not in this repo.** It happens in the kask client (`src/handlers/apps.rs:1013-1019` serves the grammar via `GET /api/apps/:slug/schema`). **[external client]**
- **Card tools `mutate_document`, `fork_state`, `compare`, `invoke_member`, `annotate_schema`, `annotate`.** All **phantom** as tools (`weather_tools.rs:3950-3955` known_debt). The REST endpoints exist; the MCP tools do not. **[prompt-only as tools]**
- **Role-based specialist routing.** `list_workspace_agents` returns `simops_contract` specifically for this (`workspace.rs:1182-1186`). A test asserts no hardcoded routing (`orchestras.rs:1898-1901`). Orchestra strategist: `orchestras.rs:87`. **[code for data / prompt for decision]**
- **Field contract enforced on execute** (`loops.rs:785-795`). **[code]**
- **Alpha "every edit_process is ask".** `force_ask` (`actions.rs:173-179`). **[code]**

### 8. `dpp_companion` — domain strategist (DPP Studio, Adaptogen Lab)

**What it does (card,** 73 lines / 5.1k chars, `agents/curated/dpp_companion/agent_card.json:6`**):**
- Discovers its fleet by the `dpp-orchestra` tag and derives roles from tags (regulatory/claims, pricing/bom, carbon/lca).
- Reads the four DPP documents before answering.
- Emits `__ACTION__` blocks: `evaluate_claims`, `price_bom`, `calculate_carbon`, `compare_lenses`, `render_lens`, `flag_divergence`.
- Distinguishes **free reads** (compare/render over stored evaluations) from **paid runs**, and scopes runs narrowly.
- Reads provenance honestly: `model_inference` vs `tool_verified` / `tool_no_match`, `not_evaluated`, `needs_expert`, `synthetic_seed`.
- Refuses to estimate unsourced numbers.

**Where the logic lives:**
- **Action endpoints.** `…/actions/render_lens|compare_lenses|evaluate_claims|price_bom|flag_divergence|calculate_carbon` (`src/api_server.rs:4171-4203`). Handlers: `handlers/workspace/lens_actions.rs`, `claim_evaluation.rs`, `bom_pricing.rs` and `carbon.rs`; carbon arithmetic is done in Rust (`handlers/workspace/mod.rs:10-13`). The action types are admitted by migs 234/236/237 (`api_server.rs:1396-1422`). **[code]**
- **The companion itself.** No Rust references the slug (grep: zero hits). Tag-based routing, the cost discipline and the provenance reading are **[prompt-only]**. The provenance stamps it reads are **[code]** (`grounding_trust`, per the manifest `data_honesty` block in `apps/adaptogen-lab/regulatory-lens/manifest.json`).
- **Card hygiene.** Missing from `port_binding_expected.json` and `taxonomy_derived_expected.json`, the newest card [I]. Its tags include `moe` and `companion`.

### 9. `wild_companion` — domain strategist (kask-wild)

**What it does (card,** 59 lines / 2.6k chars, `agents/curated/wild_companion/agent_card.json:6`; output contract `:90`**):**
- Routes user intents to fixed members by name: forage_scout + condition_forecaster, harvest_advisor ("safety first"), flavor_profiler, wild_narrator.
- Emits YAML-ish `__ACTION__` blocks `log_observation`, `update_goal` and `annotate_location`.
- Is honest about identification uncertainty.

**Where the logic lives:**
- **Routing table.** Hardcoded in the prompt. **[prompt-only]**
- **`log_observation`.** Phantom tool (`weather_tools.rs:3970`). **[prompt-only]**
- **Identification and the safety refusal.** Moved to code: `identify_specimen` with a platform-constant safety directive and an embedded mycologist prompt, agent id `forage_identify` (`src/handlers/wild.rs:228-300`, `:246`, `:367`, `:413-446`). **[code]** — and it is *not* this card.
- **Rabble → Wild cross-workspace delegation.** `POST /api/creatures/:id/forage` (`src/handlers/creatures/agent_modules.rs:1870-1900`). **[code]**

### 10. `intention_coordinator` — legacy Stage-0 coherence sub-strategist

**What it does (card,** 74 lines / 3.8k chars, `agents/curated/intention_coordinator/agent_card.json:6`**):** collects intentions, detects duplication, contradiction, dependency and resource conflicts, and emits CLEAR / OVERLAP_WARNING / CONFLICT_ALERT / DEPENDENCY_WAIT / BUDGET_GATE. It keeps the map in `_coordination/intention_map.json` with a 5-minute TTL and feeds IntentionAligns/IntentionConflicts to TEC.

**Where the logic lives:**
- **Tools, conflict detection, signals and TEC feed.** Real now (`src/agent_backend/tools/domains/coordination.rs`, `src/intentions.rs`, mig 210 at `api_server.rs:1240-1242`). **[code]**
- **Prompt drift.** The file-based map was "never wired" (`src/loop_model.rs:320-321`, and per cohere's own card). TTL pruning, `BUDGET_GATE` and contradiction detection are not implemented (`intentions.rs:9-18`). **[prompt-only / stale]**
- **Attribution quirk.** `emit_coherence_signal` attributes its chat post to sender `intention_coordinator` whichever strategist called it (`coordination.rs:664`). **[code]**

### 11. `coherence_consultant` — archived coherence strategist

The card is 46 lines, `status: archived`. It offers a free coherence index, paid recommendations (2 cr) and dream notes (5 cr). **The tiering survives in code** as the `depth` parameter `index|recommendations|dream_notes` with prices 0/2/5 (`coherence.rs:213-248`). The consultant sub-call itself was replaced by direct invocation of the seated strategist (`coherence.rs:415-417`). **[code for tiering / card superseded]**

### Compound agents

**`social_media_studio`** (83 L, `agent_type creative`; `workflow_template` Brief → Create(style_transfer) → Caption → Publish(delivery) at `:294`; required style_transfer + watermark). The ABW doc's canonical compound. It generates images, delegates style and watermark when those members are present, writes per-platform captions, **never publishes without explicit confirmation**, and publishes via instagram_publisher / bluesky_publisher or built-in tools.
- Code: none beyond tool dispatch. `create_bsky_post`, `upload_bsky_blob`, `create_media_container`, `publish_media` and `get_media_insights` are **phantom** (`weather_tools.rs:3956-3960`), and so are both publisher agents' tools.
- The confirmation gate, solo/team mode and voice adaptation are **[prompt-only]**. Image generation (Gemini) is **[code]**. Listed as a composition pattern in `src/bin/agent-mcp-server.rs:611`.

**`weather_oracle`** (241 L / 18.7k ch, `agent_type composition`; required weather_ensemble_forecaster + weather_calibrator; output contract `:257`). Pipeline: Frame (bucket = integer set) → Forecast → Calibrate (**halt on negative Brier skill**) → Price, then an adversarial challenge: settlement-station mismatch, edge vs uncertainty, measured vs prior corrections, a **centre check** (`|ensemble − market centre| ≤ predictive sd`, computed not judged) and short-lead sanity (halt if the gap exceeds 10 pp).
- Stage ordering and the halts are **[prompt-only]**, executed by the LLM via `execute_agent`.
- The challenge flags are **audited post-hoc in code** as a cross-check over stored episodes (`src/grounding_trust.rs:915-958`) for `centre_gap_within_predictive_sd` and `edge_exceeds_calibration_uncertainty`. **[code — detection, not prevention]**
- The probability line is parsed from its output (`src/assertions.rs:748`, `:1092-1100`) and its field contract is enforced on execute (`loops.rs:785-795`). **[code]**
- The weather and Polymarket tools are real (`src/agent_backend/weather_tools.rs`).

**`dream_coordinator`** (12 L, system tier; stages Embed → Cluster → Extract(ontologist) → Summarise → Narrate(dream_narrator)). **Orchestration is code:**
- members are resolved by `produces` label from the card's declared deps (`src/handlers/consolidation.rs:280-320`);
- the extraction LLM is built from the ontologist's card (`:322-352`);
- DBSCAN clustering is in `agent-bestiary/memory/src/consolidation.rs:6,14`;
- the batch worker uses the same pipeline but resolves `ontologist` by name (`agent-bestiary/consolidate/src/main.rs:225-258`).

The prompt is descriptive only. **[code]**

**`observability_coordinator`** (61 L; required eval_runner, anomaly_triager, dyad_observer). Routes monitoring questions to its three members or answers directly from its own read tools. The description claims it is auto-attached to every workspace via the Observability shelf. Only one code reference, a comment at `src/handlers/eval.rs:376`; I found no auto-attach code [I]. **[prompt-only orchestration]**

**`rabble_curator`** (89 L, `agent_type commerce`; stages Resolve(species_resolver) → Mint(specimen_minter) → Fly(ar_beacon) → Choreograph(ar_choreographer)). Manages collections, flight operations, swarm events and fees. Code references are only `pipeline.rs` docs and tests (entry-input example, `:31-36`) plus the typed-tier exemption list. **[prompt-only]**

**`shopping_assistant`** (57 L; required preference_modeler + deal_finder). Intake → Research(deal_finder) → Learn(preference_modeler, after ~5 interactions) → Marketplace(embedding_broker). **Zero Rust references.** **[prompt-only]** The shopping-profile tools are dispatchable [I — not in known_debt].

**`ar_card_producer`** (103 L): Intake → Marker Generation → Video Generation → Scene Planning(ar_avatar_renderer) → Delivery(delivery). **`ar_choreographer`** (187 L; required ar_beacon): Intake → Resolution(ar_cartographer) → Compilation. Both are **[prompt-only]**, with no references beyond the typed-tier exemption.

**`flight_coordinator`** (69 L; required navigator + naturalist). Generates 20–50 waypoints at species speeds and delegates habitat narrative and species behaviour. **Triggered by code:** `rabble_workspace::dispatch_rabble_action(…, "flight_coordinator", "flight_plan", …)` from `/fly` (`src/handlers/creatures/flights.rs:1469-1665`) and the (dead-code) `/api/flights/plan` (`:832-1015`). The JSON plan is parsed with a fallback skeleton (`:995-1003`). Delegation to navigator and naturalist is **[prompt-only]**; triggering and plan persistence are **[code]**.

**`rabble_anchor_manager` / `rabble_lifecycle_coordinator`** (36 / 42 L, `agent_type compound`). Event-driven: the code dispatches `anchor_departing`, `anchor_transferred` and `participant_joined` queries (`flights.rs:755-775`; `rabble_workspace.rs:1010-1045`; `src/handlers/creatures/state.rs:1250-1270`). Both are auto-hired into every rabble workspace (`rabble_workspace.rs:31-43`). Event triggering is **[code]**. Their delegation to ar_beacon and swarm_coordinator is **[prompt-only]**.

**`bioreactor_modeler`** (67 L; required simops_cascade + simops_predictor). Monitors SOSA (Brix, T, OD600), decides harvest (OD600 8/10 AU, Brix plateau < 0.05 %/h) and issues actuation. `send_actuation`, `get_latest_observation` and `list_active_sessions` are **phantom** (`weather_tools.rs:3913-3915`). The thresholds and the actuation are therefore **[prompt-only / cannot execute]**; `simops_cascade_forward` and `simops_kpi_compute` are real.

**`delivery`** (10 L). The front of the artist deck: it should execute style_transfer, then watermark. **It declares no tools** yet its prompt instructs `read_workspace_file` / `execute_agent`. **[prompt-only, tool mismatch]**

---

## Needs owner decision

### Proposed drops (`drop?`) — conservative, proposals only

| slug | reason |
|---|---|
| `coherence_consultant` | Archived and superseded by `cohere_and_coordinate`. Its freemium tiering already lives in code (`coherence.rs:213-248`). |
| `intention_coordinator` | Function absorbed into cohere Stage 0 and the coordination tools. Its prompt is stale (file map, TTL, BUDGET_GATE). **Port `src/intentions.rs`, `src/plan_solicitation.rs` and the coordination tools, not this card.** |
| `performance_coach` | Archived; all 3 tools phantom. |
| `daily_puzzle` | Demo/template, no tenant, both tools phantom. |
| `micro_patron_template` | Creator-economy teaching card, tools phantom, overlaps ACN monetisation. |

### Replaced
- `stripe_billing`, `stripe_connect_advisor` → `replaced:acn`. `stripe_billing`'s 9 tools are all phantom anyway (`weather_tools.rs:3961-3969`).

### Unknowns / decisions

1. **Archived flags vs. reality.** `coherence_evaluator` (a required dep of the default strategist) and `dream_narrator` (invoked by consolidation) are `status: archived` on their cards. The `AgentCard` struct has no top-level `status` (`src/agent_backend/agent_card.rs:12-27`), and the seeder hardcodes `status: "published"` (`src/api_server.rs:5902`) with an upsert that overwrites status (`agent-bestiary/memory/src/store.rs:699`). So "archived" is probably ignored and these agents are live [I]. Decide which archived cards are genuinely retired.
2. **Unseen agents referenced only in code.** Each needs a port / drop decision and has no card:
   - `forage_identify`: embedded prompt in `src/handlers/wild.rs:246`, grounding contract in `src/grounding_trust.rs:2706-2740`.
   - `notebook_runner`, `polymarket_analyst`, `swarm_telemetry_analyst`, `trend_scout`: only in `TYPED_TIER_EXEMPT` (`src/workflows/agent_contract.rs:185-273`), with no card [I: retired cards].
   - `goal_tracker`: referenced by `apps/kask_wild.json` initial files.
   - `efra_forensic`: a third-party Fermi orchestra member (`crates/fermi-console/src/main.rs`, `roster.rs`).
   - `label_generator`, `info_card_generator`, `cold_chain_forecast`: named in the regulatory-lens manifest suite and roadmap.
3. **Duplicate / divergent definitions.** Decide which copy is canonical:
   - `agent-bestiary/coherence/agent_card.json` vs. `agents/curated/coherence_evaluator/agent_card.json` (differ: `tier`, tool name `get_coherence_snapshot` vs `coherence_snapshot`).
   - `agents/templates/**` vs. `docs/templates/**` (all files differ).
   - Template examples `market_research` and `sentiment_analyzer` vs. curated agents of the same name.
4. **Shared experts across tenants.** `supply_chain_oracle` (simops + dpp), `comparator` (simops + fermi_forecast auto_hire), `monte_carlo_sim`, `ar_beacon` (AR + rabble). Decide core vs. per-tenant.
5. **Tenant for `adc_pk_oracle`, `video_analyst`, `shopping_*`, the AR suite.** No app manifest owns them. I assigned provisional tenants (`kask-biotech`, `video`, `shopping`, `ar`).
6. **Phantom-tool agents.** These cards declare integrations that never ran (`known_debt`, `weather_tools.rs:3897-3971`). Port the card, but decide whether to build the tools or strip the declarations:
   - adaptogen_curator (12 tools)
   - bluesky_publisher and instagram_publisher (all tools)
   - AR renderer
   - bioreactor_modeler actuation
   - companion_builder_coach, embedding_projector_guide, dyad_observer (partial)
   - stripe_billing
   - wild_companion `log_observation`
   - simops_companion action tools
   - pipeline_strategist `get_workflow_template`
7. **`taxonomy_proposals.json`** covers 50 of 104 cards and includes the template agents `YOUR_AGENT_NAME` and `risk_monitor`. **`port_binding_expected.json` and `taxonomy_derived_expected.json`** lack `dpp_companion`, and the latter also lacks `carbon_accountant` and `regulatory_lens_translator`. Decide whether these fixtures are ported or regenerated.

---

## Surprising or important

1. **Only `cohere_and_coordinate` has real orchestration code; the other four platform strategists are almost entirely prompt.**
   - pipeline, MoE, debate and vote became *reachable* only once the shelf started reading the seated strategist (`coherence.rs:344-364`). When invoked they run as an LLM with `execute_agent`. Their distinctive behaviours (seam halting, calibration-weighted / cep_weighted synthesis, assigned advocacy, independent polling, pre-committed aggregation rules, correlated-voter flags) **exist only in prompt text**.
   - The two code assets built for them are unwired: `pipeline::plan` has no production caller, and `execute_coordination_graph` needs `nodes`/`edges`, which no card declares.
   - Treat the prompts as the spec, and these two modules as the starting point for making them real.
2. **Several cohere_and_coordinate behaviours were moved from prompt to code after production showed the prompt alone did nothing.**
   - The member-memory cascade produced "0 of 3,576 episodes" and now has a platform floor (`coherence.rs:754-854`).
   - Stage 0 plan solicitation now has a floor too (`coherence.rs:389-413`).
   - The incoherence taxonomy is a tested function (`incoherence.rs`), and the strategist is now persisted and enriched for Loop 1 (`coherence.rs:502-544`, `:665-729`).
   - **The new core must keep these "judgement is the model's, bookkeeping is the platform's" floors.** Porting the card alone would regress to the dead loop.
3. **Prompt/code divergences to resolve when porting `cohere_and_coordinate`:**
   - TEC band thresholds: 0.4/0.7 in the prompt vs. 0.45/0.75 plus LOW 0.5 / HIGH 0.6 in code.
   - The `teammate_assignment` comparison is asked for but the data is dropped (`plan_solicitation.rs:547` vs `:124-130`).
   - The valence-spread homophily audit is not computed anywhere.
   - `BUDGET_GATE` and anti-convergence rate detection are not implemented.
   - `propose_composition_change` drops `member_agent_ids` / `member_weights` and hardcodes `proposed_by`.
   - The composition-dream endpoint hardcodes `@cohere_and_coordinate` instead of the seated strategist (`composition.rs:350`).
4. **`list_workspace_agents` may be broken for every strategist.** It joins on `agents.id` (`workspace.rs:1167`); the schema shows only `agents.agent_id`. It also omits provider/model (vote_strategist's independence check) and valence (cohere's Stage 4). [I — verify]
5. **Two MoE learning loops are half-built.**
   - `moe_routing_decision` episodes are annotated on resolution (`forecasts.rs:1459-1530`) but never written.
   - `teams.selection_weights` is read by `select_agent` but never written. Loop 4.B is open on both ends.
6. **Capability gates and tier ladders are decorative.** Every "premium-only stage" (cohere Stage 4, MoE domain-constrained mode) is unenforced (`src/command_registry.rs:17-18`). The same goes for `member_count_min`.
7. **App companions depend on an external client.** `simops_companion`, `dpp_companion` and `wild_companion` emit `__ACTION__` blocks that **this repo never parses**; the kask / Studio client does. The server provides only the REST action endpoints and the grammar via `/api/apps/:slug/schema`. Harvest the grammars from `apps/*.json` `schema_json` together with the cards.
8. **Compound agents split cleanly into two kinds:**
   - **code-triggered:** dream_coordinator (fully code), the rabble trio (flight_coordinator, rabble_anchor_manager, rabble_lifecycle_coordinator, triggered by game events), swarm_coordinator and reynolds_flock (auto-dispatch);
   - **prompt-only workflow_templates:** social_media_studio, shopping_assistant, rabble_curator, ar_card_producer, ar_choreographer, observability_coordinator, weather_oracle.

   No runtime executes any card's `workflow_template.stages`. 13 cards declare them; they are documentation plus input to the unwired `pipeline::plan`.
9. **Safety logic left the prompts.** Wild edibility refusal and the identification prompt are platform constants in `src/handlers/wild.rs`. weather_oracle's challenge flags are audited by SQL cross-checks (`grounding_trust.rs:915-958`). When porting those cards, port the code guards too, or the safety regresses to prompt-only.
10. **There are no agent seeds in migrations.** Every agent is card → DB via the boot-time upsert, which also sets ownership to the earliest admin, grants Fermi `curated_seed` membership to any card with `fermi_contract`, and seeds CEP KG entities from `fermi_contract.seed_facts` (`api_server.rs:5800-6041`, `seed_cep_entities` at `:6346`). Those side effects must be reproduced in the new core's seeding.
