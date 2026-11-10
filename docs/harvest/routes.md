# Legacy HTTP route harvest

- Source: `fermi` at `a3cea516` (working tree has uncommitted edits to the scanned files; line numbers may drift from HEAD); generated 2026-11-10 by `harvest_routes.py`.
- Method: every `.route(` in `src/api_server.rs`, `src/api/server.rs` parsed with balanced parentheses; disposition by ordered regex rules over handler path and route path (first match wins). Rules are in the script.
- Caveats: rule-based classification is a first pass [I]; a route is a *surface*, not a feature: the capability it serves is decided in the register. Routes mounted via `.nest`/`.merge` keep their local path (e.g. under `/rabble`).

## Counts by disposition

| Disposition | Routes |
| --- | --- |
| `tenant:rabble` | 101 |
| `CAP-14` | 84 |
| `tenant:fermi-console` | 67 |
| `CAP-15` | 63 |
| `CAP-01` | 54 |
| `ops` | 44 |
| `CAP-06` | 35 |
| `CAP-05` | 27 |
| `CAP-02` | 25 |
| `ui` | 18 |
| `replaced:acn` | 15 |
| `crud:misc` | 14 |
| `CAP-04` | 13 |
| `tenant:simops` | 11 |
| `tenant:dpp` | 7 |
| `CAP-03` | 7 |
| `drop?` | 5 |
| `CAP-09` | 4 |
| `tenant:shopping` | 4 |
| `CAP-11` | 2 |
| `CAP-10` | 1 |
| **total** | **601** |

## By handler module

| Module | Routes | Dispositions |
| --- | --- | --- |
| `workspace` | 58 | CAP-15 44, tenant:dpp 6, CAP-04 3, CAP-05 2, CAP-02 2, tenant:simops 1 |
| `creatures` | 55 | tenant:rabble 54, ops 1 |
| `pages` | 37 | ui 18, tenant:fermi-console 3, CAP-01 3, CAP-06 3, CAP-15 2, CAP-05 2, CAP-14 1, CAP-03 1, ops 1, tenant:rabble 1, CAP-09 1, CAP-11 1 |
| `agents` | 31 | CAP-01 21, ops 6, CAP-14 3, tenant:fermi-console 1 |
| `admin` | 23 | ops 21, replaced:acn 2 |
| `forecasts` | 20 | tenant:fermi-console 20 |
| `social` | 19 | crud:misc 9, tenant:rabble 4, CAP-14 4, CAP-05 2 |
| `observatory` | 15 | CAP-05 11, CAP-02 3, CAP-06 1 |
| `loops` | 15 | CAP-06 14, CAP-04 1 |
| `apps` | 12 | CAP-15 10, ops 1, CAP-02 1 |
| `auth` | 12 | CAP-14 12 |
| `bayesops` | 12 | tenant:fermi-console 11, tenant:simops 1 |
| `relationships` | 12 | CAP-06 6, tenant:fermi-console 5, CAP-10 1 |
| `teams` | 11 | CAP-02 11 |
| `invites` | 10 | CAP-14 9, tenant:fermi-console 1 |
| `profile` | 10 | CAP-14 10 |
| `eval` | 9 | CAP-05 8, CAP-14 1 |
| `orchestras` | 9 | CAP-04 9 |
| `kg` | 8 | CAP-01 8 |
| `notebooks` | 8 | CAP-15 5, CAP-01 1, tenant:fermi-console 1, ops 1 |
| `governance` | 8 | tenant:rabble 4, CAP-14 4 |
| `ontology` | 7 | CAP-01 4, tenant:fermi-console 3 |
| `collab` | 7 | CAP-02 3, tenant:fermi-console 2, CAP-14 1, CAP-05 1 |
| `simops` | 7 | tenant:simops 7 |
| `xaman` | 7 | CAP-14 7 |
| `rabble_workspace` | 7 | tenant:rabble 7 |
| `dashboard` | 7 | tenant:rabble 4, crud:misc 3 |
| `a2a` | 6 | CAP-14 6 |
| `consolidation` | 6 | CAP-06 6 |
| `specimen` | 6 | tenant:rabble 6 |
| `contracts` | 6 | CAP-03 6 |
| `lifecycle` | 6 | CAP-01 6 |
| `polymarket` | 6 | tenant:fermi-console 6 |
| `marketplace` | 6 | tenant:shopping 4, CAP-14 2 |
| `swarm_telemetry` | 6 | CAP-14 6 |
| `observations` | 6 | CAP-14 5, CAP-05 1 |
| `(inline)` | 6 | drop? 4, ops 1, CAP-01 1 |
| `agent_wallet` | 5 | replaced:acn 5 |
| `users` | 5 | CAP-14 5 |
| `wizard` | 5 | CAP-01 3, CAP-15 1, crud:misc 1 |
| `composition` | 5 | ops 3, CAP-02 1, CAP-06 1 |
| `forecast_benchmark` | 5 | tenant:fermi-console 5 |
| `rabble_chat` | 5 | tenant:rabble 5 |
| `misc` | 4 | ops 1, drop? 1, CAP-01 1, CAP-14 1 |
| `beacons` | 4 | tenant:rabble 4 |
| `billing` | 4 | replaced:acn 4 |
| `shares` | 4 | tenant:fermi-console 2, CAP-14 2 |
| `impersonation` | 4 | ops 3, CAP-14 1 |
| `swarm_algorithms` | 4 | tenant:rabble 4 |
| `ground` | 3 | CAP-09 3 |
| `metrics` | 3 | ops 2, CAP-01 1 |
| `qr_codes` | 3 | tenant:rabble 2, tenant:dpp 1 |
| `wallet` | 3 | replaced:acn 3 |
| `forecast_git` | 3 | tenant:fermi-console 3 |
| `annotations` | 3 | tenant:fermi-console 3 |
| `pending_cascades` | 3 | CAP-06 2, tenant:fermi-console 1 |
| `admin_rbac` | 3 | ops 3 |
| `push` | 3 | tenant:rabble 3 |
| `mcp` | 2 | CAP-14 2 |
| `dreaming_maturity` | 2 | CAP-06 2 |
| `composition_evolution` | 2 | CAP-02 2 |
| `ecology` | 2 | crud:misc 1, tenant:rabble 1 |
| `simops_benchmark` | 2 | tenant:simops 2 |
| `agent_funding` | 1 | CAP-01 1 |
| `fleet` | 1 | CAP-02 1 |
| `rounds` | 1 | CAP-14 1 |
| `bestiary` | 1 | CAP-01 1 |
| `gates` | 1 | CAP-11 1 |
| `ops` | 1 | CAP-02 1 |
| `execution` | 1 | CAP-01 1 |
| `execution_stream` | 1 | CAP-01 1 |
| `evolution` | 1 | CAP-01 1 |
| `wild` | 1 | CAP-15 1 |
| `economics` | 1 | replaced:acn 1 |
| `rbac_self_check` | 1 | CAP-14 1 |
| `streams` | 1 | tenant:rabble 1 |
| `get(|| async { Redirect` | 1 | tenant:rabble 1 |

## All routes

