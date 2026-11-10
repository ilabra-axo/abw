# Architecture decision records

A design change happens only through an ADR here, approved by the owner. Number them sequentially; never edit an accepted ADR's decision. Supersede it with a new one.

| ADR | Decision | Status |
| --- | --- | --- |
| [0001](0001-new-core.md) | Build a new core instead of refactoring in place | accepted |
| [0002](0002-target-design-defaults.md) | Adopt the target design's §16 recommendations as defaults | accepted |
| [0003](0003-allowance-and-acn.md) | Allowance plus ACN replaces wallets, credits and Stripe | accepted |
| [0004](0004-m1-conformance-surface.md) | M1 conformance surface: probe aggregate and the API the tests pin | accepted |
| [0005](0005-m1-implementation-choices.md) | M1 implementation choices; the `!Send` transaction guard is deferred to M2 | accepted |

Template:

```markdown
# ADR-NNNN: Title

- Status: proposed | accepted | superseded by ADR-NNNN
- Date: YYYY-MM-DD
- Decider:

## Context
## Decision
## Consequences
```
