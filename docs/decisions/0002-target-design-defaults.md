# ADR-0002: Adopt the target design's open-decision recommendations as defaults

- Status: accepted
- Date: 2026-11-10
- Decider: Ivan Labra

## Context

Target system design §16 lists 39 open decisions (OD-01..OD-39), each with a recommendation, and the document is written as if every recommendation were accepted.

## Decision

Every OD-nn recommendation is the default in this repository, with these exceptions:

| OD | Disposition |
| --- | --- |
| OD-03 (JCS implementation) | In-tree implementation in `abw-canon`, tested against the RFC 8785 vectors and V1–V9 (§3.6.2) |
| OD-10 (crate split) | The handler crate is `abw-http`, with no `sqlx` dependency; enforced by `cargo deny` |
| OD-22, OD-23 | Not applicable: no legacy backfill; `port_trust` doesn't exist |
| OD-24, OD-25, OD-34, OD-36 | Lapsed with target design §9 (see ADR-0003) |

Changing any default needs a new ADR that names the OD.

## Consequences

Implementers don't re-decide §16 items. Disagreement goes to an ADR, not into code.
