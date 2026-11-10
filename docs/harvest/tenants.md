# Tenant harvest

- Generated 2026-11-10 by `harvest_tenants.py`. Endpoint strings are extracted by regex over client sources and normalised (`{}`, `${x}`, `$x`, `:x` → `:p`), then matched to legacy routes from `routes.md`.
- Caveats: URLs assembled entirely at run time are missed; a matched route proves a dependency, not that the call works today [I]. Narrative sections (claims, health, MVP surface) are in `tenants-notes.md`.

## Tenants

| Tenant | Source | Files | Lines | Last commit | Repo | Distinct endpoints | Unmatched |
| --- | --- | --- | --- | --- | --- | --- | --- |
| fermi-console | `~/fermi/crates/fermi-console/src` | 40 | 75585 | 2026-08-28 | https://github.com/Replicant-Partners/fermi.git | 88 | 3 |
| simops | `~/kask/simops-*.js`<br>`~/kask/kask-sim-client.js`<br>`~/kask/dynamics` | 24 | 53828 | 2026-06-25 | https://github.com/ilabra-axo/kask.git | 2 | 1 |
| dpp-adaptogen | `~/kask/adaptogen`<br>`~/fermi/static/adaptogen-lab` | 5 | 6009 | 2026-06-12 | https://github.com/ilabra-axo/kask.git | 2 | 0 |
| kask-projects | `~/kask/projects` | 23 | 21967 | 2026-06-14 | https://github.com/ilabra-axo/kask.git | 0 | 0 |
| abw-web-ui | `~/fermi/static/js`<br>`~/fermi/templates` | 62 | 46818 | 2026-10-03 | https://github.com/Replicant-Partners/fermi.git | 197 | 7 |
| wild | `~/kask/kask-wild-client.js` | 1 | 549 | 2026-06-25 | https://github.com/ilabra-axo/kask.git | 1 | 0 |
| kask-shared | `~/kask/abw-client.js`<br>`~/kask/hooks.js`<br>`~/kask/*.html`<br>`~/kask/server.js` | 13 | 8709 | 2026-06-25 | https://github.com/ilabra-axo/kask.git | 52 | 2 |
| rabble | `~/rabble/lib` | 71 | 34944 | 2026-06-06 | https://github.com/Replicant-Partners/rabble.git | 73 | 0 |
| glasses | `~/fermi/glasses` | 2 | 11 | 2026-08-20 | https://github.com/Replicant-Partners/fermi.git | 0 | 0 |
| abw-cli | `~/fermi/crates/abw-cli/src` | 13 | 3000 | 2026-08-07 | https://github.com/Replicant-Partners/fermi.git | 17 | 4 |

## fermi-console