| Method | Path | Handler | Where | Disposition | Note |
| --- | --- | --- | --- | --- | --- |
| GET | `/` | `handlers::pages::landing` | `src/api_server.rs:2776` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/dpp/:token` | `handlers::workspace::dpp_publish::public_dpp_redirect` | `src/api_server.rs:2780` | `tenant:dpp` |  |
| GET | `/api/dpp/p/:token` | `handlers::workspace::dpp_publish::public_dpp_handler` | `src/api_server.rs:2784` | `tenant:dpp` |  |
| GET | `/api/dpp/p/:token/qr` | `handlers::workspace::dpp_publish::public_dpp_qr_handler` | `src/api_server.rs:2788` | `tenant:dpp` |  |
| GET | `/aspiration` | `handlers::pages::aspiration` | `src/api_server.rs:2792` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/catalogue` | `handlers::pages::catalogue` | `src/api_server.rs:2793` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/fermi-console/install` | `handlers::pages::install_page` | `src/api_server.rs:2799` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/fermi-console/install.sh` | `handlers::pages::install_script` | `src/api_server.rs:2800` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/fermi-console/download` | `handlers::pages::fermi_console_download` | `src/api_server.rs:2807` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/apps` | `handlers::pages::apps_catalogue_view` | `src/api_server.rs:2811` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/apps/:slug` | `handlers::pages::app_detail_view` | `src/api_server.rs:2812` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/docs` | `handlers::pages::docs_view` | `src/api_server.rs:2813` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/docs/:slug` | `handlers::pages::docs_view` | `src/api_server.rs:2814` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/agent/:agent_id` | `handlers::pages::agent_detail` | `src/api_server.rs:2815` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/agent/:agent_id/ontology` | `handlers::pages::ontology_view` | `src/api_server.rs:2816` | `CAP-01` | Remember / ontology |
| GET | `/api/health` | `handlers::misc::health` | `src/api_server.rs:2820` | `ops` |  |
| GET | `/api/debug/startup` | `handlers::misc::debug_startup` | `src/api_server.rs:2821` | `drop?` | demo/debug |
| GET | `/api/geocode` | `handlers::misc::geocode_search_handler` | `src/api_server.rs:2822` | `CAP-01` | Remember / ontology |
| GET | `/api/forecasts/public` | `handlers::forecasts::public_forecasts_handler` | `src/api_server.rs:2824` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/invites/by-token/:token` | `handlers::invites::get_invite_by_token_handler` | `src/api_server.rs:2832` | `CAP-14` | Isolate tenants / auth |
| GET | `/invites/:token` | `handlers::pages::invite_landing_view` | `src/api_server.rs:2842` | `CAP-14` | Isolate tenants / auth |
| GET | `/mcp/agents/:agent_id` | `handlers::mcp::mcp_agent_manifest` | `src/api_server.rs:2844` | `CAP-14` | Connect and protect |
| POST | `/mcp/agents/:agent_id` | `handlers::mcp::mcp_agent_rpc` | `src/api_server.rs:2848` | `CAP-14` | Connect and protect |
| GET | `/a2a/:slug/agent-card.json` | `handlers::a2a::agent_card_handler` | `src/api_server.rs:2852` | `CAP-14` | Connect and protect |
| POST | `/a2a/:slug/:method` | `handlers::a2a::method_dispatch_handler` | `src/api_server.rs:2866` | `CAP-14` | Connect and protect |
| POST | `/v1/ground/runs` | `handlers::ground::open_run_handler` | `src/api_server.rs:2875` | `CAP-09` | Trace provenance |
| POST | `/v1/ground/runs/:run_id/tools/:tool` | `handlers::ground::run_tool_handler` | `src/api_server.rs:2876` | `CAP-09` | Trace provenance |
| POST | `/v1/ground/runs/:run_id/output` | `handlers::ground::submit_output_handler` | `src/api_server.rs:2880` | `CAP-09` | Trace provenance |
| GET | `/a2a/:slug/tasks/:episode_id` | `handlers::a2a::get_task_handler` | `src/api_server.rs:2884` | `CAP-14` | Connect and protect |
| GET | `/.well-known/agent-directory.json` | `handlers::a2a::agent_directory_handler` | `src/api_server.rs:2889` | `CAP-14` | Connect and protect |
| POST/GET | `/a2a/:slug/tasks/:episode_id/pushNotificationConfigs` | `handlers::a2a::create_push_config_handler, handlers::a2a::list_push_configs_handler` | `src/api_server.rs:2893` | `CAP-14` | Connect and protect |
| GET/DELETE | `/a2a/:slug/tasks/:episode_id/pushNotificationConfigs/:config_id` | `handlers::a2a::get_push_config_handler, handlers::a2a::delete_push_config_handler` | `src/api_server.rs:2898` | `CAP-14` | Connect and protect |
| GET | `/api/agents` | `handlers::agents::list_agents` | `src/api_server.rs:2903` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id` | `handlers::agents::get_agent_handler` | `src/api_server.rs:2910` | `CAP-01` | Run agents |
| GET | `/api/agents/curated` | `handlers::agents::list_curated_agents_handler` | `src/api_server.rs:2914` | `CAP-01` | Run agents |
| GET | `/api/agents/contributions` | `handlers::agents::agent_contributions_handler` | `src/api_server.rs:2932` | `CAP-01` | Run agents |
| GET | `/api/apps` | `handlers::apps::list_apps_handler` | `src/api_server.rs:2940` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/apps/:slug` | `handlers::apps::get_app_handler` | `src/api_server.rs:2941` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/apps/:slug/schema` | `handlers::apps::get_app_schema_handler` | `src/api_server.rs:2942` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/waitlist` | `handlers::misc::waitlist_handler` | `src/api_server.rs:2946` | `CAP-14` | accounts, sharing, notifications |
| GET | `/api/models/catalogue` | `handlers::agents::model_catalogue_handler` | `src/api_server.rs:2947` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/avatar` | `handlers::agents::get_cached_avatar` | `src/api_server.rs:2951` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/episodes` | `handlers::consolidation::get_agent_episodes_handler` | `src/api_server.rs:2955` | `CAP-06` | Learn and improve |
| GET | `/api/episodes/:episode_id` | `handlers::metrics::get_episode_detail_handler` | `src/api_server.rs:2959` | `CAP-01` | Run agents |
| GET | `/api/metrics/platform` | `handlers::metrics::platform_metrics_handler` | `src/api_server.rs:2963` | `ops` |  |
| GET | `/api/agents/:agent_id/metrics` | `handlers::metrics::agent_metrics_handler` | `src/api_server.rs:2967` | `ops` |  |
| GET | `/api/agents/:agent_id/eval/test-cases` | `handlers::eval::list_eval_test_cases_handler` | `src/api_server.rs:2972` | `CAP-05` | Observe |
| POST | `/api/agents/:agent_id/eval/test-cases` | `handlers::eval::create_eval_test_case_handler` | `src/api_server.rs:2976` | `CAP-05` | Observe |
| GET | `/api/agents/:agent_id/funding` | `handlers::agent_funding::get_agent_funding_handler` | `src/api_server.rs:2988` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/mcp-servers` | `handlers::agents::get_agent_mcp_servers_handler` | `src/api_server.rs:2998` | `CAP-14` | Connect and protect |
| POST | `/api/agents/:agent_id/mcp-servers/test` | `handlers::agents::test_agent_mcp_server_handler` | `src/api_server.rs:3002` | `CAP-14` | Connect and protect |
| GET | `/api/agents/:agent_id/published-tools` | `handlers::agents::get_agent_published_tools_handler` | `src/api_server.rs:3011` | `CAP-01` | Run agents |
| GET | `/api/skills` | `handlers::agents::list_skills_handler` | `src/api_server.rs:3019` | `CAP-01` | Run agents |
| GET | `/api/fleet` | `handlers::fleet::fleet_handler` | `src/api_server.rs:3023` | `CAP-02` | Compose teams |
| PUT | `/api/agents/:agent_id/eval/test-cases/:test_case_id` | `handlers::eval::update_eval_test_case_handler` | `src/api_server.rs:3024` | `CAP-05` | Observe |
| DELETE | `/api/agents/:agent_id/eval/test-cases/:test_case_id` | `handlers::eval::delete_eval_test_case_handler` | `src/api_server.rs:3028` | `CAP-05` | Observe |
| POST | `/api/agents/:agent_id/eval/test-cases/generate-rubrics` | `handlers::eval::generate_rubrics_handler` | `src/api_server.rs:3032` | `CAP-05` | Observe |
| POST | `/api/me/eval/runs/batch` | `handlers::eval::batch_eval_run_handler` | `src/api_server.rs:3036` | `CAP-14` | Isolate tenants / auth |
| POST | `/api/agents/:agent_id/eval/run` | `handlers::eval::trigger_eval_run_handler` | `src/api_server.rs:3040` | `CAP-05` | Observe |
| GET | `/api/agents/:agent_id/eval/runs` | `handlers::eval::list_eval_runs_handler` | `src/api_server.rs:3044` | `CAP-05` | Observe |
| GET | `/api/agents/:agent_id/eval/runs/:run_id/signals` | `handlers::eval::list_eval_signals_handler` | `src/api_server.rs:3048` | `CAP-05` | Observe |
| GET | `/api/observatory/fleet/summary` | `handlers::observatory::fleet_summary_handler` | `src/api_server.rs:3054` | `CAP-02` | Compose teams |
| POST | `/api/observatory/fleet/scan` | `handlers::observatory::fleet_scan_handler` | `src/api_server.rs:3058` | `CAP-02` | Compose teams |
| GET | `/api/observatory/fleet/agents` | `handlers::observatory::fleet_agents_handler` | `src/api_server.rs:3062` | `CAP-02` | Compose teams |
| GET | `/api/observatory/loops/brier/mechanism` | `handlers::observatory::loop5_mechanism_handler` | `src/api_server.rs:3069` | `CAP-06` | Learn and improve |
| GET | `/api/observatory/loops/dreaming/maturity` | `handlers::dreaming_maturity::fleet_dreaming_maturity_handler` | `src/api_server.rs:3076` | `CAP-06` | Learn and improve |
| GET | `/api/workspaces/:workspace_id/composition/suggestions` | `handlers::composition_evolution::composition_suggestions_handler` | `src/api_server.rs:3085` | `CAP-02` | Compose teams |
| POST | `/api/workspaces/:workspace_id/composition/suggestions/materialise` | `handlers::composition_evolution::materialise_composition_proposal_handler` | `src/api_server.rs:3089` | `CAP-02` | Compose teams |
| POST | `/api/observatory/dyads/auto-form` | `handlers::observatory::auto_form_dyads_handler` | `src/api_server.rs:3094` | `CAP-05` | Observe |
| POST | `/api/observatory/agents/:agent_id/backfill-social` | `handlers::observatory::backfill_social_handler` | `src/api_server.rs:3098` | `CAP-05` | Observe |
| PATCH | `/api/observatory/dyads/:dyad_id` | `handlers::observatory::patch_dyad_profile_handler` | `src/api_server.rs:3102` | `CAP-05` | Observe |
| GET | `/api/observatory/agents/:agent_id/relationships` | `handlers::observatory::agent_relationships_handler` | `src/api_server.rs:3106` | `CAP-05` | Observe |
| GET | `/api/observatory/agents/:agent_id/timeline` | `handlers::observatory::get_agent_timeline_handler` | `src/api_server.rs:3111` | `CAP-05` | Observe |
| GET | `/api/observatory/agents/:agent_id/dyads` | `handlers::observatory::list_agent_dyads_handler` | `src/api_server.rs:3115` | `CAP-05` | Observe |
| GET | `/api/observatory/agents/:agent_id/anomalies` | `handlers::observatory::list_agent_anomalies_handler` | `src/api_server.rs:3119` | `CAP-05` | Observe |
| GET | `/api/observatory/agents/:agent_id/loops` | `handlers::loops::agent_loops_handler` | `src/api_server.rs:3138` | `CAP-06` | Learn and improve |
| GET | `/api/agents/:agent_id/coordination-notes` | `handlers::loops::agent_coordination_notes_handler` | `src/api_server.rs:3142` | `CAP-04` | Coordinate work |
| GET | `/api/loops` | `handlers::loops::list_loops_handler` | `src/api_server.rs:3155` | `CAP-06` | Learn and improve |
| GET | `/api/loops/actions` | `handlers::loops::list_loop_actions_handler` | `src/api_server.rs:3157` | `CAP-06` | Learn and improve |
| GET | `/api/loops/:loop_id` | `handlers::loops::get_loop_handler` | `src/api_server.rs:3161` | `CAP-06` | Learn and improve |
| GET | `/api/gates` | `handlers::loops::list_gates_handler` | `src/api_server.rs:3172` | `CAP-06` | Learn and improve |
| GET | `/api/gates/:gate_id/decisions` | `handlers::loops::gate_decisions_handler` | `src/api_server.rs:3174` | `CAP-06` | Learn and improve |
| POST | `/api/gates/:gate_id/decisions/:decision_id/review` | `handlers::loops::review_gate_decision_handler` | `src/api_server.rs:3189` | `CAP-06` | Learn and improve |
| GET | `/api/episodes/:episode_id/trace` | `handlers::loops::episode_trace_handler` | `src/api_server.rs:3202` | `CAP-06` | Learn and improve |
| GET | `/api/verification-queue` | `handlers::loops::verification_queue_handler` | `src/api_server.rs:3214` | `CAP-06` | Learn and improve |
| POST | `/api/verification-queue/:assertion_id/settle` | `handlers::loops::settle_verification_handler` | `src/api_server.rs:3218` | `CAP-06` | Learn and improve |
| POST | `/api/episodes/:episode_id/probe` | `handlers::loops::probe_field_handler` | `src/api_server.rs:3229` | `CAP-06` | Learn and improve |
| POST | `/api/episodes/:episode_id/contradict` | `handlers::loops::contradict_field_handler` | `src/api_server.rs:3239` | `CAP-06` | Learn and improve |
| GET | `/api/declarations` | `handlers::loops::list_declarations_handler` | `src/api_server.rs:3256` | `CAP-06` | Learn and improve |
| GET | `/api/evaluators` | `handlers::loops::list_evaluators_handler` | `src/api_server.rs:3264` | `CAP-06` | Learn and improve |
| POST | `/api/observatory/agents/:agent_id/scan` | `handlers::observatory::trigger_agent_scan_handler` | `src/api_server.rs:3268` | `CAP-05` | Observe |
| GET | `/api/observatory/hitl` | `handlers::observatory::list_hitl_queue_handler` | `src/api_server.rs:3272` | `CAP-05` | Observe |
| POST | `/api/observatory/hitl/:event_id/action` | `handlers::observatory::record_hitl_action_handler` | `src/api_server.rs:3276` | `CAP-05` | Observe |
| POST | `/api/observatory/hitl/consensus/:request_id` | `handlers::observatory::confirm_two_reviewer_handler` | `src/api_server.rs:3281` | `CAP-05` | Observe |
| GET | `/api/agents/:agent_id/dependencies` | `handlers::agents::get_agent_dependencies_handler` | `src/api_server.rs:3285` | `CAP-01` | Run agents |
| GET | `/api/agents/:id/calibration` | `handlers::agents::get_agent_calibration_handler` | `src/api_server.rs:3289` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/agents/:id/fidelity` | `handlers::agents::agent_fidelity_handler` | `src/api_server.rs:3295` | `CAP-01` | Run agents |
| GET | `/api/agents/:id/competition-stats` | `handlers::agents::agent_competition_stats_handler` | `src/api_server.rs:3301` | `CAP-01` | Run agents |
| GET | `/api/me/providers` | `handlers::agents::list_my_providers_handler` | `src/api_server.rs:3305` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/me/loop-health` | `handlers::agents::loop_health_handler` | `src/api_server.rs:3309` | `ops` |  |
| GET | `/api/me/rounds` | `handlers::rounds::rounds_handler` | `src/api_server.rs:3315` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/bestiary` | `handlers::bestiary::bestiary_handler` | `src/api_server.rs:3319` | `CAP-01` | Run agents |
| GET | `/api/gates/enforcement` | `handlers::gates::gates_handler` | `src/api_server.rs:3334` | `CAP-11` | Gate effects |
| GET | `/api/specimen/:agent_name` | `handlers::specimen::specimen_handler` | `src/api_server.rs:3341` | `tenant:rabble` |  |
| GET | `/api/episodes/recent` | `handlers::specimen::recent_episodes_handler` | `src/api_server.rs:3346` | `tenant:rabble` |  |
| GET | `/api/episodes/:episode_id/lineage` | `handlers::specimen::episode_lineage_handler` | `src/api_server.rs:3351` | `tenant:rabble` |  |
| GET | `/api/stream` | `handlers::specimen::stream_handler` | `src/api_server.rs:3356` | `tenant:rabble` |  |
| GET | `/api/bestiary/cards` | `handlers::specimen::bestiary_cards_handler` | `src/api_server.rs:3358` | `tenant:rabble` |  |
| GET | `/api/workspaces/:workspace_id/flow` | `handlers::specimen::workspace_flow_handler` | `src/api_server.rs:3364` | `tenant:rabble` |  |
| GET | `/api/me/apps-health` | `handlers::apps::apps_health_handler` | `src/api_server.rs:3368` | `ops` |  |
| GET | `/api/agents/:agent_id/versions` | `handlers::agents::list_agent_versions_handler` | `src/api_server.rs:3372` | `ops` |  |
| GET | `/api/agents/:agent_id/versions/:version_num` | `handlers::agents::get_agent_version_handler` | `src/api_server.rs:3376` | `ops` |  |
| GET | `/api/agents/:agent_id/versions/:version_num/diff` | `handlers::agents::diff_agent_version_handler` | `src/api_server.rs:3385` | `ops` |  |
| GET | `/api/agents/:agent_id/versions/timeline` | `handlers::agents::agent_version_timeline_handler` | `src/api_server.rs:3394` | `ops` |  |
| GET | `/api/agents/:agent_id/ontology` | `handlers::ontology::get_ontology` | `src/api_server.rs:3403` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/ontology/history` | `handlers::ontology::get_ontology_history` | `src/api_server.rs:3408` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/ontology/snapshots/:snapshot_id` | `handlers::ontology::get_ontology_snapshot` | `src/api_server.rs:3412` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/ontology/diff` | `handlers::ontology::get_ontology_diff` | `src/api_server.rs:3416` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/kg` | `handlers::kg::kg_overview_handler` | `src/api_server.rs:3421` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/kg/entities` | `handlers::kg::list_entities_handler` | `src/api_server.rs:3425` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/kg/entities/:entity_id` | `handlers::kg::get_entity_handler` | `src/api_server.rs:3429` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/kg/entities/:entity_id/facts` | `handlers::kg::get_entity_facts_handler` | `src/api_server.rs:3433` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/kg/facts` | `handlers::kg::list_facts_handler` | `src/api_server.rs:3437` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/kg/rules` | `handlers::kg::list_rules_handler` | `src/api_server.rs:3441` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/kg/rules/:rule_id` | `handlers::kg::get_rule_handler` | `src/api_server.rs:3445` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/kg/communities` | `handlers::kg::list_communities_handler` | `src/api_server.rs:3449` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/wallet` | `handlers::agent_wallet::get_agent_wallet_handler` | `src/api_server.rs:3454` | `replaced:acn` | money |
| GET | `/api/agents/:agent_id/earnings` | `handlers::agent_wallet::get_agent_earnings_handler` | `src/api_server.rs:3458` | `replaced:acn` | money |
| POST | `/api/agents/:agent_id/collect` | `handlers::agent_wallet::collect_handler` | `src/api_server.rs:3462` | `replaced:acn` | money |
| POST | `/api/agents/:agent_id/allocate` | `handlers::agent_wallet::allocate_handler` | `src/api_server.rs:3466` | `replaced:acn` | money |
| PUT | `/api/agents/:agent_id/auto-collect` | `handlers::agent_wallet::set_auto_collect_handler` | `src/api_server.rs:3470` | `replaced:acn` | money |
| GET | `/projector` | `handlers::pages::projector_view` | `src/api_server.rs:3475` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/agent/:agent_id/projector` | `handlers::pages::projector_view` | `src/api_server.rs:3476` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/api/agents/:agent_id/projections` | `handlers::ontology::get_agent_projections` | `src/api_server.rs:3480` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/projections/bestiary` | `handlers::ontology::get_bestiary_projections` | `src/api_server.rs:3484` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/agents/:agent_id/projections/temporal` | `handlers::ontology::get_temporal_projections` | `src/api_server.rs:3488` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/beacons/nearby` | `handlers::beacons::nearby_beacons_handler` | `src/api_server.rs:3493` | `tenant:rabble` |  |
| GET | `/api/beacons/:beacon_id` | `handlers::beacons::get_beacon_handler` | `src/api_server.rs:3497` | `tenant:rabble` |  |
| GET | `/api/beacons/:beacon_id/asset` | `handlers::beacons::beacon_asset_handler` | `src/api_server.rs:3501` | `tenant:rabble` |  |
| GET | `/api/grid-maps/:map_id` | `handlers::beacons::get_grid_map_handler` | `src/api_server.rs:3505` | `tenant:rabble` |  |
| GET | `/api/creatures` | `handlers::creatures::list_creatures_handler` | `src/api_server.rs:3510` | `tenant:rabble` |  |
| GET | `/api/creatures/:creature_id` | `handlers::creatures::get_creature_handler` | `src/api_server.rs:3514` | `tenant:rabble` |  |
| GET | `/api/creatures/:creature_id/flights` | `handlers::creatures::creature_flights_handler` | `src/api_server.rs:3518` | `tenant:rabble` |  |
| GET | `/api/creatures/:creature_id/versions` | `handlers::creatures::creature_versions_handler` | `src/api_server.rs:3522` | `ops` |  |
| GET | `/api/creatures/:creature_id/image` | `handlers::creatures::creature_image_handler` | `src/api_server.rs:3526` | `tenant:rabble` |  |
| GET | `/api/creatures/:creature_id/animation/:layer_name` | `handlers::creatures::creature_animation_layer_handler` | `src/api_server.rs:3530` | `tenant:rabble` |  |
| GET | `/api/creatures/:creature_id/animation-status` | `handlers::creatures::creature_animation_status_handler` | `src/api_server.rs:3534` | `tenant:rabble` |  |
| GET | `/api/creatures/:creature_id/flight-path/:flight_id` | `handlers::creatures::creature_flight_path_handler` | `src/api_server.rs:3538` | `tenant:rabble` |  |
| GET | `/api/creatures/:creature_id/cognition` | `handlers::creatures::creature_cognition_handler` | `src/api_server.rs:3542` | `tenant:rabble` |  |
| GET | `/api/swarms` | `handlers::creatures::list_swarms_handler` | `src/api_server.rs:3546` | `tenant:rabble` |  |
| GET | `/api/swarms/:swarm_id` | `handlers::creatures::get_swarm_handler` | `src/api_server.rs:3547` | `tenant:rabble` |  |
| GET | `/api/rabble/:id/qr` | `handlers::qr_codes::rabble_qr_handler` | `src/api_server.rs:3552` | `tenant:rabble` |  |
| GET | `/user/:user_id` | `handlers::users::user_profile_view` | `src/api_server.rs:3557` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/users/:user_id` | `handlers::users::get_public_profile_handler` | `src/api_server.rs:3558` | `CAP-14` | Isolate tenants / auth |
| GET | `/dashboard` | `handlers::pages::dashboard_view` | `src/api_server.rs:3563` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/agents/new` | `handlers::pages::agent_create_view` | `src/api_server.rs:3564` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/contracts` | `handlers::pages::contract_builder_view` | `src/api_server.rs:3568` | `CAP-03` | Contract work |
| GET | `/workspace/:workspace_id` | `handlers::pages::workspace_view` | `src/api_server.rs:3569` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/auth/google` | `handlers::auth::auth_google` | `src/api_server.rs:3574` | `CAP-14` | Isolate tenants / auth |
| GET | `/auth/github` | `handlers::auth::auth_github` | `src/api_server.rs:3575` | `CAP-14` | Isolate tenants / auth |
| GET | `/auth/callback` | `handlers::auth::auth_callback` | `src/api_server.rs:3576` | `CAP-14` | Isolate tenants / auth |
| POST | `/auth/logout` | `handlers::auth::auth_logout` | `src/api_server.rs:3577` | `CAP-14` | Isolate tenants / auth |
| GET | `/auth/cli` | `handlers::auth::auth_cli_start` | `src/api_server.rs:3579` | `CAP-14` | Isolate tenants / auth |
| GET | `/auth/cli/finish` | `handlers::auth::auth_cli_finish` | `src/api_server.rs:3580` | `CAP-14` | Isolate tenants / auth |
| POST | `/webhooks/stripe` | `handlers::billing::stripe_webhook_handler` | `src/api_server.rs:3582` | `replaced:acn` | money |
| POST | `/auth/siwe/challenge` | `handlers::auth::siwe_challenge_handler` | `src/api_server.rs:3587` | `CAP-14` | Isolate tenants / auth |
| POST | `/auth/siwe/verify` | `handlers::auth::siwe_verify_handler` | `src/api_server.rs:3591` | `CAP-14` | Isolate tenants / auth |
| GET | `/profile` | `handlers::profile::profile_view` | `src/api_server.rs:3596` | `CAP-14` | accounts, sharing, notifications |
| GET | `/settings` | `handlers::pages::settings_view` | `src/api_server.rs:3597` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/marketplace` | `handlers::pages::marketplace_view` | `src/api_server.rs:3598` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/admin` | `handlers::pages::admin_view` | `src/api_server.rs:3599` | `ops` | admin |
| GET | `/observatory` | `handlers::pages::observatory_view` | `src/api_server.rs:3601` | `CAP-05` | Observe |
| GET | `/ecology` | `handlers::pages::ecology_view` | `src/api_server.rs:3604` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/rounds` | `handlers::pages::rounds_view` | `src/api_server.rs:3605` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/bestiary` | `handlers::pages::bestiary_view` | `src/api_server.rs:3606` | `CAP-01` | Run agents |
| GET | `/loops` | `handlers::pages::loops_view` | `src/api_server.rs:3607` | `CAP-06` | Learn and improve |
| GET | `/gates` | `handlers::pages::loops_view` | `src/api_server.rs:3608` | `CAP-06` | Learn and improve |
| GET | `/evaluators` | `handlers::pages::loops_view` | `src/api_server.rs:3614` | `CAP-06` | Learn and improve |
| GET | `/specimen/:agent_name` | `handlers::pages::specimen_view` | `src/api_server.rs:3615` | `tenant:rabble` |  |
| GET | `/trace/:episode_id` | `handlers::pages::trace_view` | `src/api_server.rs:3616` | `CAP-01` | Run agents |
| GET | `/grounding/:episode_id` | `handlers::pages::grounding_view` | `src/api_server.rs:3617` | `CAP-09` | Trace provenance |
| GET | `/flow/:workspace_id` | `handlers::pages::flow_view` | `src/api_server.rs:3621` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/stream` | `handlers::pages::stream_view` | `src/api_server.rs:3622` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/declarations` | `handlers::pages::declarations_view` | `src/api_server.rs:3623` | `ui` | server-rendered page (Askama); replaced by tenant UIs |
| GET | `/gate/:gate_id` | `handlers::pages::gate_view` | `src/api_server.rs:3624` | `CAP-11` | Gate effects |
| GET | `/api/ecology/overview` | `handlers::ecology::ecology_overview_handler` | `src/api_server.rs:3625` | `crud:misc` | general workspace/social CRUD; review |
| GET | `/api/ecology/specimens` | `handlers::ecology::ecology_specimens_handler` | `src/api_server.rs:3629` | `tenant:rabble` |  |
| GET | `/observatory/hitl` | `handlers::pages::observatory_hitl_view` | `src/api_server.rs:3633` | `CAP-05` | Observe |
| GET | `/api/auth/me` | `handlers::auth::auth_me` | `src/api_server.rs:3655` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/auth/api-keys` | `handlers::auth::list_api_keys` | `src/api_server.rs:3656` | `CAP-14` | Isolate tenants / auth |
| POST | `/api/auth/api-keys` | `handlers::auth::create_api_key` | `src/api_server.rs:3657` | `CAP-14` | Isolate tenants / auth |
| DELETE | `/api/auth/api-keys/:key_id` | `handlers::auth::revoke_api_key` | `src/api_server.rs:3658` | `CAP-14` | Isolate tenants / auth |
| POST | `/api/teams` | `handlers::teams::create_team_handler` | `src/api_server.rs:3663` | `CAP-02` | Compose teams |
| GET | `/api/teams` | `handlers::teams::list_teams_handler` | `src/api_server.rs:3664` | `CAP-02` | Compose teams |
| GET | `/api/teams/:team_id` | `handlers::teams::get_team_handler` | `src/api_server.rs:3665` | `CAP-02` | Compose teams |
| DELETE | `/api/teams/:team_id` | `handlers::teams::delete_team_handler` | `src/api_server.rs:3669` | `CAP-02` | Compose teams |
| POST | `/api/teams/:team_id/members` | `handlers::teams::add_member_handler` | `src/api_server.rs:3673` | `CAP-02` | Compose teams |
| GET | `/api/teams/:team_id/members` | `handlers::teams::list_members_handler` | `src/api_server.rs:3677` | `CAP-02` | Compose teams |
| DELETE | `/api/teams/:team_id/members/:member_id` | `handlers::teams::remove_member_handler` | `src/api_server.rs:3681` | `CAP-02` | Compose teams |
| PUT | `/api/teams/:team_id/members/:member_id` | `handlers::teams::update_member_role_handler` | `src/api_server.rs:3685` | `CAP-02` | Compose teams |
| PUT | `/api/teams/:team_id/members/:member_id/capabilities` | `handlers::teams::set_member_capabilities_handler` | `src/api_server.rs:3695` | `CAP-02` | Compose teams |
| POST/GET | `/api/teams/:team_id/invites` | `handlers::invites::invite_to_team_handler, handlers::invites::list_team_invites_handler` | `src/api_server.rs:3703` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/teams/:team_id/shared` | `handlers::collab::team_shared_handler` | `src/api_server.rs:3724` | `CAP-02` | Compose teams |
| GET | `/api/teams/:team_id/activity` | `handlers::collab::team_activity_handler` | `src/api_server.rs:3728` | `CAP-02` | Compose teams |
| GET | `/api/teams/:team_id/contributions` | `handlers::collab::team_contributions_handler` | `src/api_server.rs:3732` | `CAP-02` | Compose teams |
| GET | `/api/teams/:team_id/ops` | `handlers::ops::team_ops_handler` | `src/api_server.rs:3740` | `CAP-02` | Compose teams |
| GET | `/api/me/invites` | `handlers::invites::list_my_invites_handler` | `src/api_server.rs:3750` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/me/invites/sent` | `handlers::invites::list_sent_invites_handler` | `src/api_server.rs:3754` | `CAP-14` | Isolate tenants / auth |
| POST | `/api/invites/:invite_id/decline` | `handlers::invites::decline_invite_handler` | `src/api_server.rs:3758` | `CAP-14` | Isolate tenants / auth |
| DELETE | `/api/invites/:invite_id` | `handlers::invites::revoke_invite_handler` | `src/api_server.rs:3762` | `CAP-14` | Isolate tenants / auth |
| POST | `/api/invites/:invite_id/accept` | `handlers::invites::accept_invite_handler` | `src/api_server.rs:3771` | `CAP-14` | Isolate tenants / auth |
| POST | `/api/invites/by-token/:token/accept` | `handlers::invites::accept_invite_by_token_handler` | `src/api_server.rs:3775` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/ontology-templates` | `handlers::wizard::list_ontology_templates_handler` | `src/api_server.rs:3780` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/agents/generate-ontology` | `handlers::wizard::generate_ontology_handler` | `src/api_server.rs:3784` | `CAP-01` | Run agents |
| POST | `/api/agents/generate-prompt` | `handlers::wizard::generate_prompt_handler` | `src/api_server.rs:3788` | `CAP-01` | Run agents |
| GET | `/api/agents/creation-guide` | `handlers::wizard::creation_guide_handler` | `src/api_server.rs:3792` | `CAP-01` | Run agents |
| GET | `/api/tags/popular` | `handlers::wizard::popular_tags_handler` | `src/api_server.rs:3796` | `crud:misc` | general workspace/social CRUD; review |
| POST | `/api/contracts/compile` | `handlers::contracts::compile_handler` | `src/api_server.rs:3803` | `CAP-03` | Contract work |
| GET | `/api/contracts/tools` | `handlers::contracts::tool_names_handler` | `src/api_server.rs:3807` | `CAP-03` | Contract work |
| GET | `/api/contracts/types` | `handlers::contracts::types_handler` | `src/api_server.rs:3815` | `CAP-03` | Contract work |
| POST | `/api/contracts/suggest` | `handlers::contracts::suggest_handler` | `src/api_server.rs:3819` | `CAP-03` | Contract work |
| GET | `/api/contracts/decompile/:agent_id` | `handlers::contracts::decompile_handler` | `src/api_server.rs:3826` | `CAP-03` | Contract work |
| POST | `/api/contracts/tool-request` | `handlers::contracts::tool_request_handler` | `src/api_server.rs:3833` | `CAP-03` | Contract work |
| POST | `/api/agents` | `handlers::agents::create_agent_handler` | `src/api_server.rs:3838` | `CAP-01` | Run agents |
| POST | `/api/agents/import` | `handlers::agents::import_agent_handler` | `src/api_server.rs:3839` | `CAP-01` | Run agents |
| GET | `/api/agents/mine` | `handlers::agents::list_my_agents_handler` | `src/api_server.rs:3843` | `CAP-01` | Run agents |
| PUT | `/api/agents/:agent_id` | `handlers::agents::update_agent_handler` | `src/api_server.rs:3847` | `CAP-01` | Run agents |
| DELETE | `/api/agents/:agent_id` | `handlers::agents::delete_agent_handler` | `src/api_server.rs:3851` | `CAP-01` | Run agents |
| POST | `/api/agents/:agent_id/fork` | `handlers::lifecycle::fork_agent_handler` | `src/api_server.rs:3856` | `CAP-01` | Run agents |
| PUT | `/api/agents/:agent_id/fork-pricing` | `handlers::lifecycle::update_fork_pricing_handler` | `src/api_server.rs:3860` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/publish-checks` | `handlers::lifecycle::publish_checks_handler` | `src/api_server.rs:3864` | `CAP-01` | Run agents |
| POST | `/api/agents/:agent_id/publish` | `handlers::lifecycle::publish_agent_handler` | `src/api_server.rs:3868` | `CAP-01` | Run agents |
| POST | `/api/agents/:agent_id/archive` | `handlers::lifecycle::archive_agent_handler` | `src/api_server.rs:3872` | `CAP-01` | Run agents |
| POST | `/api/agents/:agent_id/restore` | `handlers::lifecycle::restore_agent_handler` | `src/api_server.rs:3876` | `CAP-01` | Run agents |
| POST | `/api/agents/:agent_id/versions/:version_num/restore` | `handlers::agents::restore_agent_version_handler` | `src/api_server.rs:3880` | `ops` |  |
| POST | `/api/agents/:agent_id/avatar/generate` | `handlers::agents::generate_avatar` | `src/api_server.rs:3885` | `CAP-01` | Run agents |
| POST | `/api/agents/:agent_id/embeddings/import` | `handlers::agents::import_embeddings_handler` | `src/api_server.rs:3890` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/embeddings/stats` | `handlers::agents::embeddings_stats_handler` | `src/api_server.rs:3895` | `CAP-01` | Run agents |
| POST | `/api/agents/:agent_id/embeddings/export/consent` | `handlers::agents::embeddings_export_consent_handler` | `src/api_server.rs:3899` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/embeddings/export` | `handlers::agents::embeddings_export_handler` | `src/api_server.rs:3903` | `CAP-01` | Run agents |
| POST | `/api/agents/:agent_id/execute` | `handlers::execution::execute_agent_handler` | `src/api_server.rs:3908` | `CAP-01` | Run agents |
| POST | `/api/agents/:agent_id/execute/stream` | `handlers::execution_stream::execute_agent_stream_handler` | `src/api_server.rs:3913` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/dreaming/budget` | `handlers::consolidation::get_dreaming_budget` | `src/api_server.rs:3918` | `CAP-06` | Learn and improve |
| PUT | `/api/agents/:agent_id/dreaming/budget` | `handlers::consolidation::set_dreaming_budget` | `src/api_server.rs:3922` | `CAP-06` | Learn and improve |
| POST | `/api/agents/:agent_id/dreaming/topup` | `handlers::consolidation::topup_dreaming_budget_handler` | `src/api_server.rs:3926` | `CAP-06` | Learn and improve |
| POST | `/api/agents/:agent_id/consolidate` | `handlers::consolidation::consolidate_agent_handler` | `src/api_server.rs:3931` | `CAP-06` | Learn and improve |
| GET | `/api/agents/:agent_id/dreaming` | `handlers::dreaming_maturity::agent_dreaming_maturity_handler` | `src/api_server.rs:3935` | `CAP-06` | Learn and improve |
| GET | `/api/agents/:agent_id/evolution` | `handlers::evolution::agent_evolution_handler` | `src/api_server.rs:3941` | `CAP-01` | Run agents |
| GET | `/api/agents/:agent_id/consolidation/jobs/:job_id` | `handlers::consolidation::get_consolidation_job_handler` | `src/api_server.rs:3945` | `CAP-06` | Learn and improve |
| POST | `/api/apps` | `handlers::apps::create_app_handler` | `src/api_server.rs:3955` | `CAP-15` | Build and ship apps / workspaces |
| PUT | `/api/apps/:slug` | `handlers::apps::update_app_handler_full` | `src/api_server.rs:3956` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/apps/:slug/workspaces` | `handlers::apps::spawn_workspace_handler` | `src/api_server.rs:3960` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/apps/:slug/workspaces` | `handlers::apps::list_app_workspaces_handler` | `src/api_server.rs:3964` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/apps/:slug/workspaces/batch` | `handlers::apps::batch_spawn_workspaces_handler` | `src/api_server.rs:3968` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/apps/:slug/publish` | `handlers::apps::publish_app_handler` | `src/api_server.rs:3972` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/apps/:slug/archive` | `handlers::apps::archive_app_handler` | `src/api_server.rs:3976` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/apps/:slug/sync-auto-hire` | `handlers::apps::sync_auto_hire_handler` | `src/api_server.rs:3984` | `CAP-02` | Compose teams |
| POST | `/api/simops/cascade` | `handlers::simops::cascade_handler` | `src/api_server.rs:3989` | `tenant:simops` |  |
| POST | `/api/simops/project` | `handlers::simops::project_handler` | `src/api_server.rs:3994` | `tenant:simops` |  |
| POST | `/api/simops/cascade/suggest-bindings` | `handlers::workspace::actions::suggest_bindings_handler` | `src/api_server.rs:3999` | `tenant:simops` |  |
| POST | `/api/simops/dynamics` | `handlers::simops::dynamics_handler` | `src/api_server.rs:4004` | `tenant:simops` |  |
| GET | `/api/simops/dynamics/models` | `handlers::simops::dynamics_list_handler` | `src/api_server.rs:4008` | `tenant:simops` |  |
| POST | `/api/simops/rheology` | `handlers::simops::rheology_handler` | `src/api_server.rs:4013` | `tenant:simops` |  |
| GET | `/api/simops/rheology/models` | `handlers::simops::rheology_list_handler` | `src/api_server.rs:4017` | `tenant:simops` |  |
| POST | `/api/bayesops/fit_marginal` | `handlers::bayesops::fit_marginal_handler` | `src/api_server.rs:4027` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/bayesops/fit_conditional` | `handlers::bayesops::fit_conditional_handler` | `src/api_server.rs:4031` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/bayesops/predict` | `handlers::bayesops::predict_handler` | `src/api_server.rs:4035` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/bayesops/input_sensitivity` | `handlers::bayesops::input_sensitivity_handler` | `src/api_server.rs:4039` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/bayesops/compare_scenarios` | `handlers::bayesops::compare_scenarios_handler` | `src/api_server.rs:4043` | `tenant:simops` |  |
| POST | `/api/bayesops/prob_exceeds` | `handlers::bayesops::prob_exceeds_handler` | `src/api_server.rs:4047` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/bayesops/optimise_for_target` | `handlers::bayesops::optimise_for_target_handler` | `src/api_server.rs:4051` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/bayesops/posteriors` | `handlers::bayesops::list_posteriors_handler` | `src/api_server.rs:4055` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| DELETE | `/api/bayesops/posteriors/:id` | `handlers::bayesops::evict_posterior_handler` | `src/api_server.rs:4059` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/workspaces/:workspace_id/bayesops/state` | `handlers::bayesops::workspace_bayesops_state_handler` | `src/api_server.rs:4066` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/bayesops/pending/:pending_id/accept` | `handlers::bayesops::accept_pending_handler` | `src/api_server.rs:4070` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/bayesops/pending/:pending_id/reject` | `handlers::bayesops::reject_pending_handler` | `src/api_server.rs:4074` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/workspaces/:workspace_id/refit` | `handlers::workspace::refit::refit_workspace_handler` | `src/api_server.rs:4083` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/workspaces` | `handlers::workspace::list_workspaces_handler` | `src/api_server.rs:4088` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/workspaces/:workspace_id` | `handlers::workspace::get_workspace_handler` | `src/api_server.rs:4092` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/workspaces/:workspace_id/agents` | `handlers::workspace::list_workspace_agents_handler` | `src/api_server.rs:4096` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/agents` | `handlers::workspace::create_workspace_agent_handler` | `src/api_server.rs:4100` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/budget` | `handlers::workspace::fund_workspace_handler` | `src/api_server.rs:4104` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/workspaces/:workspace_id/budget` | `handlers::workspace::workspace_budget_handler` | `src/api_server.rs:4110` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/cascade` | `handlers::simops::workspace_cascade_handler` | `src/api_server.rs:4115` | `tenant:simops` |  |
| GET | `/api/simops/workspaces/:workspace_id/process-spacetime` | `handlers::simops_benchmark::process_spacetime_handler` | `src/api_server.rs:4120` | `tenant:simops` |  |
| GET/PUT | `/api/simops/workspaces/:workspace_id/sample-config` | `handlers::simops_benchmark::get_sample_config_handler, handlers::simops_benchmark::put_sample_config_handler` | `src/api_server.rs:4124` | `tenant:simops` |  |
| GET | `/api/workspaces/:workspace_id/actions` | `handlers::workspace::actions::list_actions_handler` | `src/api_server.rs:4132` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/workspaces/:workspace_id/actions/pending` | `handlers::workspace::actions::list_pending_handler` | `src/api_server.rs:4136` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/actions/mutate_document` | `handlers::workspace::actions::mutate_document_handler` | `src/api_server.rs:4140` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/actions/fork_state` | `handlers::workspace::actions::fork_state_handler` | `src/api_server.rs:4144` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/actions/compare` | `handlers::workspace::actions::compare_handler` | `src/api_server.rs:4148` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/actions/invoke_member` | `handlers::workspace::actions::invoke_member_handler` | `src/api_server.rs:4152` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/actions/annotate_schema` | `handlers::workspace::actions::annotate_schema_handler` | `src/api_server.rs:4156` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/actions/annotate` | `handlers::workspace::actions::annotate_handler` | `src/api_server.rs:4160` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/actions/log_observation` | `handlers::workspace::actions::log_observation_handler` | `src/api_server.rs:4165` | `CAP-05` | Observe |
| POST | `/api/workspaces/:workspace_id/actions/render_lens` | `handlers::workspace::lens_actions::render_lens_handler` | `src/api_server.rs:4170` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/actions/compare_lenses` | `handlers::workspace::lens_actions::compare_lenses_handler` | `src/api_server.rs:4174` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/actions/evaluate_claims` | `handlers::workspace::claim_evaluation::evaluate_claims_handler` | `src/api_server.rs:4182` | `CAP-05` | Observe |
| POST | `/api/workspaces/:workspace_id/actions/price_bom` | `handlers::workspace::bom_pricing::price_bom_handler` | `src/api_server.rs:4188` | `tenant:dpp` |  |
| POST | `/api/workspaces/:workspace_id/actions/flag_divergence` | `handlers::workspace::lens_actions::flag_divergence_handler` | `src/api_server.rs:4192` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/actions/calculate_carbon` | `handlers::workspace::carbon::calculate_carbon_handler` | `src/api_server.rs:4202` | `tenant:dpp` |  |
| POST | `/api/workspaces/:workspace_id/actions/identify` | `handlers::wild::identify_action_handler` | `src/api_server.rs:4212` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/actions/:action_id/accept` | `handlers::workspace::actions::accept_action_handler` | `src/api_server.rs:4216` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/actions/:action_id/reject` | `handlers::workspace::actions::reject_action_handler` | `src/api_server.rs:4220` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/actions/migrate_parallelism_to_twin` | `handlers::workspace::actions::migrate_parallelism_to_twin_handler` | `src/api_server.rs:4224` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/workspaces/:workspace_id/annotations` | `handlers::workspace::actions::list_annotations_handler` | `src/api_server.rs:4228` | `CAP-15` | Build and ship apps / workspaces |
| DELETE | `/api/workspaces/:workspace_id/annotations/:annotation_id` | `handlers::workspace::actions::resolve_annotation_handler` | `src/api_server.rs:4232` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/workspaces/:workspace_id/outputs` | `handlers::workspace::outputs::list_outputs_handler` | `src/api_server.rs:4237` | `CAP-15` | Build and ship apps / workspaces |
| PUT/GET/DELETE | `/api/workspaces/:workspace_id/outputs/:key` | `handlers::workspace::outputs::set_output_handler, handlers::workspace::outputs::get_output_handler, handlers::workspace::outputs::delete_output_handler` | `src/api_server.rs:4241` | `CAP-15` | Build and ship apps / workspaces |
| GET/POST | `/api/workspaces/:workspace_id/dependencies` | `handlers::workspace::outputs::list_dependencies_handler, handlers::workspace::outputs::add_dependency_handler` | `src/api_server.rs:4248` | `CAP-15` | Build and ship apps / workspaces |
| DELETE | `/api/workspaces/:workspace_id/dependencies/:upstream_id` | `handlers::workspace::outputs::remove_dependency_handler` | `src/api_server.rs:4253` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/resolve` | `handlers::workspace::resolution::resolve_workspace_handler` | `src/api_server.rs:4261` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/messages` | `handlers::workspace::post_workspace_message_handler` | `src/api_server.rs:4266` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/workspaces/:workspace_id/messages` | `handlers::workspace::get_workspace_messages_handler` | `src/api_server.rs:4270` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/workspaces/:workspace_id/messages/poll` | `handlers::workspace::poll_workspace_messages_handler` | `src/api_server.rs:4274` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/workspaces/:workspace_id/messages/stream` | `handlers::workspace::workspace_messages_stream_handler` | `src/api_server.rs:4279` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/workspaces/:workspace_id/workflow` | `handlers::workspace::get_workspace_workflow_handler` | `src/api_server.rs:4283` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/fork-to-app` | `handlers::workspace::fork_workspace_to_app_draft_handler` | `src/api_server.rs:4289` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/hire` | `handlers::workspace::hire_agent_handler` | `src/api_server.rs:4294` | `CAP-02` | Compose teams |
| POST | `/api/workspaces/:workspace_id/add` | `handlers::workspace::add_agent_handler` | `src/api_server.rs:4298` | `CAP-15` | Build and ship apps / workspaces |
| DELETE | `/api/workspaces/:workspace_id/agents/:agent_id` | `handlers::workspace::remove_workspace_agent_handler` | `src/api_server.rs:4302` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/xaman/sessions` | `handlers::xaman::list_xaman_sessions_handler` | `src/api_server.rs:4307` | `CAP-14` | Isolate tenants / auth |
| POST | `/api/xaman/sessions` | `handlers::xaman::create_xaman_session_handler` | `src/api_server.rs:4311` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/xaman/sessions/:id` | `handlers::xaman::get_xaman_session_handler` | `src/api_server.rs:4315` | `CAP-14` | Isolate tenants / auth |
| POST | `/api/xaman/sessions/:id/message` | `handlers::xaman::xaman_session_message_handler` | `src/api_server.rs:4319` | `CAP-14` | Isolate tenants / auth |
| POST | `/api/xaman/sessions/:id/complete` | `handlers::xaman::complete_xaman_session_handler` | `src/api_server.rs:4323` | `CAP-14` | Isolate tenants / auth |
| POST | `/api/xaman/sessions/:id/create-app` | `handlers::xaman::create_app_from_session_handler` | `src/api_server.rs:4327` | `CAP-14` | Isolate tenants / auth |
| DELETE | `/api/xaman/sessions/:id` | `handlers::xaman::abandon_xaman_session_handler` | `src/api_server.rs:4331` | `CAP-14` | Isolate tenants / auth |
| POST | `/api/workspaces/:workspace_id/composition/identity` | `handlers::workspace::set_composition_identity_handler` | `src/api_server.rs:4336` | `CAP-02` | Compose teams |
| GET | `/api/workspaces/:workspace_id/composition/versions` | `handlers::composition::list_composition_versions_handler` | `src/api_server.rs:4341` | `ops` |  |
| POST | `/api/workspaces/:workspace_id/composition/versions/:version_id/accept` | `handlers::composition::accept_composition_version_handler` | `src/api_server.rs:4345` | `ops` |  |
| POST | `/api/workspaces/:workspace_id/composition/versions/:version_id/reject` | `handlers::composition::reject_composition_version_handler` | `src/api_server.rs:4349` | `ops` |  |
| POST | `/api/workspaces/:workspace_id/composition/propose` | `handlers::composition::propose_composition_version_handler` | `src/api_server.rs:4353` | `CAP-02` | Compose teams |
| POST | `/api/workspaces/:workspace_id/composition/dream` | `handlers::composition::composition_dream_handler` | `src/api_server.rs:4357` | `CAP-06` | Learn and improve |
| GET | `/api/wallet` | `handlers::wallet::get_wallet_handler` | `src/api_server.rs:4362` | `replaced:acn` | money |
| GET | `/api/wallet/transactions` | `handlers::wallet::get_transactions_handler` | `src/api_server.rs:4363` | `replaced:acn` | money |
| POST | `/api/wallet/transfer` | `handlers::wallet::transfer_credits_handler` | `src/api_server.rs:4367` | `replaced:acn` | money |
| GET | `/api/billing/tiers` | `handlers::billing::billing_tiers_handler` | `src/api_server.rs:4372` | `replaced:acn` | money |
| POST | `/api/billing/checkout` | `handlers::billing::billing_checkout_handler` | `src/api_server.rs:4376` | `replaced:acn` | money |
| POST | `/api/billing/dev-topup` | `handlers::billing::billing_dev_topup_handler` | `src/api_server.rs:4380` | `replaced:acn` | money |
| GET | `/api/users/search` | `handlers::users::search_users_handler` | `src/api_server.rs:4385` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/users/collaborators` | `handlers::users::get_collaborators_handler` | `src/api_server.rs:4389` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/users/lookup` | `handlers::users::lookup_user_by_email_handler` | `src/api_server.rs:4396` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/me/workspace` | `handlers::rabble_workspace::get_personal_workspace_handler` | `src/api_server.rs:4401` | `tenant:rabble` |  |
| GET | `/api/profile` | `handlers::profile::get_profile_handler` | `src/api_server.rs:4406` | `CAP-14` | accounts, sharing, notifications |
| PUT | `/api/profile` | `handlers::profile::update_profile_handler` | `src/api_server.rs:4407` | `CAP-14` | accounts, sharing, notifications |
| GET | `/api/notifications` | `handlers::profile::list_notifications_handler` | `src/api_server.rs:4412` | `CAP-14` | accounts, sharing, notifications |
| PUT | `/api/notifications/:id/read` | `handlers::profile::mark_notification_read_handler` | `src/api_server.rs:4416` | `CAP-14` | accounts, sharing, notifications |
| PUT | `/api/notifications/read-all` | `handlers::profile::mark_all_notifications_read_handler` | `src/api_server.rs:4420` | `CAP-14` | accounts, sharing, notifications |
| POST | `/api/secrets` | `handlers::profile::create_secret_handler` | `src/api_server.rs:4425` | `CAP-14` | Keep secrets |
| GET | `/api/secrets` | `handlers::profile::list_secrets_handler` | `src/api_server.rs:4429` | `CAP-14` | Keep secrets |
| GET | `/api/secrets/audit` | `handlers::profile::secret_audit_handler` | `src/api_server.rs:4430` | `CAP-14` | Keep secrets |
| DELETE | `/api/secrets/:name` | `handlers::profile::delete_secret_handler` | `src/api_server.rs:4434` | `CAP-14` | Keep secrets |
| POST | `/api/workspaces/:workspace_id/coherence/evaluate` | `handlers::workspace::evaluate_coherence_handler` | `src/api_server.rs:4439` | `CAP-04` | Coordinate work |
| GET | `/api/workspaces/:workspace_id/coherence` | `handlers::workspace::get_coherence_handler` | `src/api_server.rs:4443` | `CAP-04` | Coordinate work |
| GET | `/api/workspaces/:workspace_id/coherence/history` | `handlers::workspace::get_coherence_history_handler` | `src/api_server.rs:4447` | `CAP-04` | Coordinate work |
| GET | `/api/workspaces/:workspace_id/ontology` | `handlers::workspace::get_workspace_ontology_handler` | `src/api_server.rs:4452` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/workspaces/:workspace_id/dpp/qr` | `handlers::qr_codes::dpp_passport_qr_handler` | `src/api_server.rs:4460` | `tenant:dpp` |  |
| GET/POST/DELETE | `/api/workspaces/:workspace_id/dpp/publish` | `handlers::workspace::dpp_publish::publication_status_handler, handlers::workspace::dpp_publish::publish_dpp_handler, handlers::workspace::dpp_publish::unpublish_dpp_handler` | `src/api_server.rs:4464` | `tenant:dpp` |  |
| GET | `/api/workspaces/:workspace_id/files` | `handlers::workspace::list_workspace_files_handler` | `src/api_server.rs:4470` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/workspaces/:workspace_id/files/*path` | `handlers::workspace::read_workspace_file_handler` | `src/api_server.rs:4474` | `CAP-15` | Build and ship apps / workspaces |
| PUT | `/api/workspaces/:workspace_id/files/*path` | `handlers::workspace::write_workspace_file_handler` | `src/api_server.rs:4478` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/workspaces/:workspace_id/files-raw/*path` | `handlers::workspace::read_workspace_file_raw_handler` | `src/api_server.rs:4482` | `CAP-15` | Build and ship apps / workspaces |
| POST | `/api/workspaces/:workspace_id/upload` | `handlers::workspace::upload_workspace_file_handler` | `src/api_server.rs:4487` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/workspaces/:workspace_id/git/log` | `handlers::workspace::workspace_git_log_handler` | `src/api_server.rs:4492` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/workspaces/:workspace_id/git/diff` | `handlers::workspace::workspace_git_diff_handler` | `src/api_server.rs:4496` | `CAP-15` | Build and ship apps / workspaces |
| GET | `/api/notebooks` | `handlers::notebooks::list_notebooks_handler` | `src/api_server.rs:4501` | `CAP-15` | notebooks (workspace state) |
| POST | `/api/notebooks` | `handlers::notebooks::create_notebook_handler` | `src/api_server.rs:4505` | `CAP-15` | notebooks (workspace state) |
| GET | `/api/notebooks/:notebook_id` | `handlers::notebooks::get_notebook_handler` | `src/api_server.rs:4509` | `CAP-15` | notebooks (workspace state) |
| PUT | `/api/notebooks/:notebook_id` | `handlers::notebooks::update_notebook_handler` | `src/api_server.rs:4513` | `CAP-15` | notebooks (workspace state) |
| DELETE | `/api/notebooks/:notebook_id` | `handlers::notebooks::delete_notebook_handler` | `src/api_server.rs:4517` | `CAP-15` | notebooks (workspace state) |
| POST | `/api/notebooks/:notebook_id/execute` | `handlers::notebooks::execute_notebook_handler` | `src/api_server.rs:4521` | `CAP-01` | Run agents |
| POST | `/api/fpl/execute` | `handlers::notebooks::fpl_execute_handler` | `src/api_server.rs:4526` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/fpl/health` | `handlers::notebooks::fpl_health_handler` | `src/api_server.rs:4530` | `ops` |  |
| POST | `/api/forecasts` | `handlers::forecasts::create_forecast_handler` | `src/api_server.rs:4535` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/forecasts` | `handlers::forecasts::list_forecasts_handler` | `src/api_server.rs:4539` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/forecasts/my-stats` | `handlers::forecasts::my_stats_handler` | `src/api_server.rs:4543` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/forecasts/:forecast_id` | `handlers::forecasts::get_forecast_handler` | `src/api_server.rs:4547` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| PUT | `/api/forecasts/:forecast_id` | `handlers::forecasts::update_forecast_handler` | `src/api_server.rs:4551` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| DELETE | `/api/forecasts/:forecast_id` | `handlers::forecasts::delete_forecast_handler` | `src/api_server.rs:4555` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/forecasts/:forecast_id/resolve` | `handlers::forecasts::resolve_forecast_handler` | `src/api_server.rs:4559` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/forecasts/:forecast_id/void` | `handlers::forecasts::void_forecast_handler` | `src/api_server.rs:4563` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/forecasts/:forecast_id/update-probability` | `handlers::forecasts::update_probability_handler` | `src/api_server.rs:4567` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET/PUT | `/api/forecasts/:forecast_id/schedules` | `handlers::forecasts::list_forecast_schedules_handler, handlers::forecasts::upsert_forecast_schedule_handler` | `src/api_server.rs:4572` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| DELETE | `/api/forecasts/:forecast_id/schedules/:schedule_id` | `handlers::forecasts::delete_forecast_schedule_handler` | `src/api_server.rs:4577` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/forecasts/:forecast_id/schedules/:schedule_id/run` | `handlers::forecasts::record_schedule_run_handler` | `src/api_server.rs:4581` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/forecasts/:forecast_id/spacetime` | `handlers::forecast_benchmark::forecast_spacetime_handler` | `src/api_server.rs:4586` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/forecasts/:forecast_id/timeline` | `handlers::forecast_benchmark::forecast_timeline_handler` | `src/api_server.rs:4594` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/forecasts/:forecast_id/cascade-provenance` | `handlers::forecast_benchmark::forecast_cascade_provenance_handler` | `src/api_server.rs:4604` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/forecasts/:forecast_id/commit` | `handlers::forecast_benchmark::commit_forecast_handler` | `src/api_server.rs:4608` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/benchmark/anchor-sweep` | `handlers::forecast_benchmark::anchor_sweep_handler` | `src/api_server.rs:4612` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET/POST | `/api/forecasts/:forecast_id/shares` | `handlers::shares::list_forecast_shares_handler, handlers::shares::create_forecast_share_handler` | `src/api_server.rs:4621` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| DELETE | `/api/forecasts/:forecast_id/shares/:share_id` | `handlers::shares::revoke_forecast_share_handler` | `src/api_server.rs:4626` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/forecasts/:forecast_id/access` | `handlers::collab::forecast_access_handler` | `src/api_server.rs:4637` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/forecasts/:forecast_id/activity` | `handlers::collab::forecast_activity_handler` | `src/api_server.rs:4641` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/forecasts/:forecast_id/history` | `handlers::forecast_git::forecast_history_handler` | `src/api_server.rs:4661` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/forecasts/:forecast_id/history/:sha` | `handlers::forecast_git::forecast_diff_handler` | `src/api_server.rs:4665` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/forecasts/:forecast_id/revert` | `handlers::forecast_git::forecast_revert_handler` | `src/api_server.rs:4669` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET/POST | `/api/forecasts/:forecast_id/annotations` | `handlers::annotations::list_annotations_handler, handlers::annotations::create_annotation_handler` | `src/api_server.rs:4689` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/forecasts/:forecast_id/annotations/:annotation_id/resolve` | `handlers::annotations::resolve_annotation_handler` | `src/api_server.rs:4694` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| DELETE | `/api/forecasts/:forecast_id/annotations/:annotation_id` | `handlers::annotations::delete_annotation_handler` | `src/api_server.rs:4698` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST/GET | `/api/forecasts/:forecast_id/invites` | `handlers::invites::invite_to_forecast_handler, handlers::invites::list_forecast_invites_handler` | `src/api_server.rs:4705` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST/GET | `/api/forecast-relationships` | `handlers::relationships::legacy::create_relationship_handler, handlers::relationships::legacy::list_relationships_handler` | `src/api_server.rs:4711` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| DELETE | `/api/forecast-relationships/:rel_id` | `handlers::relationships::legacy::delete_relationship_handler` | `src/api_server.rs:4716` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/forecast-relationships/:rel_id/propagate` | `handlers::relationships::legacy::propagate_relationship_handler` | `src/api_server.rs:4720` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST/GET | `/api/relationship-groups` | `handlers::relationships::groups::create_group_handler, handlers::relationships::groups::list_groups_handler` | `src/api_server.rs:4725` | `CAP-06` | Loop 2 cascade review queue (human-gated) |
| GET/PATCH/DELETE | `/api/relationship-groups/:group_id` | `handlers::relationships::groups::get_group_handler, handlers::relationships::groups::patch_group_handler, handlers::relationships::groups::delete_group_handler` | `src/api_server.rs:4730` | `CAP-06` | Loop 2 cascade review queue (human-gated) |
| GET | `/api/relationship-groups/:group_id/members` | `handlers::relationships::groups::get_group_members_handler` | `src/api_server.rs:4736` | `CAP-06` | Loop 2 cascade review queue (human-gated) |
| POST | `/api/relationship-groups/:group_id/propagate` | `handlers::relationships::groups::preview_group_propagation_handler` | `src/api_server.rs:4745` | `CAP-10` | Involve humans |
| GET/PUT | `/api/forecasts/:forecast_id/groups` | `handlers::relationships::membership::get_forecast_groups_handler, handlers::relationships::membership::set_forecast_groups_handler` | `src/api_server.rs:4757` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST/DELETE | `/api/forecasts/:forecast_id/groups/:group_id` | `handlers::relationships::membership::add_forecast_to_group_handler, handlers::relationships::membership::remove_forecast_from_group_handler` | `src/api_server.rs:4762` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/pending-cascades` | `handlers::pending_cascades::list_pending_cascades_handler` | `src/api_server.rs:4768` | `CAP-06` | Loop 2 cascade review queue (human-gated) |
| POST | `/api/pending-cascades/:cascade_id/apply` | `handlers::relationships::apply::apply_pending_cascade_handler` | `src/api_server.rs:4772` | `CAP-06` | Loop 2 cascade review queue (human-gated) |
| POST | `/api/pending-cascades/:cascade_id/dismiss` | `handlers::pending_cascades::dismiss_pending_cascade_handler` | `src/api_server.rs:4776` | `CAP-06` | Loop 2 cascade review queue (human-gated) |
| POST | `/api/pending-cascades/:cascade_id/undo` | `handlers::relationships::undo::undo_pending_cascade_handler` | `src/api_server.rs:4780` | `CAP-06` | Loop 2 cascade review queue (human-gated) |
| POST | `/api/pending-cascades/requeue` | `handlers::relationships::requeue::requeue_cascade_handler` | `src/api_server.rs:4784` | `CAP-06` | Loop 2 cascade review queue (human-gated) |
| GET | `/api/forecasts/:forecast_id/cascade-history` | `handlers::pending_cascades::cascade_history_handler` | `src/api_server.rs:4788` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/portfolios` | `handlers::forecasts::create_portfolio_handler` | `src/api_server.rs:4793` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/portfolios` | `handlers::forecasts::list_portfolios_handler` | `src/api_server.rs:4797` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/portfolios/:portfolio_id/stats` | `handlers::forecasts::portfolio_stats_handler` | `src/api_server.rs:4801` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| DELETE/PATCH | `/api/portfolios/:portfolio_id` | `handlers::forecasts::delete_portfolio_handler, handlers::forecasts::patch_portfolio_handler` | `src/api_server.rs:4805` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET/POST | `/api/portfolios/:portfolio_id/forecasts` | `handlers::forecasts::list_portfolio_forecasts_handler, handlers::forecasts::add_forecast_to_portfolio_handler` | `src/api_server.rs:4810` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| DELETE | `/api/portfolios/:portfolio_id/forecasts/:forecast_id` | `handlers::forecasts::remove_forecast_from_portfolio_handler` | `src/api_server.rs:4815` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET/POST | `/api/portfolios/:portfolio_id/shares` | `handlers::shares::list_portfolio_shares_handler, handlers::shares::create_portfolio_share_handler` | `src/api_server.rs:4820` | `CAP-14` | accounts, sharing, notifications |
| DELETE | `/api/portfolios/:portfolio_id/shares/:share_id` | `handlers::shares::revoke_portfolio_share_handler` | `src/api_server.rs:4825` | `CAP-14` | accounts, sharing, notifications |
| GET | `/api/portfolios/:portfolio_id/access` | `handlers::collab::portfolio_access_handler` | `src/api_server.rs:4833` | `CAP-14` | accounts, sharing, notifications |
| GET | `/api/portfolios/:portfolio_id/activity` | `handlers::collab::portfolio_activity_handler` | `src/api_server.rs:4837` | `CAP-05` | Observe |
| POST/GET | `/api/portfolios/:portfolio_id/invites` | `handlers::invites::invite_to_portfolio_handler, handlers::invites::list_portfolio_invites_handler` | `src/api_server.rs:4842` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/leaderboard` | `handlers::forecasts::leaderboard_handler` | `src/api_server.rs:4848` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/shares` | `handlers::teams::share_object_handler` | `src/api_server.rs:4853` | `CAP-02` | Compose teams |
| DELETE | `/api/shares/:share_id` | `handlers::teams::revoke_share_handler` | `src/api_server.rs:4854` | `CAP-02` | Compose teams |
| POST | `/api/polymarket/search` | `handlers::polymarket::search_handler` | `src/api_server.rs:4859` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/polymarket/snapshot` | `handlers::polymarket::snapshot_handler` | `src/api_server.rs:4863` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/polymarket/link` | `handlers::polymarket::link_handler` | `src/api_server.rs:4867` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/polymarket/import` | `handlers::polymarket::import_handler` | `src/api_server.rs:4871` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| GET | `/api/polymarket/observations` | `handlers::polymarket::observations_handler` | `src/api_server.rs:4875` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/polymarket/check-resolutions` | `handlers::polymarket::check_resolutions_handler` | `src/api_server.rs:4879` | `tenant:fermi-console` | also CAP-06/CAP-13 kernels |
| POST | `/api/admin/impersonate` | `handlers::impersonation::start_impersonation_handler` | `src/api_server.rs:4889` | `ops` | admin |
| POST | `/api/admin/impersonate/end` | `handlers::impersonation::end_impersonation_handler` | `src/api_server.rs:4893` | `ops` | admin |
| GET | `/api/admin/impersonate/sessions` | `handlers::impersonation::list_impersonation_sessions_handler` | `src/api_server.rs:4897` | `ops` | admin |
| GET | `/api/me/impersonation-history` | `handlers::impersonation::my_impersonation_history_handler` | `src/api_server.rs:4903` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/admin/economics/platform` | `handlers::economics::platform_economics_handler` | `src/api_server.rs:4910` | `replaced:acn` | money |
| GET | `/api/admin/stats` | `handlers::admin::admin_stats_handler` | `src/api_server.rs:4915` | `ops` | admin |
| GET | `/api/admin/users` | `handlers::admin::admin_list_users_handler` | `src/api_server.rs:4919` | `ops` | admin |
| POST | `/api/admin/users/:user_id/grant` | `handlers::admin::admin_grant_credits_handler` | `src/api_server.rs:4923` | `replaced:acn` | money |
| POST | `/api/admin/workspaces/:workspace_id/grant` | `handlers::admin::admin_grant_workspace_credits_handler` | `src/api_server.rs:4927` | `replaced:acn` | money |
| GET | `/api/admin/agents` | `handlers::admin::admin_list_agents_handler` | `src/api_server.rs:4931` | `ops` | admin |
| PUT | `/api/admin/agents/:agent_id/flag` | `handlers::admin::admin_flag_agent_handler` | `src/api_server.rs:4935` | `ops` | admin |
| GET/POST | `/api/admin/agents/legacy-slugs` | `handlers::admin::admin_legacy_agent_slugs_handler` | `src/api_server.rs:4945` | `ops` | admin |
| GET/POST | `/api/admin/agents/cleanup-test-cruft` | `handlers::admin::admin_cleanup_test_cruft_handler` | `src/api_server.rs:4957` | `ops` | admin |
| GET | `/api/orchestras` | `handlers::orchestras::list_orchestras_handler` | `src/api_server.rs:4966` | `CAP-04` | Coordinate work |
| GET | `/api/orchestras/:name/members` | `handlers::orchestras::list_orchestra_members_handler` | `src/api_server.rs:4970` | `CAP-04` | Coordinate work |
| DELETE | `/api/orchestras/:name/members/:agent_id` | `handlers::orchestras::revoke_orchestra_member_handler` | `src/api_server.rs:4976` | `CAP-04` | Coordinate work |
| GET/POST | `/api/orchestras/:name/requests` | `handlers::orchestras::list_orchestra_requests_handler, handlers::orchestras::submit_orchestra_request_handler` | `src/api_server.rs:4980` | `CAP-04` | Coordinate work |
| POST | `/api/orchestras/:name/requests/:request_id/approve` | `handlers::orchestras::approve_orchestra_request_handler` | `src/api_server.rs:4985` | `CAP-04` | Coordinate work |
| POST | `/api/orchestras/:name/requests/:request_id/reject` | `handlers::orchestras::reject_orchestra_request_handler` | `src/api_server.rs:4989` | `CAP-04` | Coordinate work |
| POST | `/api/orchestras/:name/requests/:request_id/withdraw` | `handlers::orchestras::withdraw_orchestra_request_handler` | `src/api_server.rs:4993` | `CAP-04` | Coordinate work |
| GET | `/api/agents/:agent_id/orchestras` | `handlers::orchestras::agent_orchestras_handler` | `src/api_server.rs:4997` | `CAP-04` | Coordinate work |
| GET | `/api/orchestras/:name/manager-effect` | `handlers::orchestras::orchestra_manager_effect_handler` | `src/api_server.rs:5004` | `CAP-04` | Coordinate work |
| GET | `/api/admin/apps` | `handlers::admin::admin_list_apps_handler` | `src/api_server.rs:5012` | `ops` | admin |
| PUT | `/api/admin/apps/:slug/visibility` | `handlers::admin::admin_set_app_visibility_handler` | `src/api_server.rs:5016` | `ops` | admin |
| GET | `/api/admin/agent-ownership-audit` | `handlers::admin::admin_agent_ownership_audit_handler` | `src/api_server.rs:5020` | `ops` | admin |
| POST | `/api/admin/agent-ownership-reassign` | `handlers::admin::admin_agent_ownership_reassign_handler` | `src/api_server.rs:5024` | `ops` | admin |
| GET | `/api/admin/rbac/orphans` | `handlers::admin_rbac::admin_rbac_orphans_handler` | `src/api_server.rs:5031` | `ops` | admin |
| POST | `/api/admin/rbac/reassign` | `handlers::admin_rbac::admin_rbac_reassign_handler` | `src/api_server.rs:5035` | `ops` | admin |
| POST | `/api/admin/rbac/heal` | `handlers::admin_rbac::admin_rbac_heal_handler` | `src/api_server.rs:5039` | `ops` | admin |
| GET | `/api/rbac/self-check` | `handlers::rbac_self_check::rbac_self_check_handler` | `src/api_server.rs:5048` | `CAP-14` | accounts, sharing, notifications |
| POST | `/api/admin/wipe-fermi-forecasts` | `handlers::admin::admin_wipe_fermi_forecasts_handler` | `src/api_server.rs:5056` | `ops` | admin |
| POST | `/api/admin/recompose-mutex-groups` | `handlers::admin::admin_recompose_mutex_groups_handler` | `src/api_server.rs:5065` | `ops` | admin |
| GET | `/api/admin/schema-health` | `handlers::admin::admin_schema_health_handler` | `src/api_server.rs:5074` | `ops` |  |
| GET | `/api/admin/liveness` | `handlers::admin::admin_liveness_handler` | `src/api_server.rs:5081` | `ops` | admin |
| GET/POST | `/api/admin/waitlist` | `handlers::admin::admin_list_waitlist_handler, handlers::admin::admin_add_waitlist_handler` | `src/api_server.rs:5085` | `ops` | admin |
| POST | `/api/admin/waitlist/invite` | `handlers::admin::admin_invite_waitlist_handler` | `src/api_server.rs:5090` | `ops` | admin |
| DELETE | `/api/admin/waitlist/:entry_id` | `handlers::admin::admin_delete_waitlist_handler` | `src/api_server.rs:5094` | `ops` | admin |
| GET | `/api/admin/creatures` | `handlers::admin::admin_list_creatures_handler` | `src/api_server.rs:5099` | `ops` | admin |
| PUT | `/api/admin/creatures/:creature_id/flag` | `handlers::admin::admin_flag_creature_handler` | `src/api_server.rs:5103` | `ops` | admin |
| GET | `/api/admin/swarms` | `handlers::admin::admin_list_swarms_handler` | `src/api_server.rs:5107` | `ops` | admin |
| PUT | `/api/admin/swarms/:swarm_id/status` | `handlers::admin::admin_update_swarm_status_handler` | `src/api_server.rs:5111` | `ops` | admin |
| POST | `/api/marketplace/match` | `handlers::marketplace::marketplace_match_handler` | `src/api_server.rs:5116` | `tenant:shopping` | listing/matching; selling between owners moves to ACN |
| GET | `/api/marketplace/listings` | `handlers::marketplace::list_marketplace_listings_handler` | `src/api_server.rs:5120` | `tenant:shopping` | listing/matching; selling between owners moves to ACN |
| POST | `/api/marketplace/listings` | `handlers::marketplace::create_marketplace_listing_handler` | `src/api_server.rs:5124` | `tenant:shopping` | listing/matching; selling between owners moves to ACN |
| GET | `/api/marketplace/history` | `handlers::marketplace::marketplace_history_handler` | `src/api_server.rs:5128` | `tenant:shopping` | listing/matching; selling between owners moves to ACN |
| GET | `/api/shopping/profile` | `handlers::marketplace::get_shopping_profiles_handler` | `src/api_server.rs:5132` | `CAP-14` | accounts, sharing, notifications |
| PUT | `/api/shopping/profile/:listing_id/listing` | `handlers::marketplace::update_listing_handler` | `src/api_server.rs:5136` | `CAP-14` | accounts, sharing, notifications |
| GET/POST | `/api/collections` | `handlers::creatures::list_collections_handler, handlers::creatures::create_collection_handler` | `src/api_server.rs:5141` | `tenant:rabble` |  |
| PUT | `/api/collections/:collection_id` | `handlers::creatures::update_collection_handler` | `src/api_server.rs:5146` | `tenant:rabble` |  |
| POST | `/api/flights` | `handlers::creatures::record_flight_handler` | `src/api_server.rs:5150` | `tenant:rabble` |  |
| PUT | `/api/flights/:flight_id/end` | `handlers::creatures::end_flight_handler` | `src/api_server.rs:5154` | `tenant:rabble` |  |
| POST | `/api/flights/:flight_id/telemetry` | `handlers::creatures::append_telemetry_handler` | `src/api_server.rs:5158` | `tenant:rabble` |  |
| GET | `/api/flights/:flight_id/export` | `handlers::creatures::export_flight_handler` | `src/api_server.rs:5162` | `tenant:rabble` |  |
| POST | `/api/flights/import` | `handlers::creatures::import_flight_handler` | `src/api_server.rs:5166` | `tenant:rabble` |  |
| GET | `/api/creatures/:creature_id/stream` | `handlers::streams::creature_stream_handler` | `src/api_server.rs:5171` | `tenant:rabble` |  |
| POST | `/api/creatures/:creature_id/perch` | `handlers::creatures::perch_handler` | `src/api_server.rs:5176` | `tenant:rabble` |  |
| POST | `/api/creatures/:creature_id/host` | `handlers::creatures::host_rabble_handler` | `src/api_server.rs:5180` | `tenant:rabble` |  |
| POST | `/api/creatures/:creature_id/fly` | `handlers::creatures::fly_handler` | `src/api_server.rs:5184` | `tenant:rabble` |  |
| POST/DELETE | `/api/creatures/:creature_id/tether` | `handlers::creatures::tether_handler, handlers::creatures::untether_handler` | `src/api_server.rs:5189` | `tenant:rabble` |  |
| POST | `/api/creatures/:creature_id/enemy-sensor` | `handlers::creatures::enemy_sensor_handler` | `src/api_server.rs:5194` | `tenant:rabble` |  |
| POST | `/api/creatures/:creature_id/genome-profiler` | `handlers::creatures::genome_profiler_handler` | `src/api_server.rs:5199` | `tenant:rabble` |  |
| POST | `/api/creatures/:creature_id/prey-locator` | `handlers::creatures::prey_locator_handler` | `src/api_server.rs:5204` | `tenant:rabble` |  |
| POST | `/api/creatures/:creature_id/forage` | `handlers::creatures::forage_handler` | `src/api_server.rs:5209` | `tenant:rabble` |  |
| GET/POST | `/api/creatures/:creature_id/goals` | `handlers::creatures::list_goals_handler, handlers::creatures::create_goal_handler` | `src/api_server.rs:5214` | `tenant:rabble` |  |
| PATCH | `/api/creatures/:creature_id/goals/:goal_id` | `handlers::creatures::update_goal_handler` | `src/api_server.rs:5219` | `tenant:rabble` |  |
| POST | `/api/creatures/:creature_id/telemetry` | `handlers::creatures::push_telemetry_handler` | `src/api_server.rs:5223` | `tenant:rabble` |  |
| GET | `/api/creatures/:creature_id/track` | `handlers::creatures::get_track_handler` | `src/api_server.rs:5227` | `tenant:rabble` |  |
| POST | `/api/creatures/mint` | `handlers::creatures::mint_creature_handler` | `src/api_server.rs:5232` | `tenant:rabble` |  |
| GET | `/api/my/rabbles` | `handlers::creatures::my_rabbles_handler` | `src/api_server.rs:5236` | `tenant:rabble` |  |
| POST | `/api/swarms/create` | `handlers::creatures::create_swarm_handler` | `src/api_server.rs:5240` | `tenant:rabble` |  |
| POST | `/api/swarms/:swarm_id/join` | `handlers::creatures::join_swarm_handler` | `src/api_server.rs:5244` | `tenant:rabble` |  |
| PATCH | `/api/swarms/:swarm_id` | `handlers::creatures::update_swarm_handler` | `src/api_server.rs:5248` | `tenant:rabble` |  |
| GET | `/api/feed` | `handlers::creatures::feed_handler` | `src/api_server.rs:5253` | `tenant:rabble` |  |
| GET | `/api/dashboard/my-rabbles` | `handlers::dashboard::my_rabbles_handler` | `src/api_server.rs:5256` | `tenant:rabble` |  |
| GET | `/api/dashboard/nearby` | `handlers::dashboard::nearby_rabbles_handler` | `src/api_server.rs:5260` | `tenant:rabble` |  |
| GET | `/api/dashboard/creatures` | `handlers::dashboard::creatures_handler` | `src/api_server.rs:5264` | `tenant:rabble` |  |
| GET | `/api/dashboard/boundary-violations` | `handlers::dashboard::boundary_violations_handler` | `src/api_server.rs:5268` | `crud:misc` | general workspace/social CRUD; review |
| GET | `/api/dashboard/nearby-creatures` | `handlers::dashboard::nearby_creatures_handler` | `src/api_server.rs:5272` | `tenant:rabble` |  |
| GET/POST | `/api/locations` | `handlers::dashboard::list_locations_handler, handlers::dashboard::save_location_handler` | `src/api_server.rs:5277` | `crud:misc` | general workspace/social CRUD; review |
| PATCH/DELETE | `/api/locations/:id` | `handlers::dashboard::update_location_handler, handlers::dashboard::delete_location_handler` | `src/api_server.rs:5282` | `crud:misc` | general workspace/social CRUD; review |
| POST/DELETE | `/api/creatures/:creature_id/favourite` | `handlers::creatures::favourite_creature_handler, handlers::creatures::unfavourite_creature_handler` | `src/api_server.rs:5287` | `tenant:rabble` |  |
| POST | `/api/creatures/:creature_id/generate-art` | `handlers::creatures::generate_art_handler` | `src/api_server.rs:5293` | `tenant:rabble` |  |
| POST | `/api/creatures/generate-art-batch` | `handlers::creatures::generate_art_batch_handler` | `src/api_server.rs:5297` | `tenant:rabble` |  |
| PUT | `/api/creatures/:creature_id/sosa-opt-in` | `handlers::creatures::sosa_opt_in_handler` | `src/api_server.rs:5301` | `tenant:rabble` |  |
| PUT | `/api/creatures/:creature_id/update` | `handlers::creatures::update_creature_handler` | `src/api_server.rs:5306` | `tenant:rabble` |  |
| PUT | `/api/creatures/:creature_id/status` | `handlers::creatures::update_creature_status_handler` | `src/api_server.rs:5310` | `tenant:rabble` |  |
| PUT | `/api/creatures/:creature_id/presence` | `handlers::creatures::update_creature_presence_handler` | `src/api_server.rs:5314` | `tenant:rabble` |  |
| POST | `/api/creatures/:creature_id/transfer` | `handlers::creatures::transfer_creature_handler` | `src/api_server.rs:5318` | `tenant:rabble` |  |
| POST | `/api/creatures/:creature_id/animate` | `handlers::creatures::animate_creature_handler` | `src/api_server.rs:5323` | `tenant:rabble` |  |
| GET | `/api/creatures/:creature_id/level` | `handlers::creatures::creature_level_handler` | `src/api_server.rs:5328` | `tenant:rabble` |  |
| POST | `/api/creatures/:creature_id/dream` | `handlers::creatures::creature_dream_handler` | `src/api_server.rs:5332` | `tenant:rabble` |  |
| GET | `/api/creatures/:creature_id/activity` | `handlers::creatures::creature_activity_handler` | `src/api_server.rs:5336` | `tenant:rabble` |  |
| PUT | `/api/creatures/:creature_id/visibility` | `handlers::creatures::update_creature_visibility_handler` | `src/api_server.rs:5341` | `tenant:rabble` |  |
| GET | `/api/flights/visible` | `handlers::creatures::list_visible_flights_handler` | `src/api_server.rs:5346` | `tenant:rabble` |  |
| GET/POST | `/api/creatures/:creature_id/devices` | `handlers::creatures::list_devices_handler, handlers::creatures::pair_device_handler` | `src/api_server.rs:5351` | `tenant:rabble` |  |
| PUT/DELETE | `/api/devices/:device_id` | `handlers::creatures::update_device_handler, handlers::creatures::unpair_device_handler` | `src/api_server.rs:5356` | `tenant:rabble` |  |
| POST | `/api/devices/:device_id/location` | `handlers::creatures::report_device_location_handler` | `src/api_server.rs:5361` | `tenant:rabble` |  |
| GET/POST | `/api/contacts` | `handlers::social::list_contacts_handler, handlers::social::add_contact_handler` | `src/api_server.rs:5366` | `crud:misc` | general workspace/social CRUD; review |
| PUT/DELETE | `/api/contacts/:contact_id` | `handlers::social::update_contact_handler, handlers::social::remove_contact_handler` | `src/api_server.rs:5371` | `crud:misc` | general workspace/social CRUD; review |
| POST | `/api/creature-friendships` | `handlers::social::send_friendship_request_handler` | `src/api_server.rs:5377` | `crud:misc` | general workspace/social CRUD; review |
| GET | `/api/creature-friendships/pending` | `handlers::social::pending_friendships_handler` | `src/api_server.rs:5381` | `crud:misc` | general workspace/social CRUD; review |
| POST | `/api/creature-friendships/:id/accept` | `handlers::social::accept_friendship_handler` | `src/api_server.rs:5385` | `crud:misc` | general workspace/social CRUD; review |
| POST | `/api/creature-friendships/:id/decline` | `handlers::social::decline_friendship_handler` | `src/api_server.rs:5389` | `crud:misc` | general workspace/social CRUD; review |
| DELETE | `/api/creature-friendships/:id` | `handlers::social::remove_friendship_handler` | `src/api_server.rs:5393` | `crud:misc` | general workspace/social CRUD; review |
| GET | `/api/creatures/:creature_id/friends` | `handlers::social::list_creature_friends_handler` | `src/api_server.rs:5397` | `tenant:rabble` |  |
| POST | `/api/creature-invites` | `handlers::social::send_creature_invite_handler` | `src/api_server.rs:5402` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/creature-invites/pending` | `handlers::social::list_pending_creature_invites_handler` | `src/api_server.rs:5406` | `CAP-14` | Isolate tenants / auth |
| POST | `/api/creature-invites/:id/accept` | `handlers::social::accept_creature_invite_handler` | `src/api_server.rs:5410` | `CAP-14` | Isolate tenants / auth |
| POST | `/api/creature-invites/:id/decline` | `handlers::social::decline_creature_invite_handler` | `src/api_server.rs:5414` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/rabble/:id/recap/:creature_id` | `handlers::social::rabble_recap_handler` | `src/api_server.rs:5419` | `tenant:rabble` |  |
| POST | `/api/rabble/:id/co-presence` | `handlers::social::record_co_presence_handler` | `src/api_server.rs:5423` | `tenant:rabble` |  |
| POST/PUT/DELETE | `/api/rabbles/:id/follow` | `handlers::social::follow_rabble_handler, handlers::social::update_follow_handler, handlers::social::unfollow_rabble_handler` | `src/api_server.rs:5428` | `tenant:rabble` |  |
| GET | `/api/my/following` | `handlers::social::list_following_handler` | `src/api_server.rs:5434` | `crud:misc` | general workspace/social CRUD; review |
| PUT | `/api/users/social-visibility` | `handlers::social::update_social_visibility_handler` | `src/api_server.rs:5439` | `crud:misc` | general workspace/social CRUD; review |
| GET | `/api/feed/events` | `handlers::social::activity_feed_handler` | `src/api_server.rs:5444` | `CAP-05` | Observe |
| GET | `/api/feed/stream` | `handlers::social::activity_feed_stream_handler` | `src/api_server.rs:5448` | `CAP-05` | Observe |
| GET/POST | `/api/rabble/:id/messages` | `handlers::rabble_chat::get_rabble_messages, handlers::rabble_chat::post_rabble_message` | `src/api_server.rs:5453` | `tenant:rabble` |  |
| GET | `/api/rabble/:id/stream` | `handlers::rabble_chat::rabble_stream` | `src/api_server.rs:5458` | `tenant:rabble` |  |
| POST | `/api/rabble/:id/invite` | `handlers::rabble_chat::invite_to_rabble` | `src/api_server.rs:5463` | `tenant:rabble` |  |
| DELETE | `/api/rabble/:id/invite/:user_id` | `handlers::rabble_chat::revoke_rabble_invite` | `src/api_server.rs:5467` | `tenant:rabble` |  |
| GET | `/api/rabble/:id/members` | `handlers::rabble_chat::list_rabble_members` | `src/api_server.rs:5471` | `tenant:rabble` |  |
| GET/POST | `/api/rabble/join/:qr_token` | `handlers::qr_codes::resolve_qr_token_handler, handlers::creatures::join_by_qr_token_handler` | `src/api_server.rs:5475` | `tenant:rabble` |  |
| POST | `/api/rabble/:id/flock` | `handlers::rabble_workspace::flock_tick_handler` | `src/api_server.rs:5481` | `tenant:rabble` |  |
| GET | `/api/rabble/:id/flock-history` | `handlers::rabble_workspace::flock_history_handler` | `src/api_server.rs:5485` | `tenant:rabble` |  |
| POST | `/api/rabble/:id/end` | `handlers::creatures::end_rabble_handler` | `src/api_server.rs:5490` | `tenant:rabble` |  |
| POST | `/api/rabble/:id/leave` | `handlers::creatures::leave_rabble_handler` | `src/api_server.rs:5494` | `tenant:rabble` |  |
| POST | `/api/rabble/:id/eject` | `handlers::governance::eject_creature_handler` | `src/api_server.rs:5498` | `tenant:rabble` |  |
| DELETE | `/api/rabble/:id/eject/:creature_id` | `handlers::governance::lift_ejection_handler` | `src/api_server.rs:5502` | `tenant:rabble` |  |
| POST | `/api/creatures/:creature_id/block` | `handlers::governance::block_creature_handler` | `src/api_server.rs:5507` | `tenant:rabble` |  |
| DELETE | `/api/creatures/:creature_id/block/:blocked_creature_id` | `handlers::governance::unblock_creature_handler` | `src/api_server.rs:5511` | `tenant:rabble` |  |
| POST | `/api/users/block` | `handlers::governance::block_user_handler` | `src/api_server.rs:5515` | `CAP-14` | moderation: block/report users |
| DELETE | `/api/users/block/:blocked_user_id` | `handlers::governance::unblock_user_handler` | `src/api_server.rs:5519` | `CAP-14` | moderation: block/report users |
| GET | `/api/my/blocks` | `handlers::governance::list_blocks_handler` | `src/api_server.rs:5523` | `CAP-14` | moderation: block/report users |
| POST | `/api/reports` | `handlers::governance::create_report_handler` | `src/api_server.rs:5527` | `CAP-14` | moderation: block/report users |
| GET | `/api/push/vapid-key` | `handlers::push::get_vapid_key_handler` | `src/api_server.rs:5532` | `tenant:rabble` | web push + proximity (Rabble) |
| POST/DELETE | `/api/push/subscribe` | `handlers::push::subscribe_handler, handlers::push::unsubscribe_handler` | `src/api_server.rs:5536` | `tenant:rabble` | web push + proximity (Rabble) |
| POST | `/api/push/proximity` | `handlers::push::proximity_check_handler` | `src/api_server.rs:5540` | `tenant:rabble` | web push + proximity (Rabble) |
| POST | `/api/rabble/:id/transfer-anchor` | `handlers::rabble_workspace::transfer_anchor_handler` | `src/api_server.rs:5544` | `tenant:rabble` |  |
| POST | `/api/rabble/:id/update-anchor-position` | `handlers::rabble_workspace::update_anchor_position_handler` | `src/api_server.rs:5548` | `tenant:rabble` |  |
| POST | `/api/swarms/:swarm_id/join-batch` | `handlers::rabble_workspace::join_batch_handler` | `src/api_server.rs:5553` | `tenant:rabble` |  |
| GET | `/api/rabble/:id/attraction-leaderboard` | `handlers::rabble_workspace::attraction_leaderboard_handler` | `src/api_server.rs:5557` | `tenant:rabble` |  |
| POST/GET | `/api/swarm/sessions` | `handlers::swarm_telemetry::create_session_handler, handlers::swarm_telemetry::list_sessions_handler` | `src/api_server.rs:5562` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/swarm/sessions/:session_id` | `handlers::swarm_telemetry::get_session_handler` | `src/api_server.rs:5567` | `CAP-14` | Isolate tenants / auth |
| PUT | `/api/swarm/sessions/:session_id/end` | `handlers::swarm_telemetry::end_session_handler` | `src/api_server.rs:5571` | `CAP-14` | Isolate tenants / auth |
| POST/GET | `/api/swarm/sessions/:session_id/telemetry` | `handlers::swarm_telemetry::ingest_telemetry_handler, handlers::swarm_telemetry::query_telemetry_handler` | `src/api_server.rs:5575` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/swarm/sessions/:session_id/summary` | `handlers::swarm_telemetry::session_summary_handler` | `src/api_server.rs:5580` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/swarm/sessions/:session_id/experience` | `handlers::swarm_telemetry::experience_export_handler` | `src/api_server.rs:5584` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/swarm-algorithms` | `handlers::swarm_algorithms::list_algorithms_handler` | `src/api_server.rs:5589` | `tenant:rabble` | swarm algorithm catalogue (Reynolds flock etc.) |
| POST | `/api/swarm-algorithms/activate` | `handlers::swarm_algorithms::activate_algorithm_handler` | `src/api_server.rs:5593` | `tenant:rabble` | swarm algorithm catalogue (Reynolds flock etc.) |
| GET | `/api/swarm-algorithms/activations/:swarm_id` | `handlers::swarm_algorithms::list_activations_handler` | `src/api_server.rs:5597` | `tenant:rabble` | swarm algorithm catalogue (Reynolds flock etc.) |
| GET | `/api/swarm-algorithms/:algorithm_id` | `handlers::swarm_algorithms::get_algorithm_handler` | `src/api_server.rs:5601` | `tenant:rabble` | swarm algorithm catalogue (Reynolds flock etc.) |
| POST/GET | `/api/observe/platforms` | `handlers::observations::create_platform_handler, handlers::observations::list_platforms_handler` | `src/api_server.rs:5606` | `CAP-05` | Observe |
| POST/GET | `/api/observe/sessions` | `handlers::observations::create_observation_session_handler, handlers::observations::list_observation_sessions_handler` | `src/api_server.rs:5611` | `CAP-14` | Isolate tenants / auth |
| PUT | `/api/observe/sessions/:session_id/end` | `handlers::observations::end_observation_session_handler` | `src/api_server.rs:5616` | `CAP-14` | Isolate tenants / auth |
| POST/GET | `/api/observe/sessions/:session_id/observations` | `handlers::observations::ingest_observations_handler, handlers::observations::query_observations_handler` | `src/api_server.rs:5620` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/observe/sessions/:session_id/summary` | `handlers::observations::observation_summary_handler` | `src/api_server.rs:5625` | `CAP-14` | Isolate tenants / auth |
| GET | `/api/observe/sessions/:session_id/experience` | `handlers::observations::observation_experience_handler` | `src/api_server.rs:5629` | `CAP-14` | Isolate tenants / auth |
| GET | `/rabble/` | `get(|| async { Redirect::permanent("/rabble") })` | `src/api_server.rs:5686` | `tenant:rabble` |  |
| GET | `/health` | `health_check` | `src/api/server.rs:20` | `ops` |  |
| GET | `/agents` | `list_agents` | `src/api/server.rs:22` | `drop?` | src/api/server.rs: second, older router; not mounted by api_server [I] |
| POST | `/agents` | `register_agent` | `src/api/server.rs:23` | `drop?` | src/api/server.rs: second, older router; not mounted by api_server [I] |
| GET | `/agents/:id` | `get_agent` | `src/api/server.rs:24` | `drop?` | src/api/server.rs: second, older router; not mounted by api_server [I] |
| POST | `/agents/:id/execute` | `execute_agent` | `src/api/server.rs:25` | `CAP-01` | Run agents |
| POST | `/agents/:id/save` | `save_agent` | `src/api/server.rs:26` | `drop?` | src/api/server.rs: second, older router; not mounted by api_server [I] |

## Needs owner decision

- `drop?` GET `/api/debug/startup` → `handlers::misc::debug_startup` (`src/api_server.rs:2821`) demo/debug
- `drop?` GET `/agents` → `list_agents` (`src/api/server.rs:22`) src/api/server.rs: second, older router; not mounted by api_server [I]
- `drop?` POST `/agents` → `register_agent` (`src/api/server.rs:23`) src/api/server.rs: second, older router; not mounted by api_server [I]
- `drop?` GET `/agents/:id` → `get_agent` (`src/api/server.rs:24`) src/api/server.rs: second, older router; not mounted by api_server [I]
- `drop?` POST `/agents/:id/save` → `save_agent` (`src/api/server.rs:26`) src/api/server.rs: second, older router; not mounted by api_server [I]

- `ui` routes are server-rendered Askama pages. The new core is API-only; each page's function belongs to a tenant UI or is dropped. Decide per page group.
