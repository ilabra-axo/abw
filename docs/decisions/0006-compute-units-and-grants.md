# ADR-0006: Allowance in compute units, with three kinds of grant

- Status: accepted
- Date: 2026-11-11
- Decider: Ivan Labra
- Amends: ADR-0003 (the unit and the grant rules; everything else stands)

## Context

ADR-0003 replaced wallets with a non-transferable allowance, and its first draft counted the allowance in millisatoshi. Legacy credits were never money: they were system tokens for metering and quotas, not tied to bitcoin. Counting the allowance in msat would make every grant a gift of bitcoin, expose balances to price swings, and blur the line between a quota and stored value.

## Decision

1. **The unit is the compute unit (CU)**, an integer with no cash value. A versioned price policy maps usage (model tokens, tool calls, kernel time) to CU, and sets the sats price of CU for ACN purchases.
2. **Three kinds of grant**, each under a versioned, human-accepted grant policy:
   - **onboarding:** once per owner; never expires;
   - **free tier:** once per calendar month (UTC), topping the free-tier bucket up to the policy cap. Unused free units don't accumulate beyond the cap;
   - **operator:** ad-hoc grants by a human operator, with a reason, within a per-owner limit; never expire.
3. **Purchased CU** come only from a settled ACN purchase and never expire.
4. **Buckets and order.** The allowance keeps `free_tier`, `granted` and `purchased` apart, and consumption draws from them in that order.
5. Unchanged from ADR-0003: no transfer between owners; no redemption; hold before work and settle at close; value between owners moves only as sats through ACN.

Normative detail is in `docs/design/economy.md` §2–§5.

## Consequences

- Register ECO-1, ECO-2, ECO-3 and ECO-7 are restated to match.
- Policy values (onboarding amount, free-tier cap, operator limit, CU prices) are data, set before M2 closes.
- The regulatory question for counsel becomes "non-monetary units bought with sats", which is simpler than prepaid stored value.
