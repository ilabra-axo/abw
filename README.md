# ABW

ABW is a substrate on which teams of agents do work under contracts, coordinate, and improve through feedback loops, and on which every result is checked on evidence before anyone relies on it.

This repository is the new core (ADR-0001). It is being built milestone by milestone; see `docs/PLAN.md`.

| Milestone | Delivers | Status |
| --- | --- | --- |
| M0 | Foundations: register, trace, harvest, CI | in progress |
| M1 | Event store (`abw-ledger`) | |
| M2 | Agents, episodes, allowance | |
| M3 | Verification Lattice and Fermi Console | |
| M4 | ACN | |
| M5 | Coordination, strategists, loops | |
| M6 | Remaining tenants | |

## Layout

```text
docs/PLAN.md            the plan; start here
docs/design/            normative design documents (ranked in PLAN.md §3)
docs/decisions/         architecture decision records
docs/requirements/      the requirement register
docs/harvest/           inventory of the legacy system and its disposition
docs/reference/         legacy documents, for context only
xtask/                  repository harness (`cargo xtask trace`, `pr-scope`)
```

## Working on it

Read `AGENTS.md`. In short: tests first, tagged with requirement IDs; only `abw-ledger` writes; no value moves between owners inside ABW.

```sh
cargo xtask trace
```