| Endpoint | Legacy route(s) | Disposition | First call site |
| --- | --- | --- | --- |
| `/api/agents` | GET `handlers::agents::list_agents` src/api_server.rs:2903<br>POST `handlers::agents::create_agent_handler` src/api_server.rs:3838 | CAP-01 | `fermi/crates/fermi-console/src/api/client.rs:2547` |
| `/api/agents/:p` | GET `handlers::agents::get_agent_handler` src/api_server.rs:2910<br>PUT `handlers::agents::update_agent_handler` src/api_server.rs:3847<br>DELETE `handlers::agents::delete_agent_handler` src/api_server.rs:3851 | CAP-01 | `fermi/crates/fermi-console/src/api/client.rs:2582` |
| `/api/agents/:p/calibration` | GET `handlers::agents::get_agent_calibration_handler` src/api_server.rs:3289 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2337` |
| `/api/agents/:p/execute` | POST `handlers::execution::execute_agent_handler` src/api_server.rs:3908 | CAP-01 | `fermi/crates/fermi-console/src/abw_pacing.rs:10` |
| `/api/agents/:p/execute/stream` | POST `handlers::execution_stream::execute_agent_stream_handler` src/api_server.rs:3913 | CAP-01 | `fermi/crates/fermi-console/src/abw_pacing.rs:10` |
| `/api/agents/contributions` | GET `handlers::agents::agent_contributions_handler` src/api_server.rs:2932 | CAP-01 | `fermi/crates/fermi-console/src/api/client.rs:2350` |
| `/api/agents/fermi/messages` | **none: no legacy route** | — | `fermi/crates/fermi-console/src/chat.rs:209` |
| `/api/apps/fermi_forecast/workspaces` | **none: no legacy route** | — | `fermi/crates/fermi-console/src/api/client.rs:2689` |
| `/api/auth/me` | GET `handlers::auth::auth_me` src/api_server.rs:3655 | CAP-14 | `fermi/crates/fermi-console/src/api/client.rs:2249` |
| `/api/bayesops/pending/:p/accept` | POST `handlers::bayesops::accept_pending_handler` src/api_server.rs:4070 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2850` |
| `/api/bayesops/pending/:p/reject` | POST `handlers::bayesops::reject_pending_handler` src/api_server.rs:4074 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2864` |
| `/api/forecast-relationships` | POST/GET `handlers::relationships::legacy::create_relationship_handler, handlers::relationships::legacy::list_relationships_handler` src/api_server.rs:4711 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:3048` |
| `/api/forecast-relationships/:p` | DELETE `handlers::relationships::legacy::delete_relationship_handler` src/api_server.rs:4716 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:3060` |
| `/api/forecast-relationships/:p/propagate` | POST `handlers::relationships::legacy::propagate_relationship_handler` src/api_server.rs:4720 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:3087` |
| `/api/forecasts` | POST `handlers::forecasts::create_forecast_handler` src/api_server.rs:4535<br>GET `handlers::forecasts::list_forecasts_handler` src/api_server.rs:4539 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2268` |
| `/api/forecasts/:p` | GET `handlers::forecasts::get_forecast_handler` src/api_server.rs:4547<br>PUT `handlers::forecasts::update_forecast_handler` src/api_server.rs:4551<br>DELETE `handlers::forecasts::delete_forecast_handler` src/api_server.rs:4555 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2273` |
| `/api/forecasts/:p/access` | GET `handlers::collab::forecast_access_handler` src/api_server.rs:4637 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:3292` |
| `/api/forecasts/:p/activity` | GET `handlers::collab::forecast_activity_handler` src/api_server.rs:4641 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:3310` |
| `/api/forecasts/:p/annotations` | GET/POST `handlers::annotations::list_annotations_handler, handlers::annotations::create_annotation_handler` src/api_server.rs:4689 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:3415` |
| `/api/forecasts/:p/annotations/:p` | DELETE `handlers::annotations::delete_annotation_handler` src/api_server.rs:4698 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:3460` |
| `/api/forecasts/:p/annotations/:p/resolve` | POST `handlers::annotations::resolve_annotation_handler` src/api_server.rs:4694 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:3444` |
| `/api/forecasts/:p/cascade-provenance` | GET `handlers::forecast_benchmark::forecast_cascade_provenance_handler` src/api_server.rs:4604 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2900` |
| `/api/forecasts/:p/groups` | GET/PUT `handlers::relationships::membership::get_forecast_groups_handler, handlers::relationships::membership::set_forecast_groups_handler` src/api_server.rs:4757 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2960` |
| `/api/forecasts/:p/groups/:p` | POST/DELETE `handlers::relationships::membership::add_forecast_to_group_handler, handlers::relationships::membership::remove_forecast_from_group_handler` src/api_server.rs:4762 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2974` |
| `/api/forecasts/:p/history` | GET `handlers::forecast_git::forecast_history_handler` src/api_server.rs:4661 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:3369` |
| `/api/forecasts/:p/history/:p` | GET `handlers::forecast_git::forecast_diff_handler` src/api_server.rs:4665 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:3383` |
| `/api/forecasts/:p/invites` | POST/GET `handlers::invites::invite_to_forecast_handler, handlers::invites::list_forecast_invites_handler` src/api_server.rs:4705 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:3493` |
| `/api/forecasts/:p/resolve` | POST `handlers::forecasts::resolve_forecast_handler` src/api_server.rs:4559 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2307` |
| `/api/forecasts/:p/revert` | POST `handlers::forecast_git::forecast_revert_handler` src/api_server.rs:4669 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:3402` |
| `/api/forecasts/:p/schedules` | GET/PUT `handlers::forecasts::list_forecast_schedules_handler, handlers::forecasts::upsert_forecast_schedule_handler` src/api_server.rs:4572 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2629` |
| `/api/forecasts/:p/schedules/:p` | DELETE `handlers::forecasts::delete_forecast_schedule_handler` src/api_server.rs:4577 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2653` |
| `/api/forecasts/:p/schedules/:p/run` | POST `handlers::forecasts::record_schedule_run_handler` src/api_server.rs:4581 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2666` |
| `/api/forecasts/:p/shares` | GET/POST `handlers::shares::list_forecast_shares_handler, handlers::shares::create_forecast_share_handler` src/api_server.rs:4621 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:3146` |
| `/api/forecasts/:p/shares/:p` | DELETE `handlers::shares::revoke_forecast_share_handler` src/api_server.rs:4626 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:3168` |
| `/api/forecasts/:p/timeline` | GET `handlers::forecast_benchmark::forecast_timeline_handler` src/api_server.rs:4594 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2886` |
| `/api/forecasts/:p/update-probability` | POST `handlers::forecasts::update_probability_handler` src/api_server.rs:4567 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2324` |
| `/api/forecasts/:p/void` | POST `handlers::forecasts::void_forecast_handler` src/api_server.rs:4563 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2313` |
| `/api/forecasts/my-stats` | GET `handlers::forecasts::my_stats_handler` src/api_server.rs:4543 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2332` |
| `/api/forecasts/public` | GET `handlers::forecasts::public_forecasts_handler` src/api_server.rs:2824 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2359` |
| `/api/health` | GET `handlers::misc::health` src/api_server.rs:2820 | ops | `fermi/crates/fermi-console/src/api/client.rs:2798` |
| `/api/invites/:p` | DELETE `handlers::invites::revoke_invite_handler` src/api_server.rs:3762 | CAP-14 | `fermi/crates/fermi-console/src/api/client.rs:3567` |
| `/api/invites/:p/accept` | POST `handlers::invites::accept_invite_handler` src/api_server.rs:3771 | CAP-14 | `fermi/crates/fermi-console/src/api/client.rs:3555` |
| `/api/invites/:p/decline` | POST `handlers::invites::decline_invite_handler` src/api_server.rs:3758 | CAP-14 | `fermi/crates/fermi-console/src/api/client.rs:3561` |
| `/api/leaderboard` | GET `handlers::forecasts::leaderboard_handler` src/api_server.rs:4848 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2539` |
| `/api/me/invites` | GET `handlers::invites::list_my_invites_handler` src/api_server.rs:3750 | CAP-14 | `fermi/crates/fermi-console/src/api/client.rs:3517` |
| `/api/me/invites/sent` | GET `handlers::invites::list_sent_invites_handler` src/api_server.rs:3754 | CAP-14 | `fermi/crates/fermi-console/src/api/client.rs:3525` |
| `/api/orchestras/:p/members` | GET `handlers::orchestras::list_orchestra_members_handler` src/api_server.rs:4970 | CAP-04 | `fermi/crates/fermi-console/src/api/client.rs:2576` |
| `/api/orchestras/fermi/members` | **none: no legacy route** | — | `fermi/crates/fermi-console/src/api/client.rs:2553` |
| `/api/pending-cascades` | GET `handlers::pending_cascades::list_pending_cascades_handler` src/api_server.rs:4768 | CAP-06 | `fermi/crates/fermi-console/src/api/client.rs:3107` |
| `/api/pending-cascades/:p/apply` | POST `handlers::relationships::apply::apply_pending_cascade_handler` src/api_server.rs:4772 | CAP-06 | `fermi/crates/fermi-console/src/api/client.rs:3117` |
| `/api/pending-cascades/:p/dismiss` | POST `handlers::pending_cascades::dismiss_pending_cascade_handler` src/api_server.rs:4776 | CAP-06 | `fermi/crates/fermi-console/src/api/client.rs:3130` |
| `/api/polymarket/check-resolutions` | POST `handlers::polymarket::check_resolutions_handler` src/api_server.rs:4879 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2525` |
| `/api/polymarket/link` | POST `handlers::polymarket::link_handler` src/api_server.rs:4867 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2488` |
| `/api/polymarket/search` | POST `handlers::polymarket::search_handler` src/api_server.rs:4859 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2444` |
| `/api/polymarket/snapshot` | POST `handlers::polymarket::snapshot_handler` src/api_server.rs:4863 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2512` |
| `/api/portfolios` | POST `handlers::forecasts::create_portfolio_handler` src/api_server.rs:4793<br>GET `handlers::forecasts::list_portfolios_handler` src/api_server.rs:4797 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2371` |
| `/api/portfolios/:p` | DELETE/PATCH `handlers::forecasts::delete_portfolio_handler, handlers::forecasts::patch_portfolio_handler` src/api_server.rs:4805 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2413` |
| `/api/portfolios/:p/access` | GET `handlers::collab::portfolio_access_handler` src/api_server.rs:4833 | CAP-14 | `fermi/crates/fermi-console/src/api/client.rs:3299` |
| `/api/portfolios/:p/activity` | GET `handlers::collab::portfolio_activity_handler` src/api_server.rs:4837 | CAP-05 | `fermi/crates/fermi-console/src/api/client.rs:3323` |
| `/api/portfolios/:p/forecasts` | GET/POST `handlers::forecasts::list_portfolio_forecasts_handler, handlers::forecasts::add_forecast_to_portfolio_handler` src/api_server.rs:4810 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2392` |
| `/api/portfolios/:p/forecasts/:p` | DELETE `handlers::forecasts::remove_forecast_from_portfolio_handler` src/api_server.rs:4815 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2405` |
| `/api/portfolios/:p/invites` | POST/GET `handlers::invites::invite_to_portfolio_handler, handlers::invites::list_portfolio_invites_handler` src/api_server.rs:4842 | CAP-14 | `fermi/crates/fermi-console/src/api/client.rs:3502` |
| `/api/portfolios/:p/shares` | GET/POST `handlers::shares::list_portfolio_shares_handler, handlers::shares::create_portfolio_share_handler` src/api_server.rs:4820 | CAP-14 | `fermi/crates/fermi-console/src/api/client.rs:3178` |
| `/api/portfolios/:p/shares/:p` | DELETE `handlers::shares::revoke_portfolio_share_handler` src/api_server.rs:4825 | CAP-14 | `fermi/crates/fermi-console/src/api/client.rs:3197` |
| `/api/portfolios/:p/stats` | GET `handlers::forecasts::portfolio_stats_handler` src/api_server.rs:4801 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2381` |
| `/api/rbac/self-check` | GET `handlers::rbac_self_check::rbac_self_check_handler` src/api_server.rs:5048 | CAP-14 | `fermi/crates/fermi-console/src/api/client.rs:2815` |
| `/api/relationship-groups` | POST/GET `handlers::relationships::groups::create_group_handler, handlers::relationships::groups::list_groups_handler` src/api_server.rs:4725 | CAP-06 | `fermi/crates/fermi-console/src/api/client.rs:2919` |
| `/api/relationship-groups/:p` | GET/PATCH/DELETE `handlers::relationships::groups::get_group_handler, handlers::relationships::groups::patch_group_handler, handlers::relationships::groups::delete_group_handler` src/api_server.rs:4730 | CAP-06 | `fermi/crates/fermi-console/src/api/client.rs:2925` |
| `/api/relationship-groups/:p/propagate` | POST `handlers::relationships::groups::preview_group_propagation_handler` src/api_server.rs:4745 | CAP-10 | `fermi/crates/fermi-console/src/api/client.rs:3012` |
| `/api/teams` | POST `handlers::teams::create_team_handler` src/api_server.rs:3663<br>GET `handlers::teams::list_teams_handler` src/api_server.rs:3664 | CAP-02 | `fermi/crates/fermi-console/src/api/client.rs:2612` |
| `/api/teams/:p` | GET `handlers::teams::get_team_handler` src/api_server.rs:3665<br>DELETE `handlers::teams::delete_team_handler` src/api_server.rs:3669 | CAP-02 | `fermi/crates/fermi-console/src/api/client.rs:2617` |
| `/api/teams/:p/activity` | GET `handlers::collab::team_activity_handler` src/api_server.rs:3728 | CAP-02 | `fermi/crates/fermi-console/src/api/client.rs:3340` |
| `/api/teams/:p/contributions` | GET `handlers::collab::team_contributions_handler` src/api_server.rs:3732 | CAP-02 | `fermi/crates/fermi-console/src/api/client.rs:3355` |
| `/api/teams/:p/invites` | POST/GET `handlers::invites::invite_to_team_handler, handlers::invites::list_team_invites_handler` src/api_server.rs:3703 | CAP-14 | `fermi/crates/fermi-console/src/api/client.rs:3511` |
| `/api/teams/:p/members` | POST `handlers::teams::add_member_handler` src/api_server.rs:3673<br>GET `handlers::teams::list_members_handler` src/api_server.rs:3677 | CAP-02 | `fermi/crates/fermi-console/src/api/client.rs:3231` |
| `/api/teams/:p/members/:p` | DELETE `handlers::teams::remove_member_handler` src/api_server.rs:3681<br>PUT `handlers::teams::update_member_role_handler` src/api_server.rs:3685 | CAP-02 | `fermi/crates/fermi-console/src/api/client.rs:3237` |
| `/api/teams/:p/members/:p/capabilities` | PUT `handlers::teams::set_member_capabilities_handler` src/api_server.rs:3695 | CAP-02 | `fermi/crates/fermi-console/src/api/client.rs:3277` |
| `/api/teams/:p/ops` | GET `handlers::ops::team_ops_handler` src/api_server.rs:3740 | CAP-02 | `fermi/crates/fermi-console/src/api/client.rs:3472` |
| `/api/teams/:p/shared` | GET `handlers::collab::team_shared_handler` src/api_server.rs:3724 | CAP-02 | `fermi/crates/fermi-console/src/api/client.rs:3481` |
| `/api/users/lookup` | GET `handlers::users::lookup_user_by_email_handler` src/api_server.rs:4396 | CAP-14 | `fermi/crates/fermi-console/src/api/client.rs:3579` |
| `/api/wallet` | GET `handlers::wallet::get_wallet_handler` src/api_server.rs:4362 | replaced:acn | `fermi/crates/fermi-console/src/api/client.rs:2256` |
| `/api/workspaces/:p/actions/:p` | ~prefix `2 routes under prefix`  | — | `fermi/crates/fermi-console/src/api/client.rs:2785` |
| `/api/workspaces/:p/bayesops/state` | GET `handlers::bayesops::workspace_bayesops_state_handler` src/api_server.rs:4066 | tenant:fermi-console | `fermi/crates/fermi-console/src/api/client.rs:2837` |
| `/api/workspaces/:p/dependencies` | GET/POST `handlers::workspace::outputs::list_dependencies_handler, handlers::workspace::outputs::add_dependency_handler` src/api_server.rs:4248 | CAP-15 | `fermi/crates/fermi-console/src/api/client.rs:2758` |
| `/api/workspaces/:p/messages` | POST `handlers::workspace::post_workspace_message_handler` src/api_server.rs:4266<br>GET `handlers::workspace::get_workspace_messages_handler` src/api_server.rs:4270 | CAP-15 | `fermi/crates/fermi-console/src/api/client.rs:2716` |
| `/api/workspaces/:p/outputs` | GET `handlers::workspace::outputs::list_outputs_handler` src/api_server.rs:4237 | CAP-15 | `fermi/crates/fermi-console/src/api/client.rs:2746` |
| `/api/workspaces/:p/outputs/:p` | PUT/GET/DELETE `handlers::workspace::outputs::set_output_handler, handlers::workspace::outputs::get_output_handler, handlers::workspace::outputs::delete_output_handler` src/api_server.rs:4241 | CAP-15 | `fermi/crates/fermi-console/src/api/client.rs:2728` |
| `/api/workspaces/:p/refit` | POST `handlers::workspace::refit::refit_workspace_handler` src/api_server.rs:4083 | CAP-15 | `fermi/crates/fermi-console/src/api/client.rs:2875` |

## simops

| Endpoint | Legacy route(s) | Disposition | First call site |
| --- | --- | --- | --- |
| `/api/simops/cascade` | POST `handlers::simops::cascade_handler` src/api_server.rs:3989 | tenant:simops | `kask/kask-sim-client.js:1338` |
| `/api/workspaces/:p/actions/pending.` | **none: no legacy route** | — | `kask/kask-sim-client.js:976` |

## dpp-adaptogen

| Endpoint | Legacy route(s) | Disposition | First call site |
| --- | --- | --- | --- |
| `/api/agents/:p/metrics` | GET `handlers::metrics::agent_metrics_handler` src/api_server.rs:2967 | ops | `fermi/static/adaptogen-lab/index.html:4434` |
| `/api/dpp/p/:p` | GET `handlers::workspace::dpp_publish::public_dpp_handler` src/api_server.rs:2784 | tenant:dpp | `fermi/static/adaptogen-lab/passport.html:189` |

## kask-projects

No core endpoints found in client sources.

## abw-web-ui

| Endpoint | Legacy route(s) | Disposition | First call site |
| --- | --- | --- | --- |
| `/api/admin/agent-ownership-audit` | GET `handlers::admin::admin_agent_ownership_audit_handler` src/api_server.rs:5020 | ops | `fermi/templates/admin.html:1793` |
| `/api/admin/agent-ownership-reassign` | POST `handlers::admin::admin_agent_ownership_reassign_handler` src/api_server.rs:5024 | ops | `fermi/templates/admin.html:1837` |
| `/api/admin/agents` | GET `handlers::admin::admin_list_agents_handler` src/api_server.rs:4931 | ops | `fermi/templates/admin.html:1478` |
| `/api/admin/apps` | GET `handlers::admin::admin_list_apps_handler` src/api_server.rs:5012 | ops | `fermi/templates/admin.html:1602` |
| `/api/admin/apps/:p/visibility` | PUT `handlers::admin::admin_set_app_visibility_handler` src/api_server.rs:5016 | ops | `fermi/templates/admin.html:1663` |
| `/api/admin/economics/platform` | GET `handlers::economics::platform_economics_handler` src/api_server.rs:4910 | replaced:acn | `fermi/templates/admin.html:1293` |
| `/api/admin/impersonate` | POST `handlers::impersonation::start_impersonation_handler` src/api_server.rs:4889 | ops | `fermi/templates/admin.html:1415` |
| `/api/admin/impersonate/end` | POST `handlers::impersonation::end_impersonation_handler` src/api_server.rs:4893 | ops | `fermi/static/js/impersonation-banner.js:21` |
| `/api/admin/schema-health` | GET `handlers::admin::admin_schema_health_handler` src/api_server.rs:5074 | ops | `fermi/templates/loops.html:772` |
| `/api/admin/stats` | GET `handlers::admin::admin_stats_handler` src/api_server.rs:4915 | ops | `fermi/templates/admin.html:826` |
| `/api/admin/users` | GET `handlers::admin::admin_list_users_handler` src/api_server.rs:4919 | ops | `fermi/static/js/widgets/xaman-ek.js:822` |
| `/api/admin/users/:p/grant` | POST `handlers::admin::admin_grant_credits_handler` src/api_server.rs:4923 | replaced:acn | `fermi/static/js/widgets/xaman-ek.js:860` |
| `/api/admin/waitlist` | GET/POST `handlers::admin::admin_list_waitlist_handler, handlers::admin::admin_add_waitlist_handler` src/api_server.rs:5085 | ops | `fermi/templates/admin.html:1888` |
| `/api/admin/waitlist/invite` | POST `handlers::admin::admin_invite_waitlist_handler` src/api_server.rs:5090 | ops | `fermi/templates/admin.html:1966` |
| `/api/admin/workspaces/:p/grant` | POST `handlers::admin::admin_grant_workspace_credits_handler` src/api_server.rs:4927 | replaced:acn | `fermi/templates/admin.html:1251` |
| `/api/agents` | GET `handlers::agents::list_agents` src/api_server.rs:2903<br>POST `handlers::agents::create_agent_handler` src/api_server.rs:3838 | CAP-01 | `fermi/static/js/widgets/contract-builder.js:1813` |
| `/api/agents/:p` | GET `handlers::agents::get_agent_handler` src/api_server.rs:2910<br>PUT `handlers::agents::update_agent_handler` src/api_server.rs:3847<br>DELETE `handlers::agents::delete_agent_handler` src/api_server.rs:3851 | CAP-01 | `fermi/static/js/widgets/agent-fields.js:1737` |
| `/api/agents/:p/:p` | **none: no legacy route** | — | `fermi/static/js/widgets/agent-fields.js:1870` |
| `/api/agents/:p/allocate` | POST `handlers::agent_wallet::allocate_handler` src/api_server.rs:3466 | replaced:acn | `fermi/templates/agent_detail.html:5368` |
| `/api/agents/:p/archive` | POST `handlers::lifecycle::archive_agent_handler` src/api_server.rs:3872 | CAP-01 | `fermi/templates/admin.html:1570` |
| `/api/agents/:p/auto-collect` | PUT `handlers::agent_wallet::set_auto_collect_handler` src/api_server.rs:3470 | replaced:acn | `fermi/static/js/widgets/agent-fields.js:1297` |
| `/api/agents/:p/avatar` | GET `handlers::agents::get_cached_avatar` src/api_server.rs:2951 | CAP-01 | `fermi/templates/agent_detail.html:929` |
| `/api/agents/:p/avatar/generate` | POST `handlers::agents::generate_avatar` src/api_server.rs:3885 | CAP-01 | `fermi/templates/agent_detail.html:2518` |
| `/api/agents/:p/calibration` | GET `handlers::agents::get_agent_calibration_handler` src/api_server.rs:3289 | tenant:fermi-console | `fermi/templates/observatory.html:1085` |
| `/api/agents/:p/collect` | POST `handlers::agent_wallet::collect_handler` src/api_server.rs:3462 | replaced:acn | `fermi/static/js/widgets/agent-fields.js:1286` |
| `/api/agents/:p/competition-stats` | GET `handlers::agents::agent_competition_stats_handler` src/api_server.rs:3301 | CAP-01 | `fermi/templates/specimen.html:1218` |
| `/api/agents/:p/consolidate` | POST `handlers::consolidation::consolidate_agent_handler` src/api_server.rs:3931 | CAP-06 | `fermi/static/js/widgets/agent-fields.js:1255` |
| `/api/agents/:p/consolidation/jobs/:p` | GET `handlers::consolidation::get_consolidation_job_handler` src/api_server.rs:3945 | CAP-06 | `fermi/templates/agent_detail.html:3878` |
| `/api/agents/:p/coordination-notes` | GET `handlers::loops::agent_coordination_notes_handler` src/api_server.rs:3142 | CAP-04 | `fermi/templates/specimen.html:644` |
| `/api/agents/:p/dependencies` | GET `handlers::agents::get_agent_dependencies_handler` src/api_server.rs:3285 | CAP-01 | `fermi/templates/workspace.html:2721` |
| `/api/agents/:p/dreaming` | GET `handlers::dreaming_maturity::agent_dreaming_maturity_handler` src/api_server.rs:3935 | CAP-06 | `fermi/templates/observatory.html:1089` |
| `/api/agents/:p/dreaming/topup` | POST `handlers::consolidation::topup_dreaming_budget_handler` src/api_server.rs:3926 | CAP-06 | `fermi/static/js/widgets/agent-fields.js:1244` |
| `/api/agents/:p/embeddings/export` | GET `handlers::agents::embeddings_export_handler` src/api_server.rs:3903 | CAP-01 | `fermi/templates/agent_detail.html:4481` |
| `/api/agents/:p/embeddings/export/consent` | POST `handlers::agents::embeddings_export_consent_handler` src/api_server.rs:3899 | CAP-01 | `fermi/templates/agent_detail.html:4539` |
| `/api/agents/:p/embeddings/stats` | GET `handlers::agents::embeddings_stats_handler` src/api_server.rs:3895 | CAP-01 | `fermi/templates/agent_detail.html:2368` |
| `/api/agents/:p/episodes` | GET `handlers::consolidation::get_agent_episodes_handler` src/api_server.rs:2955 | CAP-06 | `fermi/templates/agent_detail.html:4822` |
| `/api/agents/:p/eval/run` | POST `handlers::eval::trigger_eval_run_handler` src/api_server.rs:3040 | CAP-05 | `fermi/templates/agent_detail.html:3752` |
| `/api/agents/:p/eval/runs` | GET `handlers::eval::list_eval_runs_handler` src/api_server.rs:3044 | CAP-05 | `fermi/templates/agent_detail.html:3438` |
| `/api/agents/:p/eval/runs/:p/signals` | GET `handlers::eval::list_eval_signals_handler` src/api_server.rs:3048 | CAP-05 | `fermi/templates/agent_detail.html:3665` |
| `/api/agents/:p/eval/test-cases` | GET `handlers::eval::list_eval_test_cases_handler` src/api_server.rs:2972<br>POST `handlers::eval::create_eval_test_case_handler` src/api_server.rs:2976 | CAP-05 | `fermi/templates/agent_detail.html:3397` |
| `/api/agents/:p/eval/test-cases/:p` | PUT `handlers::eval::update_eval_test_case_handler` src/api_server.rs:3024<br>DELETE `handlers::eval::delete_eval_test_case_handler` src/api_server.rs:3028 | CAP-05 | `fermi/templates/agent_detail.html:3642` |
| `/api/agents/:p/eval/test-cases/generate-rubrics` | POST `handlers::eval::generate_rubrics_handler` src/api_server.rs:3032 | CAP-05 | `fermi/templates/observatory.html:2733` |
| `/api/agents/:p/evolution` | GET `handlers::evolution::agent_evolution_handler` src/api_server.rs:3941 | CAP-01 | `fermi/templates/agent_detail.html:1909` |
| `/api/agents/:p/execute` | POST `handlers::execution::execute_agent_handler` src/api_server.rs:3908 | CAP-01 | `fermi/static/js/widgets/xaman-ek.js:769` |
| `/api/agents/:p/fidelity` | GET `handlers::agents::agent_fidelity_handler` src/api_server.rs:3295 | CAP-01 | `fermi/templates/specimen.html:1217` |
| `/api/agents/:p/fork` | POST `handlers::lifecycle::fork_agent_handler` src/api_server.rs:3856 | CAP-01 | `fermi/static/js/widgets/fork-modal.js:80` |
| `/api/agents/:p/fork-pricing` | PUT `handlers::lifecycle::update_fork_pricing_handler` src/api_server.rs:3860 | CAP-01 | `fermi/static/js/widgets/agent-fields.js:1272` |
| `/api/agents/:p/funding` | GET `handlers::agent_funding::get_agent_funding_handler` src/api_server.rs:2988 | CAP-01 | `fermi/static/js/widgets/agent-fields.js:1366` |
| `/api/agents/:p/mcp-servers` | GET `handlers::agents::get_agent_mcp_servers_handler` src/api_server.rs:2998 | CAP-14 | `fermi/static/js/widgets/agent-instruments.js:373` |
| `/api/agents/:p/mcp-servers/test` | POST `handlers::agents::test_agent_mcp_server_handler` src/api_server.rs:3002 | CAP-14 | `fermi/static/js/widgets/agent-instruments.js:341` |
| `/api/agents/:p/metrics` | GET `handlers::metrics::agent_metrics_handler` src/api_server.rs:2967 | ops | `fermi/templates/agent_detail.html:2011` |
| `/api/agents/:p/ontology` | GET `handlers::ontology::get_ontology` src/api_server.rs:3403 | CAP-01 | `fermi/templates/agent_detail.html:2174` |
| `/api/agents/:p/orchestras` | GET `handlers::orchestras::agent_orchestras_handler` src/api_server.rs:4997 | CAP-04 | `fermi/templates/agent_detail.html:5065` |
| `/api/agents/:p/projections` | GET `handlers::ontology::get_agent_projections` src/api_server.rs:3480 | tenant:fermi-console | `fermi/templates/agent_detail.html:2351` |
| `/api/agents/:p/projections/temporal` | GET `handlers::ontology::get_temporal_projections` src/api_server.rs:3488 | tenant:fermi-console | `fermi/templates/projector.html:815` |
| `/api/agents/:p/publish` | POST `handlers::lifecycle::publish_agent_handler` src/api_server.rs:3868 | CAP-01 | `fermi/templates/admin.html:1543` |
| `/api/agents/:p/publish-checks` | GET `handlers::lifecycle::publish_checks_handler` src/api_server.rs:3864 | CAP-01 | `fermi/templates/admin.html:1514` |
| `/api/agents/:p/published-tools` | GET `handlers::agents::get_agent_published_tools_handler` src/api_server.rs:3011 | CAP-01 | `fermi/static/js/widgets/agent-instruments.js:803` |
| `/api/agents/:p/restore` | POST `handlers::lifecycle::restore_agent_handler` src/api_server.rs:3876 | CAP-01 | `fermi/templates/admin.html:1584` |
| `/api/agents/:p/sessions` | **none: no legacy route** | — | `fermi/templates/ontology.html:774` |
| `/api/agents/:p/versions` | GET `handlers::agents::list_agent_versions_handler` src/api_server.rs:3372 | ops | `fermi/static/js/widgets/agent-fields.js:1703` |
| `/api/agents/:p/versions/:p/diff` | GET `handlers::agents::diff_agent_version_handler` src/api_server.rs:3385 | ops | `fermi/static/js/widgets/agent-fields.js:929` |
| `/api/agents/:p/versions/:p/restore` | POST `handlers::agents::restore_agent_version_handler` src/api_server.rs:3880 | ops | `fermi/static/js/widgets/agent-fields.js:954` |
| `/api/agents/:p/versions/timeline` | GET `handlers::agents::agent_version_timeline_handler` src/api_server.rs:3394 | ops | `fermi/templates/observatory.html:1107` |
| `/api/agents/:p/wallet` | GET `handlers::agent_wallet::get_agent_wallet_handler` src/api_server.rs:3454 | replaced:acn | `fermi/static/js/widgets/agent-fields.js:1186` |
| `/api/agents/creation-guide` | GET `handlers::wizard::creation_guide_handler` src/api_server.rs:3792 | CAP-01 | `fermi/templates/agent_create.html:1512` |
| `/api/agents/curated` | GET `handlers::agents::list_curated_agents_handler` src/api_server.rs:2914 | CAP-01 | `fermi/templates/profile.html:1076` |
| `/api/agents/embedding_projector_guide/execute` | **none: no legacy route** | — | `fermi/templates/projector.html:1262` |
| `/api/agents/generate-ontology` | POST `handlers::wizard::generate_ontology_handler` src/api_server.rs:3784 | CAP-01 | `fermi/templates/agent_create.html:1444` |
| `/api/agents/generate-prompt` | POST `handlers::wizard::generate_prompt_handler` src/api_server.rs:3788 | CAP-01 | `fermi/templates/agent_create.html:1486` |
| `/api/agents/import` | POST `handlers::agents::import_agent_handler` src/api_server.rs:3839 | CAP-01 | `fermi/templates/agent_create.html:1174` |
| `/api/agents/mine` | GET `handlers::agents::list_my_agents_handler` src/api_server.rs:3843 | CAP-01 | `fermi/templates/contract_builder.html:175` |
| `/api/agents/{{` | **none: no legacy route** | — | `fermi/templates/agent_execute.html:11` |
| `/api/apps` | GET `handlers::apps::list_apps_handler` src/api_server.rs:2940<br>POST `handlers::apps::create_app_handler` src/api_server.rs:3955 | CAP-15 | `fermi/templates/apps.html:263` |
| `/api/apps/:p` | GET `handlers::apps::get_app_handler` src/api_server.rs:2941<br>PUT `handlers::apps::update_app_handler_full` src/api_server.rs:3956 | CAP-15 | `fermi/templates/app_detail.html:348` |
| `/api/apps/:p/archive` | POST `handlers::apps::archive_app_handler` src/api_server.rs:3976 | CAP-15 | `fermi/templates/app_detail.html:671` |
| `/api/apps/:p/publish` | POST `handlers::apps::publish_app_handler` src/api_server.rs:3972 | CAP-15 | `fermi/templates/app_detail.html:514` |
| `/api/apps/:p/workspaces` | POST `handlers::apps::spawn_workspace_handler` src/api_server.rs:3960<br>GET `handlers::apps::list_app_workspaces_handler` src/api_server.rs:3964 | CAP-15 | `fermi/templates/app_detail.html:537` |
| `/api/auth/api-keys` | GET `handlers::auth::list_api_keys` src/api_server.rs:3656<br>POST `handlers::auth::create_api_key` src/api_server.rs:3657 | CAP-14 | `fermi/templates/settings.html:355` |
| `/api/auth/api-keys/:p` | DELETE `handlers::auth::revoke_api_key` src/api_server.rs:3658 | CAP-14 | `fermi/templates/settings.html:436` |
| `/api/auth/me` | GET `handlers::auth::auth_me` src/api_server.rs:3655 | CAP-14 | `fermi/static/js/auth.js:13` |
| `/api/bestiary` | GET `handlers::bestiary::bestiary_handler` src/api_server.rs:3319 | CAP-01 | `fermi/templates/bestiary.html:417` |
| `/api/bestiary/cards` | GET `handlers::specimen::bestiary_cards_handler` src/api_server.rs:3358 | tenant:rabble | `fermi/templates/bestiary.html:418` |
| `/api/billing/checkout` | POST `handlers::billing::billing_checkout_handler` src/api_server.rs:4376 | replaced:acn | `fermi/templates/dashboard.html:2195` |
| `/api/billing/tiers` | GET `handlers::billing::billing_tiers_handler` src/api_server.rs:4372 | replaced:acn | `fermi/templates/dashboard.html:1302` |
| `/api/contacts` | GET/POST `handlers::social::list_contacts_handler, handlers::social::add_contact_handler` src/api_server.rs:5366 | crud:misc | `fermi/templates/dashboard.html:2033` |
| `/api/contacts/:p` | PUT/DELETE `handlers::social::update_contact_handler, handlers::social::remove_contact_handler` src/api_server.rs:5371 | crud:misc | `fermi/templates/dashboard.html:2078` |
| `/api/contracts/compile` | POST `handlers::contracts::compile_handler` src/api_server.rs:3803 | CAP-03 | `fermi/static/js/widgets/contract-builder.js:1499` |
| `/api/contracts/decompile` | ~prefix `1 routes under prefix`  | — | `fermi/static/js/widgets/contract-builder.js:1749` |
| `/api/contracts/suggest` | POST `handlers::contracts::suggest_handler` src/api_server.rs:3819 | CAP-03 | `fermi/static/js/widgets/contract-builder.js:618` |
| `/api/contracts/tool-request` | POST `handlers::contracts::tool_request_handler` src/api_server.rs:3833 | CAP-03 | `fermi/static/js/widgets/contract-builder.js:1837` |
| `/api/contracts/tools` | GET `handlers::contracts::tool_names_handler` src/api_server.rs:3807 | CAP-03 | `fermi/static/js/widgets/contract-builder.js:1696` |
| `/api/contracts/types` | GET `handlers::contracts::types_handler` src/api_server.rs:3815 | CAP-03 | `fermi/static/js/widgets/contract-builder.js:1151` |
| `/api/declarations` | GET `handlers::loops::list_declarations_handler` src/api_server.rs:3256 | CAP-06 | `fermi/templates/declarations.html:219` |
| `/api/ecology/overview` | GET `handlers::ecology::ecology_overview_handler` src/api_server.rs:3625 | crud:misc | `fermi/templates/ecology.html:732` |
| `/api/ecology/specimens` | GET `handlers::ecology::ecology_specimens_handler` src/api_server.rs:3629 | tenant:rabble | `fermi/templates/ecology.html:732` |
| `/api/episodes/:p` | GET `handlers::metrics::get_episode_detail_handler` src/api_server.rs:2959 | CAP-01 | `fermi/templates/agent_detail.html:4899` |
| `/api/episodes/:p/contradict` | POST `handlers::loops::contradict_field_handler` src/api_server.rs:3239 | CAP-06 | `fermi/templates/trace.html:2234` |
| `/api/episodes/:p/lineage` | GET `handlers::specimen::episode_lineage_handler` src/api_server.rs:3351 | tenant:rabble | `fermi/templates/grounding.html:250` |
| `/api/episodes/:p/probe` | POST `handlers::loops::probe_field_handler` src/api_server.rs:3229 | CAP-06 | `fermi/templates/trace.html:2157` |
| `/api/episodes/:p/trace` | GET `handlers::loops::episode_trace_handler` src/api_server.rs:3202 | CAP-06 | `fermi/templates/grounding.html:249` |
| `/api/episodes/recent` | GET `handlers::specimen::recent_episodes_handler` src/api_server.rs:3346 | tenant:rabble | `fermi/templates/loops.html:932` |
| `/api/evaluators` | GET `handlers::loops::list_evaluators_handler` src/api_server.rs:3264 | CAP-06 | `fermi/templates/loops.html:936` |
| `/api/fleet` | GET `handlers::fleet::fleet_handler` src/api_server.rs:3023 | CAP-02 | `fermi/templates/dashboard.html:2643` |
| `/api/gates` | GET `handlers::loops::list_gates_handler` src/api_server.rs:3172 | CAP-06 | `fermi/templates/gate.html:299` |
| `/api/gates/:p/decisions` | GET `handlers::loops::gate_decisions_handler` src/api_server.rs:3174 | CAP-06 | `fermi/templates/gate.html:298` |
| `/api/gates/:p/decisions/:p/review` | POST `handlers::loops::review_gate_decision_handler` src/api_server.rs:3189 | CAP-06 | `fermi/templates/gate.html:250` |
| `/api/gates/enforcement` | GET `handlers::gates::gates_handler` src/api_server.rs:3334 | CAP-11 | `fermi/templates/loops.html:931` |
| `/api/invites/:p/decline` | POST `handlers::invites::decline_invite_handler` src/api_server.rs:3758 | CAP-14 | `fermi/templates/invite_landing.html:312` |
| `/api/invites/by-token/:p` | GET `handlers::invites::get_invite_by_token_handler` src/api_server.rs:2832 | CAP-14 | `fermi/templates/invite_landing.html:213` |
| `/api/invites/by-token/:p/accept` | POST `handlers::invites::accept_invite_by_token_handler` src/api_server.rs:3775 | CAP-14 | `fermi/templates/invite_landing.html:256` |
| `/api/loops` | GET `handlers::loops::list_loops_handler` src/api_server.rs:3155 | CAP-06 | `fermi/templates/loops.html:930` |
| `/api/loops/actions` | GET `handlers::loops::list_loop_actions_handler` src/api_server.rs:3157 | CAP-06 | `fermi/templates/loops.html:931` |
| `/api/marketplace/history` | GET `handlers::marketplace::marketplace_history_handler` src/api_server.rs:5128 | tenant:shopping | `fermi/templates/marketplace.html:699` |
| `/api/marketplace/listings:p` | **none: no legacy route** | — | `fermi/templates/marketplace.html:660` |
| `/api/marketplace/match` | POST `handlers::marketplace::marketplace_match_handler` src/api_server.rs:5116 | tenant:shopping | `fermi/templates/marketplace.html:575` |
| `/api/me/apps-health` | GET `handlers::apps::apps_health_handler` src/api_server.rs:3368 | ops | `fermi/templates/dashboard.html:1513` |
| `/api/me/eval/runs/batch` | POST `handlers::eval::batch_eval_run_handler` src/api_server.rs:3036 | CAP-14 | `fermi/templates/observatory.html:3147` |
| `/api/me/loop-health` | GET `handlers::agents::loop_health_handler` src/api_server.rs:3309 | ops | `fermi/templates/dashboard.html:1640` |
| `/api/me/providers` | GET `handlers::agents::list_my_providers_handler` src/api_server.rs:3305 | CAP-14 | `fermi/templates/observatory.html:780` |
| `/api/me/rounds` | GET `handlers::rounds::rounds_handler` src/api_server.rs:3315 | CAP-14 | `fermi/templates/observatory.html:3035` |
| `/api/metrics/platform` | GET `handlers::metrics::platform_metrics_handler` src/api_server.rs:2963 | ops | `fermi/templates/dashboard.html:1362` |
| `/api/models/catalogue` | GET `handlers::agents::model_catalogue_handler` src/api_server.rs:2947 | CAP-01 | `fermi/templates/agent_create.html:1200` |
| `/api/notifications` | GET `handlers::profile::list_notifications_handler` src/api_server.rs:4412 | CAP-14 | `fermi/static/js/widgets/nav.js:274` |
| `/api/notifications/read-all` | PUT `handlers::profile::mark_all_notifications_read_handler` src/api_server.rs:4420 | CAP-14 | `fermi/static/js/widgets/nav.js:335` |
| `/api/observatory/agents/:p/anomalies` | GET `handlers::observatory::list_agent_anomalies_handler` src/api_server.rs:3119 | CAP-05 | `fermi/templates/observatory.html:1084` |
| `/api/observatory/agents/:p/backfill-social` | POST `handlers::observatory::backfill_social_handler` src/api_server.rs:3098 | CAP-05 | `fermi/templates/observatory.html:3120` |
| `/api/observatory/agents/:p/dyads` | GET `handlers::observatory::list_agent_dyads_handler` src/api_server.rs:3115 | CAP-05 | `fermi/templates/observatory.html:1083` |
| `/api/observatory/agents/:p/loops` | GET `handlers::loops::agent_loops_handler` src/api_server.rs:3138 | CAP-06 | `fermi/templates/observatory.html:1090` |
| `/api/observatory/agents/:p/relationships` | GET `handlers::observatory::agent_relationships_handler` src/api_server.rs:3106 | CAP-05 | `fermi/templates/observatory.html:1087` |
| `/api/observatory/agents/:p/scan` | POST `handlers::observatory::trigger_agent_scan_handler` src/api_server.rs:3268 | CAP-05 | `fermi/static/js/widgets/xaman-ek.js:894` |
| `/api/observatory/agents/:p/timeline` | GET `handlers::observatory::get_agent_timeline_handler` src/api_server.rs:3111 | CAP-05 | `fermi/templates/observatory.html:1082` |
| `/api/observatory/dyads/:p` | PATCH `handlers::observatory::patch_dyad_profile_handler` src/api_server.rs:3102 | CAP-05 | `fermi/templates/observatory.html:2662` |
| `/api/observatory/dyads/auto-form` | POST `handlers::observatory::auto_form_dyads_handler` src/api_server.rs:3094 | CAP-05 | `fermi/templates/observatory.html:1016` |
| `/api/observatory/fleet/agents` | GET `handlers::observatory::fleet_agents_handler` src/api_server.rs:3062 | CAP-02 | `fermi/templates/observatory.html:3152` |
| `/api/observatory/fleet/scan` | POST `handlers::observatory::fleet_scan_handler` src/api_server.rs:3058 | CAP-02 | `fermi/templates/observatory.html:996` |
| `/api/observatory/fleet/summary` | GET `handlers::observatory::fleet_summary_handler` src/api_server.rs:3054 | CAP-02 | `fermi/templates/observatory.html:745` |
| `/api/observatory/hitl` | GET `handlers::observatory::list_hitl_queue_handler` src/api_server.rs:3272 | CAP-05 | `fermi/templates/observatory_hitl.html:121` |
| `/api/observatory/hitl/:p/action` | POST `handlers::observatory::record_hitl_action_handler` src/api_server.rs:3276 | CAP-05 | `fermi/templates/observatory_hitl.html:174` |
| `/api/observatory/loops/dreaming/maturity` | GET `handlers::dreaming_maturity::fleet_dreaming_maturity_handler` src/api_server.rs:3076 | CAP-06 | `fermi/templates/observatory.html:747` |
| `/api/ontology-templates` | GET `handlers::wizard::list_ontology_templates_handler` src/api_server.rs:3780 | CAP-15 | `fermi/templates/agent_create.html:1383` |
| `/api/orchestras` | GET `handlers::orchestras::list_orchestras_handler` src/api_server.rs:4966 | CAP-04 | `fermi/templates/agent_detail.html:5066` |
| `/api/orchestras/:p/manager-effect` | GET `handlers::orchestras::orchestra_manager_effect_handler` src/api_server.rs:5004 | CAP-04 | `fermi/templates/agent_detail.html:1708` |
| `/api/orchestras/:p/requests` | GET/POST `handlers::orchestras::list_orchestra_requests_handler, handlers::orchestras::submit_orchestra_request_handler` src/api_server.rs:4980 | CAP-04 | `fermi/templates/admin.html:1032` |
| `/api/orchestras/:p/requests/:p/approve` | POST `handlers::orchestras::approve_orchestra_request_handler` src/api_server.rs:4985 | CAP-04 | `fermi/templates/admin.html:1143` |
| `/api/orchestras/:p/requests/:p/reject` | POST `handlers::orchestras::reject_orchestra_request_handler` src/api_server.rs:4989 | CAP-04 | `fermi/templates/admin.html:1173` |
| `/api/orchestras/:p/requests/:p/withdraw` | POST `handlers::orchestras::withdraw_orchestra_request_handler` src/api_server.rs:4993 | CAP-04 | `fermi/templates/agent_detail.html:5238` |
| `/api/profile` | GET `handlers::profile::get_profile_handler` src/api_server.rs:4406<br>PUT `handlers::profile::update_profile_handler` src/api_server.rs:4407 | CAP-14 | `fermi/templates/profile.html:1012` |
| `/api/projections/bestiary` | GET `handlers::ontology::get_bestiary_projections` src/api_server.rs:3484 | tenant:fermi-console | `fermi/templates/projector.html:783` |
| `/api/secrets` | POST `handlers::profile::create_secret_handler` src/api_server.rs:4425<br>GET `handlers::profile::list_secrets_handler` src/api_server.rs:4429 | CAP-14 | `fermi/static/js/widgets/agent-fields.js:1367` |
| `/api/secrets/:p` | DELETE `handlers::profile::delete_secret_handler` src/api_server.rs:4434 | CAP-14 | `fermi/templates/profile.html:831` |
| `/api/secrets/audit` | GET `handlers::profile::secret_audit_handler` src/api_server.rs:4430 | CAP-14 | `fermi/templates/profile.html:889` |
| `/api/skills` | GET `handlers::agents::list_skills_handler` src/api_server.rs:3019 | CAP-01 | `fermi/static/js/widgets/agent-fields.js:487` |
| `/api/specimen/:p` | GET `handlers::specimen::specimen_handler` src/api_server.rs:3341 | tenant:rabble | `fermi/templates/specimen.html:1473` |
| `/api/stream` | GET `handlers::specimen::stream_handler` src/api_server.rs:3356 | tenant:rabble | `fermi/templates/stream.html:113` |
| `/api/tags/popular` | GET `handlers::wizard::popular_tags_handler` src/api_server.rs:3796 | crud:misc | `fermi/templates/agent_create.html:1545` |
| `/api/teams` | POST `handlers::teams::create_team_handler` src/api_server.rs:3663<br>GET `handlers::teams::list_teams_handler` src/api_server.rs:3664 | CAP-02 | `fermi/templates/dashboard.html:2562` |
| `/api/teams/:p` | GET `handlers::teams::get_team_handler` src/api_server.rs:3665<br>DELETE `handlers::teams::delete_team_handler` src/api_server.rs:3669 | CAP-02 | `fermi/templates/dashboard.html:1155` |
| `/api/teams/:p/members` | POST `handlers::teams::add_member_handler` src/api_server.rs:3673<br>GET `handlers::teams::list_members_handler` src/api_server.rs:3677 | CAP-02 | `fermi/templates/workspace.html:2377` |
| `/api/teams/:p/members/:p` | DELETE `handlers::teams::remove_member_handler` src/api_server.rs:3681<br>PUT `handlers::teams::update_member_role_handler` src/api_server.rs:3685 | CAP-02 | `fermi/templates/workspace.html:2284` |
| `/api/users/:p` | GET `handlers::users::get_public_profile_handler` src/api_server.rs:3558 | CAP-14 | `fermi/templates/user_profile.html:229` |
| `/api/users/collaborators` | GET `handlers::users::get_collaborators_handler` src/api_server.rs:4389 | CAP-14 | `fermi/templates/dashboard.html:1998` |
| `/api/users/search` | GET `handlers::users::search_users_handler` src/api_server.rs:4385 | CAP-14 | `fermi/templates/dashboard.html:2117` |
| `/api/verification-queue` | GET `handlers::loops::verification_queue_handler` src/api_server.rs:3214 | CAP-06 | `fermi/templates/trace.html:2785` |
| `/api/verification-queue/:p/settle` | POST `handlers::loops::settle_verification_handler` src/api_server.rs:3218 | CAP-06 | `fermi/templates/trace.html:2736` |
| `/api/waitlist` | POST `handlers::misc::waitlist_handler` src/api_server.rs:2946 | CAP-14 | `fermi/templates/landing.html:807` |
| `/api/wallet` | GET `handlers::wallet::get_wallet_handler` src/api_server.rs:4362 | replaced:acn | `fermi/templates/dashboard.html:1015` |
| `/api/wallet/transactions` | GET `handlers::wallet::get_transactions_handler` src/api_server.rs:4363 | replaced:acn | `fermi/templates/admin.html:1197` |
| `/api/workspaces` | GET `handlers::workspace::list_workspaces_handler` src/api_server.rs:4088 | CAP-15 | `fermi/templates/dashboard.html:1200` |
| `/api/workspaces/:p` | GET `handlers::workspace::get_workspace_handler` src/api_server.rs:4092 | CAP-15 | `fermi/static/js/widgets/xaman-ek.js:932` |
| `/api/workspaces/:p/add` | POST `handlers::workspace::add_agent_handler` src/api_server.rs:4298 | CAP-15 | `fermi/templates/dashboard.html:2587` |
| `/api/workspaces/:p/agents` | GET `handlers::workspace::list_workspace_agents_handler` src/api_server.rs:4096<br>POST `handlers::workspace::create_workspace_agent_handler` src/api_server.rs:4100 | CAP-15 | `fermi/templates/workspace.html:3956` |
| `/api/workspaces/:p/agents/:p` | DELETE `handlers::workspace::remove_workspace_agent_handler` src/api_server.rs:4302 | CAP-15 | `fermi/templates/workspace.html:2302` |
| `/api/workspaces/:p/budget` | POST `handlers::workspace::fund_workspace_handler` src/api_server.rs:4104<br>GET `handlers::workspace::workspace_budget_handler` src/api_server.rs:4110 | CAP-15 | `fermi/templates/workspace.html:2986` |
| `/api/workspaces/:p/coherence` | GET `handlers::workspace::get_coherence_handler` src/api_server.rs:4443 | CAP-04 | `fermi/templates/workspace.html:3032` |
| `/api/workspaces/:p/coherence/evaluate` | POST `handlers::workspace::evaluate_coherence_handler` src/api_server.rs:4439 | CAP-04 | `fermi/templates/workspace.html:3381` |
| `/api/workspaces/:p/coherence/history` | GET `handlers::workspace::get_coherence_history_handler` src/api_server.rs:4447 | CAP-04 | `fermi/templates/workspace.html:4173` |
| `/api/workspaces/:p/composition/dream` | POST `handlers::composition::composition_dream_handler` src/api_server.rs:4357 | CAP-06 | `fermi/templates/workspace.html:3241` |
| `/api/workspaces/:p/composition/identity` | POST `handlers::workspace::set_composition_identity_handler` src/api_server.rs:4336 | CAP-02 | `fermi/templates/workspace.html:4131` |
| `/api/workspaces/:p/composition/versions` | GET `handlers::composition::list_composition_versions_handler` src/api_server.rs:4341 | ops | `fermi/templates/workspace.html:3125` |
| `/api/workspaces/:p/composition/versions/:p/accept` | POST `handlers::composition::accept_composition_version_handler` src/api_server.rs:4345 | ops | `fermi/templates/workspace.html:3265` |
| `/api/workspaces/:p/composition/versions/:p/reject` | POST `handlers::composition::reject_composition_version_handler` src/api_server.rs:4349 | ops | `fermi/templates/workspace.html:3282` |
| `/api/workspaces/:p/files` | GET `handlers::workspace::list_workspace_files_handler` src/api_server.rs:4470 | CAP-15 | `fermi/templates/workspace.html:3433` |
| `/api/workspaces/:p/files/:p` | **none: no legacy route** | — | `fermi/templates/workspace.html:3525` |
| `/api/workspaces/:p/flow` | GET `handlers::specimen::workspace_flow_handler` src/api_server.rs:3364 | tenant:rabble | `fermi/templates/flow.html:241` |
| `/api/workspaces/:p/fork-to-app` | POST `handlers::workspace::fork_workspace_to_app_draft_handler` src/api_server.rs:4289 | CAP-15 | `fermi/templates/workspace.html:4251` |
| `/api/workspaces/:p/git/diff` | GET `handlers::workspace::workspace_git_diff_handler` src/api_server.rs:4496 | CAP-15 | `fermi/templates/workspace.html:3644` |
| `/api/workspaces/:p/git/log` | GET `handlers::workspace::workspace_git_log_handler` src/api_server.rs:4492 | CAP-15 | `fermi/templates/workspace.html:3587` |
| `/api/workspaces/:p/hire` | POST `handlers::workspace::hire_agent_handler` src/api_server.rs:4294 | CAP-02 | `fermi/templates/workspace.html:2818` |
| `/api/workspaces/:p/messages` | POST `handlers::workspace::post_workspace_message_handler` src/api_server.rs:4266<br>GET `handlers::workspace::get_workspace_messages_handler` src/api_server.rs:4270 | CAP-15 | `fermi/templates/workspace.html:2057` |
| `/api/workspaces/:p/messages/stream:p` | **none: no legacy route** | — | `fermi/templates/workspace.html:2247` |
| `/api/workspaces/:p/upload` | POST `handlers::workspace::upload_workspace_file_handler` src/api_server.rs:4487 | CAP-15 | `fermi/templates/workspace.html:3732` |
| `/api/workspaces/:p/workflow` | GET `handlers::workspace::get_workspace_workflow_handler` src/api_server.rs:4283 | CAP-15 | `fermi/templates/workspace.html:3819` |
| `/api/xaman/sessions` | GET `handlers::xaman::list_xaman_sessions_handler` src/api_server.rs:4307<br>POST `handlers::xaman::create_xaman_session_handler` src/api_server.rs:4311 | CAP-14 | `fermi/static/js/widgets/xaman-ek.js:160` |
| `/api/xaman/sessions/:p` | GET `handlers::xaman::get_xaman_session_handler` src/api_server.rs:4315<br>DELETE `handlers::xaman::abandon_xaman_session_handler` src/api_server.rs:4331 | CAP-14 | `fermi/static/js/widgets/xaman-ek.js:256` |
| `/api/xaman/sessions/:p/create-app` | POST `handlers::xaman::create_app_from_session_handler` src/api_server.rs:4327 | CAP-14 | `fermi/static/js/widgets/xaman-ek.js:507` |
| `/api/xaman/sessions/:p/message` | POST `handlers::xaman::xaman_session_message_handler` src/api_server.rs:4319 | CAP-14 | `fermi/static/js/widgets/xaman-ek.js:339` |

## wild

| Endpoint | Legacy route(s) | Disposition | First call site |
| --- | --- | --- | --- |
| `/api/workspaces/:p/actions/:p` | ~prefix `2 routes under prefix`  | — | `kask/kask-wild-client.js:329` |

## kask-shared

| Endpoint | Legacy route(s) | Disposition | First call site |
| --- | --- | --- | --- |
| `/api/agents` | GET `handlers::agents::list_agents` src/api_server.rs:2903<br>POST `handlers::agents::create_agent_handler` src/api_server.rs:3838 | CAP-01 | `kask/abw-client.js:305` |
| `/api/agents/:p/execute` | POST `handlers::execution::execute_agent_handler` src/api_server.rs:3908 | CAP-01 | `kask/abw-client.js:389` |
| `/api/agents/:p/execute/stream` | POST `handlers::execution_stream::execute_agent_stream_handler` src/api_server.rs:3913 | CAP-01 | `kask/abw-client.js:1197` |
| `/api/apps` | GET `handlers::apps::list_apps_handler` src/api_server.rs:2940<br>POST `handlers::apps::create_app_handler` src/api_server.rs:3955 | CAP-15 | `kask/abw-client.js:814` |
| `/api/apps/:p` | GET `handlers::apps::get_app_handler` src/api_server.rs:2941<br>PUT `handlers::apps::update_app_handler_full` src/api_server.rs:3956 | CAP-15 | `kask/abw-client.js:819` |
| `/api/apps/:p/archive` | POST `handlers::apps::archive_app_handler` src/api_server.rs:3976 | CAP-15 | `kask/abw-client.js:839` |
| `/api/apps/:p/publish` | POST `handlers::apps::publish_app_handler` src/api_server.rs:3972 | CAP-15 | `kask/abw-client.js:834` |
| `/api/apps/:p/workspaces` | POST `handlers::apps::spawn_workspace_handler` src/api_server.rs:3960<br>GET `handlers::apps::list_app_workspaces_handler` src/api_server.rs:3964 | CAP-15 | `kask/abw-client.js:849` |
| `/api/auth/me` | GET `handlers::auth::auth_me` src/api_server.rs:3655 | CAP-14 | `kask/abw-client.js:233` |
| `/api/feed` | GET `handlers::creatures::feed_handler` src/api_server.rs:5253 | tenant:rabble | `kask/abw-client.js:426` |
| `/api/forecasts` | POST `handlers::forecasts::create_forecast_handler` src/api_server.rs:4535<br>GET `handlers::forecasts::list_forecasts_handler` src/api_server.rs:4539 | tenant:fermi-console | `kask/abw-client.js:1186` |
| `/api/fpl/execute` | POST `handlers::notebooks::fpl_execute_handler` src/api_server.rs:4526 | tenant:fermi-console | `kask/abw-client.js:873` |
| `/api/health` | GET `handlers::misc::health` src/api_server.rs:2820 | ops | `kask/abw-client.js:208` |
| `/api/observe/platforms` | POST/GET `handlers::observations::create_platform_handler, handlers::observations::list_platforms_handler` src/api_server.rs:5606 | CAP-05 | `kask/abw-client.js:712` |
| `/api/observe/sessions` | POST/GET `handlers::observations::create_observation_session_handler, handlers::observations::list_observation_sessions_handler` src/api_server.rs:5611 | CAP-14 | `kask/abw-client.js:723` |
| `/api/observe/sessions/:p/observations` | POST/GET `handlers::observations::ingest_observations_handler, handlers::observations::query_observations_handler` src/api_server.rs:5620 | CAP-14 | `kask/abw-client.js:740` |
| `/api/simops/cascade` | POST `handlers::simops::cascade_handler` src/api_server.rs:3989 | tenant:simops | `kask/abw-client.js:897` |
| `/api/simops/cascade/suggest-bindings` | POST `handlers::workspace::actions::suggest_bindings_handler` src/api_server.rs:3999 | tenant:simops | `kask/abw-client.js:989` |
| `/api/simops/dynamics` | POST `handlers::simops::dynamics_handler` src/api_server.rs:4004 | tenant:simops | `kask/abw-client.js:1053` |
| `/api/simops/dynamics/models` | GET `handlers::simops::dynamics_list_handler` src/api_server.rs:4008 | tenant:simops | `kask/abw-client.js:1073` |
| `/api/simops/project` | POST `handlers::simops::project_handler` src/api_server.rs:3994 | tenant:simops | `kask/abw-client.js:1062` |
| `/api/simops/rheology` | POST `handlers::simops::rheology_handler` src/api_server.rs:4013 | tenant:simops | `kask/abw-client.js:1091` |
| `/api/simops/rheology/models` | GET `handlers::simops::rheology_list_handler` src/api_server.rs:4017 | tenant:simops | `kask/abw-client.js:1101` |
| `/api/swarms` | GET `handlers::creatures::list_swarms_handler` src/api_server.rs:3546 | tenant:rabble | `kask/abw-client.js:441` |
| `/api/teams` | POST `handlers::teams::create_team_handler` src/api_server.rs:3663<br>GET `handlers::teams::list_teams_handler` src/api_server.rs:3664 | CAP-02 | `kask/abw-client.js:463` |
| `/api/teams/:p` | GET `handlers::teams::get_team_handler` src/api_server.rs:3665<br>DELETE `handlers::teams::delete_team_handler` src/api_server.rs:3669 | CAP-02 | `kask/abw-client.js:491` |
| `/api/waitlist` | POST `handlers::misc::waitlist_handler` src/api_server.rs:2946 | CAP-14 | `kask/abw-client.js:409` |
| `/api/workspaces` | GET `handlers::workspace::list_workspaces_handler` src/api_server.rs:4088 | CAP-15 | `kask/abw-client.js:454` |
| `/api/workspaces/:p` | GET `handlers::workspace::get_workspace_handler` src/api_server.rs:4092 | CAP-15 | `kask/abw-client.js:472` |
| `/api/workspaces/:p/actions/:p/accept` | POST `handlers::workspace::actions::accept_action_handler` src/api_server.rs:4216 | CAP-15 | `kask/abw-client.js:658` |
| `/api/workspaces/:p/actions/:p/reject` | POST `handlers::workspace::actions::reject_action_handler` src/api_server.rs:4220 | CAP-15 | `kask/abw-client.js:666` |
| `/api/workspaces/:p/actions/annotate` | POST `handlers::workspace::actions::annotate_handler` src/api_server.rs:4160 | CAP-15 | `kask/abw-client.js:638` |
| `/api/workspaces/:p/actions/annotate_schema` | POST `handlers::workspace::actions::annotate_schema_handler` src/api_server.rs:4156 | CAP-15 | `kask/abw-client.js:630` |
| `/api/workspaces/:p/actions/compare` | POST `handlers::workspace::actions::compare_handler` src/api_server.rs:4148 | CAP-15 | `kask/abw-client.js:614` |
| `/api/workspaces/:p/actions/fork_state` | POST `handlers::workspace::actions::fork_state_handler` src/api_server.rs:4144 | CAP-15 | `kask/abw-client.js:606` |
| `/api/workspaces/:p/actions/invoke_member` | POST `handlers::workspace::actions::invoke_member_handler` src/api_server.rs:4152 | CAP-15 | `kask/abw-client.js:622` |
| `/api/workspaces/:p/actions/mutate_document` | POST `handlers::workspace::actions::mutate_document_handler` src/api_server.rs:4140 | CAP-15 | `kask/abw-client.js:598` |
| `/api/workspaces/:p/actions:p` | **none: no legacy route** | — | `kask/abw-client.js:650` |
| `/api/workspaces/:p/add` | POST `handlers::workspace::add_agent_handler` src/api_server.rs:4298 | CAP-15 | `kask/abw-client.js:1140` |
| `/api/workspaces/:p/agents` | GET `handlers::workspace::list_workspace_agents_handler` src/api_server.rs:4096<br>POST `handlers::workspace::create_workspace_agent_handler` src/api_server.rs:4100 | CAP-15 | `kask/abw-client.js:1112` |
| `/api/workspaces/:p/agents/:p` | DELETE `handlers::workspace::remove_workspace_agent_handler` src/api_server.rs:4302 | CAP-15 | `kask/abw-client.js:1174` |
| `/api/workspaces/:p/annotations` | GET `handlers::workspace::actions::list_annotations_handler` src/api_server.rs:4228 | CAP-15 | `kask/abw-client.js:674` |
| `/api/workspaces/:p/annotations/:p` | DELETE `handlers::workspace::actions::resolve_annotation_handler` src/api_server.rs:4232 | CAP-15 | `kask/abw-client.js:681` |
| `/api/workspaces/:p/cascade` | POST `handlers::simops::workspace_cascade_handler` src/api_server.rs:4115 | tenant:simops | `kask/abw-client.js:942` |
| `/api/workspaces/:p/files` | GET `handlers::workspace::list_workspace_files_handler` src/api_server.rs:4470 | CAP-15 | `kask/abw-client.js:513` |
| `/api/workspaces/:p/files/:p` | **none: no legacy route** | — | `kask/abw-client.js:503` |
| `/api/workspaces/:p/git/diff` | GET `handlers::workspace::workspace_git_diff_handler` src/api_server.rs:4496 | CAP-15 | `kask/abw-client.js:552` |
| `/api/workspaces/:p/git/log` | GET `handlers::workspace::workspace_git_log_handler` src/api_server.rs:4492 | CAP-15 | `kask/abw-client.js:539` |
| `/api/workspaces/:p/hire` | POST `handlers::workspace::hire_agent_handler` src/api_server.rs:4294 | CAP-02 | `kask/abw-client.js:1124` |
| `/api/workspaces/:p/messages` | POST `handlers::workspace::post_workspace_message_handler` src/api_server.rs:4266<br>GET `handlers::workspace::get_workspace_messages_handler` src/api_server.rs:4270 | CAP-15 | `kask/abw-client.js:565` |
| `/api/workspaces/:p/messages/stream` | GET `handlers::workspace::workspace_messages_stream_handler` src/api_server.rs:4279 | CAP-15 | `kask/abw-client.js:782` |
| `/mcp/agents/:p` | GET `handlers::mcp::mcp_agent_manifest` src/api_server.rs:2844<br>POST `handlers::mcp::mcp_agent_rpc` src/api_server.rs:2848 | CAP-14 | `kask/abw-client.js:359` |

## rabble

| Endpoint | Legacy route(s) | Disposition | First call site |
| --- | --- | --- | --- |
| `/api/admin/creatures` | GET `handlers::admin::admin_list_creatures_handler` src/api_server.rs:5099 | ops | `rabble/lib/services/api_client.dart:1141` |
| `/api/admin/creatures/:p/flag` | PUT `handlers::admin::admin_flag_creature_handler` src/api_server.rs:5103 | ops | `rabble/lib/services/api_client.dart:1152` |
| `/api/admin/stats` | GET `handlers::admin::admin_stats_handler` src/api_server.rs:4915 | ops | `rabble/lib/services/api_client.dart:1130` |
| `/api/admin/swarms` | GET `handlers::admin::admin_list_swarms_handler` src/api_server.rs:5107 | ops | `rabble/lib/services/api_client.dart:1160` |
| `/api/admin/swarms/:p/status` | PUT `handlers::admin::admin_update_swarm_status_handler` src/api_server.rs:5111 | ops | `rabble/lib/services/api_client.dart:1169` |
| `/api/admin/users` | GET `handlers::admin::admin_list_users_handler` src/api_server.rs:4919 | ops | `rabble/lib/services/api_client.dart:1180` |
| `/api/admin/users/:p/grant` | POST `handlers::admin::admin_grant_credits_handler` src/api_server.rs:4923 | replaced:acn | `rabble/lib/services/api_client.dart:1191` |
| `/api/auth/me` | GET `handlers::auth::auth_me` src/api_server.rs:3655 | CAP-14 | `rabble/lib/services/api_client.dart:43` |
| `/api/billing/checkout` | POST `handlers::billing::billing_checkout_handler` src/api_server.rs:4376 | replaced:acn | `rabble/lib/services/api_client.dart:1008` |
| `/api/billing/dev-topup` | POST `handlers::billing::billing_dev_topup_handler` src/api_server.rs:4380 | replaced:acn | `rabble/lib/services/api_client.dart:1018` |
| `/api/billing/tiers` | GET `handlers::billing::billing_tiers_handler` src/api_server.rs:4372 | replaced:acn | `rabble/lib/services/api_client.dart:999` |
| `/api/collections` | GET/POST `handlers::creatures::list_collections_handler, handlers::creatures::create_collection_handler` src/api_server.rs:5141 | tenant:rabble | `rabble/lib/services/api_client.dart:842` |
| `/api/contacts` | GET/POST `handlers::social::list_contacts_handler, handlers::social::add_contact_handler` src/api_server.rs:5366 | crud:misc | `rabble/lib/services/api_client.dart:1030` |
| `/api/contacts/:p` | PUT/DELETE `handlers::social::update_contact_handler, handlers::social::remove_contact_handler` src/api_server.rs:5371 | crud:misc | `rabble/lib/services/api_client.dart:1053` |
| `/api/creatures` | GET `handlers::creatures::list_creatures_handler` src/api_server.rs:3510 | tenant:rabble | `rabble/lib/services/api_client.dart:69` |
| `/api/creatures/:p` | GET `handlers::creatures::get_creature_handler` src/api_server.rs:3514 | tenant:rabble | `rabble/lib/services/api_client.dart:80` |
| `/api/creatures/:p/animation/:p` | GET `handlers::creatures::creature_animation_layer_handler` src/api_server.rs:3530 | tenant:rabble | `rabble/lib/models/creature.dart:215` |
| `/api/creatures/:p/devices` | GET/POST `handlers::creatures::list_devices_handler, handlers::creatures::pair_device_handler` src/api_server.rs:5351 | tenant:rabble | `rabble/lib/services/api_client.dart:1081` |
| `/api/creatures/:p/favourite` | POST/DELETE `handlers::creatures::favourite_creature_handler, handlers::creatures::unfavourite_creature_handler` src/api_server.rs:5287 | tenant:rabble | `rabble/lib/services/api_client.dart:677` |
| `/api/creatures/:p/flights` | GET `handlers::creatures::creature_flights_handler` src/api_server.rs:3518 | tenant:rabble | `rabble/lib/services/api_client.dart:88` |
| `/api/creatures/:p/fly` | POST `handlers::creatures::fly_handler` src/api_server.rs:5184 | tenant:rabble | `rabble/lib/services/api_client.dart:294` |
| `/api/creatures/:p/generate-art` | POST `handlers::creatures::generate_art_handler` src/api_server.rs:5293 | tenant:rabble | `rabble/lib/services/api_client.dart:912` |
| `/api/creatures/:p/host` | POST `handlers::creatures::host_rabble_handler` src/api_server.rs:5180 | tenant:rabble | `rabble/lib/services/api_client.dart:275` |
| `/api/creatures/:p/perch` | POST `handlers::creatures::perch_handler` src/api_server.rs:5176 | tenant:rabble | `rabble/lib/services/api_client.dart:242` |
| `/api/creatures/:p/presence` | PUT `handlers::creatures::update_creature_presence_handler` src/api_server.rs:5314 | tenant:rabble | `rabble/lib/services/api_client.dart:1525` |
| `/api/creatures/:p/sosa-opt-in` | PUT `handlers::creatures::sosa_opt_in_handler` src/api_server.rs:5301 | tenant:rabble | `rabble/lib/services/api_client.dart:969` |
| `/api/creatures/:p/status` | PUT `handlers::creatures::update_creature_status_handler` src/api_server.rs:5310 | tenant:rabble | `rabble/lib/services/api_client.dart:124` |
| `/api/creatures/:p/telemetry` | POST `handlers::creatures::push_telemetry_handler` src/api_server.rs:5223 | tenant:rabble | `rabble/lib/services/api_client.dart:338` |
| `/api/creatures/:p/tether` | POST/DELETE `handlers::creatures::tether_handler, handlers::creatures::untether_handler` src/api_server.rs:5189 | tenant:rabble | `rabble/lib/services/api_client.dart:316` |
| `/api/creatures/:p/track` | GET `handlers::creatures::get_track_handler` src/api_server.rs:5227 | tenant:rabble | `rabble/lib/services/api_client.dart:355` |
| `/api/creatures/:p/transfer` | POST `handlers::creatures::transfer_creature_handler` src/api_server.rs:5318 | tenant:rabble | `rabble/lib/services/api_client.dart:1213` |
| `/api/creatures/:p/update` | PUT `handlers::creatures::update_creature_handler` src/api_server.rs:5306 | tenant:rabble | `rabble/lib/services/api_client.dart:115` |
| `/api/creatures/:p/versions` | GET `handlers::creatures::creature_versions_handler` src/api_server.rs:3522 | ops | `rabble/lib/services/api_client.dart:100` |
| `/api/creatures/mint` | POST `handlers::creatures::mint_creature_handler` src/api_server.rs:5232 | tenant:rabble | `rabble/lib/services/api_client.dart:894` |
| `/api/dashboard/creatures` | GET `handlers::dashboard::creatures_handler` src/api_server.rs:5264 | tenant:rabble | `rabble/lib/services/api_client.dart:1679` |
| `/api/dashboard/nearby` | GET `handlers::dashboard::nearby_rabbles_handler` src/api_server.rs:5260 | tenant:rabble | `rabble/lib/services/api_client.dart:1660` |
| `/api/devices/:p` | PUT/DELETE `handlers::creatures::update_device_handler, handlers::creatures::unpair_device_handler` src/api_server.rs:5356 | tenant:rabble | `rabble/lib/services/api_client.dart:1111` |
| `/api/devices/:p/location` | POST `handlers::creatures::report_device_location_handler` src/api_server.rs:5361 | tenant:rabble | `rabble/lib/services/api_client.dart:1119` |
| `/api/feed` | GET `handlers::creatures::feed_handler` src/api_server.rs:5253 | tenant:rabble | `rabble/lib/services/api_client.dart:641` |
| `/api/flights` | POST `handlers::creatures::record_flight_handler` src/api_server.rs:5150 | tenant:rabble | `rabble/lib/services/api_client.dart:157` |
| `/api/flights/:p/end` | PUT `handlers::creatures::end_flight_handler` src/api_server.rs:5154 | tenant:rabble | `rabble/lib/services/api_client.dart:167` |
| `/api/flights/:p/export` | GET `handlers::creatures::export_flight_handler` src/api_server.rs:5162 | tenant:rabble | `rabble/lib/services/api_client.dart:191` |
| `/api/flights/:p/telemetry` | POST `handlers::creatures::append_telemetry_handler` src/api_server.rs:5158 | tenant:rabble | `rabble/lib/services/api_client.dart:180` |
| `/api/flights/import` | POST `handlers::creatures::import_flight_handler` src/api_server.rs:5166 | tenant:rabble | `rabble/lib/services/api_client.dart:206` |
| `/api/geocode` | GET `handlers::misc::geocode_search_handler` src/api_server.rs:2822 | CAP-01 | `rabble/lib/screens/creature/creature_actions.dart:113` |
| `/api/notifications` | GET `handlers::profile::list_notifications_handler` src/api_server.rs:4412 | CAP-14 | `rabble/lib/services/api_client.dart:1227` |
| `/api/notifications/:p/read` | PUT `handlers::profile::mark_notification_read_handler` src/api_server.rs:4416 | CAP-14 | `rabble/lib/services/api_client.dart:1235` |
| `/api/notifications/read-all` | PUT `handlers::profile::mark_all_notifications_read_handler` src/api_server.rs:4420 | CAP-14 | `rabble/lib/services/api_client.dart:1243` |
| `/api/push/vapid-key` | GET `handlers::push::get_vapid_key_handler` src/api_server.rs:5532 | tenant:rabble | `rabble/lib/screens/home_shell.dart:74` |
| `/api/rabble/:p/attraction-leaderboard` | GET `handlers::rabble_workspace::attraction_leaderboard_handler` src/api_server.rs:5557 | tenant:rabble | `rabble/lib/services/api_client.dart:1285` |
| `/api/rabble/:p/end` | POST `handlers::creatures::end_rabble_handler` src/api_server.rs:5490 | tenant:rabble | `rabble/lib/services/api_client.dart:695` |
| `/api/rabble/:p/flock` | POST `handlers::rabble_workspace::flock_tick_handler` src/api_server.rs:5481 | tenant:rabble | `rabble/lib/services/api_client.dart:1275` |
| `/api/rabble/:p/flock-history` | GET `handlers::rabble_workspace::flock_history_handler` src/api_server.rs:5485 | tenant:rabble | `rabble/lib/services/api_client.dart:1265` |
| `/api/rabble/:p/invite` | POST `handlers::rabble_chat::invite_to_rabble` src/api_server.rs:5463 | tenant:rabble | `rabble/lib/services/api_client.dart:812` |
| `/api/rabble/:p/invite/:p` | DELETE `handlers::rabble_chat::revoke_rabble_invite` src/api_server.rs:5467 | tenant:rabble | `rabble/lib/services/api_client.dart:831` |
| `/api/rabble/:p/leave` | POST `handlers::creatures::leave_rabble_handler` src/api_server.rs:5494 | tenant:rabble | `rabble/lib/services/api_client.dart:705` |
| `/api/rabble/:p/members` | GET `handlers::rabble_chat::list_rabble_members` src/api_server.rs:5471 | tenant:rabble | `rabble/lib/services/api_client.dart:822` |
| `/api/rabble/:p/messages` | GET/POST `handlers::rabble_chat::get_rabble_messages, handlers::rabble_chat::post_rabble_message` src/api_server.rs:5453 | tenant:rabble | `rabble/lib/screens/frame_ar_screen.dart:124` |
| `/api/rabble/:p/transfer-anchor` | POST `handlers::rabble_workspace::transfer_anchor_handler` src/api_server.rs:5544 | tenant:rabble | `rabble/lib/services/api_client.dart:718` |
| `/api/rabble/:p/update-anchor-position` | POST `handlers::rabble_workspace::update_anchor_position_handler` src/api_server.rs:5548 | tenant:rabble | `rabble/lib/services/api_client.dart:733` |
| `/api/rabble/join/:p` | GET/POST `handlers::qr_codes::resolve_qr_token_handler, handlers::creatures::join_by_qr_token_handler` src/api_server.rs:5475 | tenant:rabble | `rabble/lib/services/api_client.dart:751` |
| `/api/swarm-algorithms` | GET `handlers::swarm_algorithms::list_algorithms_handler` src/api_server.rs:5589 | tenant:rabble | `rabble/lib/services/api_client.dart:1540` |
| `/api/swarm-algorithms/activate` | POST `handlers::swarm_algorithms::activate_algorithm_handler` src/api_server.rs:5593 | tenant:rabble | `rabble/lib/services/api_client.dart:1554` |
| `/api/swarm-algorithms/activations/:p` | GET `handlers::swarm_algorithms::list_activations_handler` src/api_server.rs:5597 | tenant:rabble | `rabble/lib/services/api_client.dart:1569` |
| `/api/swarms` | GET `handlers::creatures::list_swarms_handler` src/api_server.rs:3546 | tenant:rabble | `rabble/lib/services/api_client.dart:373` |
| `/api/swarms/:p` | GET `handlers::creatures::get_swarm_handler` src/api_server.rs:3547<br>PATCH `handlers::creatures::update_swarm_handler` src/api_server.rs:5248 | tenant:rabble | `rabble/lib/services/api_client.dart:384` |
| `/api/swarms/:p/join` | POST `handlers::creatures::join_swarm_handler` src/api_server.rs:5244 | tenant:rabble | `rabble/lib/services/api_client.dart:403` |
| `/api/swarms/:p/join-batch` | POST `handlers::rabble_workspace::join_batch_handler` src/api_server.rs:5553 | tenant:rabble | `rabble/lib/services/api_client.dart:616` |
| `/api/users/:p` | GET `handlers::users::get_public_profile_handler` src/api_server.rs:3558 | CAP-14 | `rabble/lib/services/api_client.dart:1253` |
| `/api/users/search` | GET `handlers::users::search_users_handler` src/api_server.rs:4385 | CAP-14 | `rabble/lib/services/api_client.dart:1069` |
| `/api/wallet` | GET `handlers::wallet::get_wallet_handler` src/api_server.rs:4362 | replaced:acn | `rabble/lib/services/api_client.dart:981` |
| `/api/wallet/transactions` | GET `handlers::wallet::get_transactions_handler` src/api_server.rs:4363 | replaced:acn | `rabble/lib/services/api_client.dart:989` |
| `/api/wallet/transfer` | POST `handlers::wallet::transfer_credits_handler` src/api_server.rs:4367 | replaced:acn | `rabble/lib/services/api_client.dart:1203` |

## glasses

No core endpoints found in client sources.

## abw-cli

| Endpoint | Legacy route(s) | Disposition | First call site |
| --- | --- | --- | --- |
| `/api/admin/agents/cleanup-test-cruft:p` | **none: no legacy route** | — | `fermi/crates/abw-cli/src/commands/admin.rs:412` |
| `/api/admin/agents/legacy-slugs:p` | **none: no legacy route** | — | `fermi/crates/abw-cli/src/commands/admin.rs:136` |
| `/api/apps` | GET `handlers::apps::list_apps_handler` src/api_server.rs:2940<br>POST `handlers::apps::create_app_handler` src/api_server.rs:3955 | CAP-15 | `fermi/crates/abw-cli/src/commands/deploy.rs:107` |
| `/api/apps/:p` | GET `handlers::apps::get_app_handler` src/api_server.rs:2941<br>PUT `handlers::apps::update_app_handler_full` src/api_server.rs:3956 | CAP-15 | `fermi/crates/abw-cli/src/commands/deploy.rs:105` |
| `/api/apps/:p/publish` | POST `handlers::apps::publish_app_handler` src/api_server.rs:3972 | CAP-15 | `fermi/crates/abw-cli/src/commands/publish.rs:23` |
| `/api/apps/:p/workspaces` | POST `handlers::apps::spawn_workspace_handler` src/api_server.rs:3960<br>GET `handlers::apps::list_app_workspaces_handler` src/api_server.rs:3964 | CAP-15 | `fermi/crates/abw-cli/src/commands/spawn.rs:32` |
| `/api/apps:p` | **none: no legacy route** | — | `fermi/crates/abw-cli/src/commands/list.rs:57` |
| `/api/workspaces/:p/actions` | GET `handlers::workspace::actions::list_actions_handler` src/api_server.rs:4132 | CAP-15 | `fermi/crates/abw-cli/src/commands/workspace.rs:342` |
| `/api/workspaces/:p/actions/:p/accept` | POST `handlers::workspace::actions::accept_action_handler` src/api_server.rs:4216 | CAP-15 | `fermi/crates/abw-cli/src/commands/workspace.rs:522` |
| `/api/workspaces/:p/actions/:p/reject` | POST `handlers::workspace::actions::reject_action_handler` src/api_server.rs:4220 | CAP-15 | `fermi/crates/abw-cli/src/commands/workspace.rs:564` |
| `/api/workspaces/:p/actions/annotate` | POST `handlers::workspace::actions::annotate_handler` src/api_server.rs:4160 | CAP-15 | `fermi/crates/abw-cli/src/commands/workspace.rs:622` |
| `/api/workspaces/:p/actions/fork_state` | POST `handlers::workspace::actions::fork_state_handler` src/api_server.rs:4144 | CAP-15 | `fermi/crates/abw-cli/src/commands/workspace.rs:796` |
| `/api/workspaces/:p/actions/mutate_document` | POST `handlers::workspace::actions::mutate_document_handler` src/api_server.rs:4140 | CAP-15 | `fermi/crates/abw-cli/src/commands/workspace.rs:693` |
| `/api/workspaces/:p/actions/pending` | GET `handlers::workspace::actions::list_pending_handler` src/api_server.rs:4136 | CAP-15 | `fermi/crates/abw-cli/src/commands/workspace.rs:423` |
| `/api/workspaces/:p/files` | GET `handlers::workspace::list_workspace_files_handler` src/api_server.rs:4470 | CAP-15 | `fermi/crates/abw-cli/src/commands/workspace.rs:254` |
| `/api/workspaces/:p/files/:p` | **none: no legacy route** | — | `fermi/crates/abw-cli/src/commands/workspace.rs:148` |
| `/api/workspaces/:p/messages` | POST `handlers::workspace::post_workspace_message_handler` src/api_server.rs:4266<br>GET `handlers::workspace::get_workspace_messages_handler` src/api_server.rs:4270 | CAP-15 | `fermi/crates/abw-cli/src/commands/workspace.rs:88` |

## Cross-tenant: endpoints needed by more than one tenant

| Endpoint | Tenants |
| --- | --- |
| `/api/auth/me` | abw-web-ui, fermi-console, kask-shared, rabble |
| `/api/workspaces/:p/messages` | abw-cli, abw-web-ui, fermi-console, kask-shared |
| `/api/agents` | abw-web-ui, fermi-console, kask-shared |
| `/api/agents/:p/execute` | abw-web-ui, fermi-console, kask-shared |
| `/api/apps` | abw-cli, abw-web-ui, kask-shared |
| `/api/apps/:p` | abw-cli, abw-web-ui, kask-shared |
| `/api/apps/:p/publish` | abw-cli, abw-web-ui, kask-shared |
| `/api/apps/:p/workspaces` | abw-cli, abw-web-ui, kask-shared |
| `/api/teams` | abw-web-ui, fermi-console, kask-shared |
| `/api/teams/:p` | abw-web-ui, fermi-console, kask-shared |
| `/api/wallet` | abw-web-ui, fermi-console, rabble |
| `/api/workspaces/:p/files` | abw-cli, abw-web-ui, kask-shared |
| `/api/workspaces/:p/files/:p` | abw-cli, abw-web-ui, kask-shared |
| `/api/admin/stats` | abw-web-ui, rabble |
| `/api/admin/users` | abw-web-ui, rabble |
| `/api/admin/users/:p/grant` | abw-web-ui, rabble |
| `/api/agents/:p` | abw-web-ui, fermi-console |
| `/api/agents/:p/calibration` | abw-web-ui, fermi-console |
| `/api/agents/:p/execute/stream` | fermi-console, kask-shared |
| `/api/agents/:p/metrics` | abw-web-ui, dpp-adaptogen |
| `/api/apps/:p/archive` | abw-web-ui, kask-shared |
| `/api/billing/checkout` | abw-web-ui, rabble |
| `/api/billing/tiers` | abw-web-ui, rabble |
| `/api/contacts` | abw-web-ui, rabble |
| `/api/contacts/:p` | abw-web-ui, rabble |
| `/api/feed` | kask-shared, rabble |
| `/api/forecasts` | fermi-console, kask-shared |
| `/api/health` | fermi-console, kask-shared |
| `/api/invites/:p/decline` | abw-web-ui, fermi-console |
| `/api/notifications` | abw-web-ui, rabble |
| `/api/notifications/read-all` | abw-web-ui, rabble |
| `/api/simops/cascade` | kask-shared, simops |
| `/api/swarms` | kask-shared, rabble |
| `/api/teams/:p/members` | abw-web-ui, fermi-console |
| `/api/teams/:p/members/:p` | abw-web-ui, fermi-console |
| `/api/users/:p` | abw-web-ui, rabble |
| `/api/users/search` | abw-web-ui, rabble |
| `/api/waitlist` | abw-web-ui, kask-shared |
| `/api/wallet/transactions` | abw-web-ui, rabble |
| `/api/workspaces` | abw-web-ui, kask-shared |
| `/api/workspaces/:p` | abw-web-ui, kask-shared |
| `/api/workspaces/:p/actions/:p` | fermi-console, wild |
| `/api/workspaces/:p/actions/:p/accept` | abw-cli, kask-shared |
| `/api/workspaces/:p/actions/:p/reject` | abw-cli, kask-shared |
| `/api/workspaces/:p/actions/annotate` | abw-cli, kask-shared |
| `/api/workspaces/:p/actions/fork_state` | abw-cli, kask-shared |
| `/api/workspaces/:p/actions/mutate_document` | abw-cli, kask-shared |
| `/api/workspaces/:p/add` | abw-web-ui, kask-shared |
| `/api/workspaces/:p/agents` | abw-web-ui, kask-shared |
| `/api/workspaces/:p/agents/:p` | abw-web-ui, kask-shared |
| `/api/workspaces/:p/git/diff` | abw-web-ui, kask-shared |
| `/api/workspaces/:p/git/log` | abw-web-ui, kask-shared |
| `/api/workspaces/:p/hire` | abw-web-ui, kask-shared |

## Legacy `/api` routes no tenant client calls (73)

These are used only by the server-rendered UI, by agents (tools), by external callers, or not at all. Each needs a disposition in the register, not an automatic drop.

`/api/admin/liveness`, `/api/admin/rbac/heal`, `/api/admin/rbac/orphans`, `/api/admin/rbac/reassign`, `/api/admin/recompose-mutex-groups`, `/api/admin/wipe-fermi-forecasts`, `/api/bayesops/compare_scenarios`, `/api/bayesops/fit_conditional`, `/api/bayesops/fit_marginal`, `/api/bayesops/input_sensitivity`, `/api/bayesops/optimise_for_target`, `/api/bayesops/posteriors`, `/api/bayesops/posteriors/:p`, `/api/bayesops/predict`, `/api/bayesops/prob_exceeds`, `/api/beacons/:p`, `/api/beacons/:p/asset`, `/api/beacons/nearby`, `/api/benchmark/anchor-sweep`, `/api/creature-friendships`, `/api/creature-friendships/:p`, `/api/creature-friendships/:p/accept`, `/api/creature-friendships/:p/decline`, `/api/creature-friendships/pending`, `/api/creature-invites`, `/api/creature-invites/:p/accept`, `/api/creature-invites/:p/decline`, `/api/creature-invites/pending`, `/api/dashboard/boundary-violations`, `/api/dashboard/my-rabbles`, `/api/dashboard/nearby-creatures`, `/api/debug/startup`, `/api/fpl/health`, `/api/grid-maps/:p`, `/api/locations`, `/api/locations/:p`, `/api/marketplace/listings`, `/api/me/impersonation-history`, `/api/me/workspace`, `/api/my/blocks`, `/api/my/following`, `/api/my/rabbles`, `/api/notebooks`, `/api/notebooks/:p`, `/api/notebooks/:p/execute`, `/api/observatory/loops/brier/mechanism`, `/api/polymarket/import`, `/api/polymarket/observations`, `/api/push/proximity`, `/api/push/subscribe`, `/api/rabble/:p/co-presence`, `/api/rabble/:p/eject`, `/api/rabble/:p/eject/:p`, `/api/rabble/:p/qr`, `/api/rabble/:p/recap/:p`, `/api/rabble/:p/stream`, `/api/rabbles/:p/follow`, `/api/reports`, `/api/shares`, `/api/shares/:p`, `/api/shopping/profile`, `/api/shopping/profile/:p/listing`, `/api/simops/workspaces/:p/process-spacetime`, `/api/simops/workspaces/:p/sample-config`, `/api/swarm/sessions`, `/api/swarm/sessions/:p`, `/api/swarm/sessions/:p/end`, `/api/swarm/sessions/:p/experience`, `/api/swarm/sessions/:p/summary`, `/api/swarm/sessions/:p/telemetry`, `/api/users/block`, `/api/users/block/:p`, `/api/users/social-visibility`
