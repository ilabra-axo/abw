# ADR-0009: M2 implementation choices

- Status: accepted
- Date: 2026-11-11
- Decider: Ivan Labra

## Context

The M2 product PR (#9) implemented the ADR-0008 surface: ledger modules policy, allowance, agent, episode, reaper and crud; crates `abw-agent` and `abw-http`; and 98 agent cards. It meets all 19 M2 requirements. The builder resolved 14 points; they are recorded here so later work treats them as decisions. Tags: [V] verified against the design or tests, [I] inferred.

## Decision

**Points an auditor should know about.**

1. **`proj.episodes` has no `(status = 'running') = (closed_pos IS NULL)` CHECK** [V], although target design §4.6 has one. A CHECK fires before the deferred T3 justification trigger, which would mask the T3 error. Episode status consistency rests on the ledger's `decide` plus T3. *Revisit:* add it as a deferred constraint trigger.
2. **`abw_app` may INSERT and UPDATE the allowance projections** [I]. §13.2's "no write on money projections" applied to the lapsed wallet tables. Writes stay guarded by the T3 trigger (a row must cite an event the same transaction appended) and by the non-negative CHECKs. *Revisit:* a SECURITY DEFINER writer, if an audit asks for a second layer.
3. **No foreign key on `proj.allowances.stream_id`** [V], for the same reason as `proj.probes` (ADR-0005 item 4).

**Accepted choices.**

4. **Policy resolution** [I]: a tenant's own versions of a policy kind win over the platform default whenever the tenant has any; otherwise the platform default applies. The most recently accepted version with `active_from <= at` wins. A tenant whose own policies aren't active yet doesn't fall back.
5. **`allowance::Open` and `OperatorGrant` check the cited grant policy against the current time** [I], since those commands carry no timestamp.
6. **Episode command idempotency keys are prefixed with the episode stream** [I].
7. **`policy::Accept` defaults to `Expected::NoStream`**; `PutVersion` without `.expected` uses the head under the lock (§4.2) [I].
8. **`agent.version_created` carries `tools_allowed`**, so TX-A stamps the allowlist without reading the card artifact [I].
9. **`Abandon` accepts only `AbandonedTimeout`, `AbandonedDropped` and `LedgerError`** [I].
10. **Card validation** [I]: `system_prompt` must be a string or null; `capabilities.tools_allowed` must be an array of strings.
11. **Executor** [I]: a call to a tool that is allowlisted but not registered is recorded as allowed, with error class `tool_unavailable`. Refused calls aren't metered. A TX-B or TX-C refusal (for example, the reaper won) surfaces as `RunError::Ledger`.
12. **Phantom-tool stripping (D5)** [I]: `biotech_analyst` keeps three ontology tools that its declared remote MCP server provides. `kask_advisor`'s `mutate_document` and `annotate` are stripped: they are client-side action names, re-added with the workspace actions in M3.
13. **Timestamps are truncated to microseconds**, so rebuilt projections compare equal [I].
14. **Folds replay from the first event**; `ledger.aggregate_snapshots` is unused at M2 sizes [I].

**Dependencies added:** `chrono` (ledger, agent); `reqwest` 0.12 with rustls, only inside `abw-agent::egress`; `axum` 0.8 (http); `tower` as a dev-dependency. Name lookup uses the OS resolver through `tokio::net`, not `hickory-resolver`.

## Consequences

The register's current milestone moves to M2, so CI blocks regressions in all M0–M2 requirements (35). Two M2 exit items from PLAN §7 remain, and neither is a register requirement:
- a real LLM provider, so a real agent runs an episode end to end;
- policy values (onboarding amount, free-tier cap, operator limit, CU prices).
