# ADR-0003: Allowance plus ACN replaces wallets, credits and Stripe

- Status: accepted
- Date: 2026-11-10
- Decider: Ivan Labra

## Context

The original intent was an internal credit economy with transfers between workspaces, agents and owners (capabilities.md area 12; target design §9). In legacy that produced the worst audit findings: minting without a counter-leg (G01), charging non-atomically after the work (G04), errors downgraded to zero charges, and a sponsor wallet that one caller could drain. A transfer-capable internal currency has a large surface for leaks, and ACN already exists to move value between parties.

## Decision

No internal currency. Each owner has a non-transferable allowance, funded only by a settled ACN purchase or a capped operator grant, and consumed only by that owner's episodes through hold-before-work and settle-at-close. Value between owners moves only through ACN commitments settled in Cashu. Normative detail is in `docs/design/economy.md`.

## Consequences

- Target design §9 and T9 are replaced; ECO-1..ECO-9 are in the register.
- Spawning an app moves no value; workspaces get spend caps.
- Capability area 12 ("Account for money") stands as a requirement, met through the allowance mechanism.
- Selling or renting agents between owners waits for M4 (ACN).
