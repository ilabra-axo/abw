# ADR-0001: Build a new core instead of refactoring in place

- Status: accepted
- Date: 2026-11-10
- Decider: Ivan Labra

## Context

The legacy repository (`Replicant-Partners/fermi`, last commit `a3cea516`) is about 215k lines in its main crate, with 595 routes, 147 tables, 248 migrations (which can't rebuild a database from empty) and 335 direct `UPDATE` sites. Two remediation plans existed: the Coding Agent Directive (Nov 6) and the Refactor-in-Place Plan (Nov 10). Both migrate the code one write site at a time. A year of in-place work showed that new code copies the dominant local pattern, so the architecture drifted while tests passed.

## Decision

Build a new, minimal core in `github.com/ilabra-axo/abw`, governed by `docs/PLAN.md`. Carry over proven parts deliberately: algorithm kernels (with independent oracles), agent cards and prompts (as data), and strategist and loop designs (re-implemented). The legacy repository is frozen except for security fixes and stays the behavioural reference until each tenant has moved.

## Consequences

- The design's invariants hold from the first commit; there is nothing to ratchet.
- Nothing is migrated automatically. Every legacy capability is dispositioned in `docs/harvest/`, and the protected capabilities are register entries (PROT-01..12).
- Tenants must be re-pointed at a new API. The API is designed from the tenants' needs (`docs/harvest/tenants.md`).
- Legacy demo balances, Stripe sessions and wallets aren't carried over.
- The Directive and the Refactor-in-Place Plan are superseded; copies are in `docs/reference/` as history.
