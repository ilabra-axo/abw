# ADR-0007: One coherence agent; the platform delivers member observations

- Status: accepted
- Date: 2026-11-11
- Decider: Ivan Labra

## Context

Legacy split coherence work across three pieces (`docs/harvest/agents.md` §1):

- `cohere_and_coordinate`, the strategist;
- `coherence_evaluator`, a card with no prompt that wraps the Rust TEC engine;
- the "coherence coach" role of writing a coordination observation into each team member's episodic memory.

That last role was left to the strategist's prompt and produced 0 writes in 3,576 episodes, until legacy moved it into platform code (`coordination_note::deliver`, legacy `src/coherence.rs:754-854`). There were also two divergent `coherence_evaluator` cards. `coherence_consultant` and `intention_coordinator` were superseded.

## Decision

1. **`cohere_and_coordinate` is the only coherence agent.** It conducts the session, classifies conflict, steers, and writes the coordination brief.
2. **Coherence scoring (TEC Γ) is a deterministic tool**, not an agent decision. The tool is backed by the certified TEC kernel. The curated `agents/curated/coherence_evaluator` card is canonical and exposes those tools, marked active, with legacy wallet fields removed. The bestiary card's theory section is kept as documentation.
3. **There is no coach agent.** When the strategist records a coordination observation, the platform delivers it to each **current** member's episodic memory in the same transaction, as events. Only the workspace's seated strategist can record one. Delivery can't be skipped because a model forgot it.
4. `coherence_consultant` and `intention_coordinator` aren't ported as cards. The code behind intention conflicts (legacy `src/intentions.rs`, `src/plan_solicitation.rs`) is ported as the strategist's tools.

## Consequences

- Register PROT-01 is restated to match.
- Fewer agents to version, price and verify. The cascade becomes testable as a platform guarantee in M5.
