# Agent Bestiary World: A Logical Architecture for Recursive Self-Improving Multi-Agent Systems

**Authors:** Ivan Labra — axelotl partners  
**Version:** 1.0 — June 2026, corrected November 2026
**Status:** Historical. Working paper; the theory sections stand. For current requirements see `docs/architecture/ABW — Logical Architecture in Capability Terms.md`, and for verified loop state `docs/architecture/FEEDBACK_LOOPS.md`. Corrections since June are marked as dated notes.

---

## Abstract

Agent Bestiary World (ABW) is a multi-agent platform architecture whose distinguishing property is a five-loop recursive self-improvement (RSI) primitive operating at three nested levels: individual agents improve through episodic memory consolidation; compositions improve through coherence evaluation and team restructuring; and the platform's routing layer improves through calibration against resolved ground truth. This paper presents ABW as a *logical* architecture — a description of the conceptual system, its primitives, and their relationships — rather than an implementation specification. We situate ABW within the lineage of complex adaptive systems theory (Holland, 1995), cybernetics (Wiener, 1948; Ashby, 1956; Beer, 1972), and ultra-large-scale systems design (Northrop et al., 2006). We describe five core primitives — the Agent, the Composition, the Episode, the Workspace, and the Feedback Loop — and show how they compose into a system that exhibits the four self-* properties of autonomic computing (IBM, 2006): self-configuring, self-healing, self-optimising, and self-protecting. We identify two classes of evaluation signal — LLM-judged and hard-verified — and show that the architecture's trustworthiness property follows from the requirement that every feedback path be gated by either a human review step or a deterministic coherence check. We close by situating the architecture within the broader programme of domain-constrained mixture-of-experts (MoE) systems, and identify the formal open questions the architecture raises.

**Keywords:** multi-agent systems, recursive self-improvement, complex adaptive systems, mixture of experts, episodic memory, cybernetics, coherence evaluation

---

## 1. Introduction

### 1.1 The coordination problem

The proliferation of capable language-model-based agents has created a coordination problem that is structurally novel. Existing frameworks address the *mechanics* of multi-agent coordination — task decomposition (Talebirad & Nadiri, 2023), role assignment (Li et al., 2023), communication protocols (Google, 2025) — but not its *quality*. A system of agents that routes queries, executes tasks, and returns results may do so correctly on any given invocation while becoming less accurate over time, drifting from its design intent, or failing to improve from accumulated operational experience. The coordination problem, properly stated, includes the problem of *sustained quality under adaptation*.

This problem has a well-developed theoretical lineage. Holland (1995) identifies credit assignment, rule discovery, and implicit parallelism as the structural properties that allow complex adaptive systems (CAS) to improve through experience. Beer (1972) formalises the conditions under which an organisation remains *viable* — capable of maintaining its identity under environmental perturbation — as a recursive hierarchy of five management functions. Ashby (1956) states the condition for effective control as the Law of Requisite Variety: a regulator can only maintain a target state if its variety matches the variety of the perturbations it must absorb. All three frameworks make the same fundamental claim: sustained performance under changing conditions requires a system that modifies itself in response to its own outputs.

ABW is an engineering instantiation of this claim in the domain of multi-agent AI systems.

### 1.2 What this paper is and is not

This paper describes ABW as a *logical* architecture — a description of its conceptual primitives and their relationships, independent of implementation substrate. We do not describe the Rust type system, the PostgreSQL schema, or the API surface. We describe the things the system *is made of* at the level of abstraction appropriate for reasoning about its properties.

A companion paper (Labra, 2026a) describes the allosteric analogy — the structural homology between ABW's RSI primitive and signal transduction dynamics in biochemistry — and situates the architecture within CAS and cybernetic theory. The present paper takes that theoretical grounding as given and focuses on the architecture itself.

Three domain instantiations of ABW are documented separately: SimOps (process optimisation for biomanufacturing), Fermi (probabilistic forecasting as a domain-constrained MoE), and Rabble (distributed creature ecology with spatial intelligence). The present paper describes the substrate that all three instantiate.

### 1.3 Organisation

Section 2 defines the five core primitives. Section 3 describes the RSI primitive as a set of five feedback loops. Section 4 introduces the two signal classes and their gate requirements. Section 5 describes the Mixture-of-Experts routing architecture and its calibration loop. Section 6 situates the architecture within the theoretical lineage. Section 7 states the open formal questions.

---

## 2. The Five Core Primitives

### 2.1 The Agent

An Agent is the atomic unit of computation in ABW. Logically, an Agent is defined by a triple ⟨*identity*, *capabilities*, *memory*⟩:

- **Identity** is a stable declarative specification: a unique identifier, a domain type, a version, a system prompt encoding behavioral policy, and a set of typed input/output contracts (`accepts`, `produces`). Identity is stable in the sense that it changes only through an explicit versioning event, not through normal execution. (Since September 2026 each save snapshots the whole card as an agent version, migrations 240–241, and each episode records the version that produced it; `persona_version` alone does not identify the configuration.)

- **Capabilities** define what the Agent can do: which execution model it uses (LLM, deterministic, or hybrid), which tools it can call, and a *model ladder* — a mapping from cognition tier (free, standard, premium) to specific (provider, model) bindings. The model ladder instantiates Ashby's Law of Requisite Variety at the agent level: the same behavioral specification is served at different compute budgets, with the variety of the agent's response calibrated to the complexity of the task and the resources allocated.

- **Memory** is the accumulation of the Agent's operational history: episodes (raw execution records), semantic rules (distilled propositional knowledge), entities and facts (knowledge graph entries), and an ontology snapshot (the current consolidated state). Memory persists across sessions and is the mechanism by which Agents adapt over time.

An Agent is explicitly *not* a static prompt-engineered configuration. The same agent card that was instantiated at version 1 may reason qualitatively differently at version 3, because the memory layer has been updated through consolidation and human-gated correction. The identity (the card) is stable; the effective behavior is adaptive.

**Formal analogy:** An Agent maps onto a CAS *classifier* (Holland, 1995) with its own tag system, credit accumulation, and rule set. It also maps onto a Beer VSM System 1 (operations) unit — the component that performs actual work — equipped with its own System 4 (intelligence) via the memory and dreaming cycle.

### 2.2 The Episode

An Episode is the record of a single Agent execution. Logically, an Episode is a tuple ⟨*query*, *context*, *response*, *tool_trace*, *provenance*, *authority_weight*⟩:

- **Query** is the input received.
- **Context** is the enriched prompt that the Agent actually processed — the query plus the KG context injected from the Agent's current memory state.
- **Response** is the Agent's output.
- **Tool trace** is the ordered sequence of tool invocations during execution.
- **Provenance** classifies the episode's epistemic authority: `AutoPass` (standard execution), `HumanApproved` (HITL-verified), `HumanCorrected` (HITL-corrected), or `SyntheticCorrection` (injected by the correction system).
- **Authority weight** is a scalar encoding the strength of the provenance signal in subsequent consolidation (human-sourced episodes carry higher weight than auto-generated ones).

Episodes are the raw material of learning. Their accumulation defines the Agent's operational history; their consolidation distills that history into the semantic rules and KG entries that shape future behavior. The order of episodes matters: consolidation is path-dependent, and the episode log must be linearly orderable for RSI signal integrity to be preserved (Labra, 2026b, §10.4.2).

### 2.3 The Composition

A Composition is a *goal-bearing assemblage of Agents*. It is the unit of multi-agent coordination. Logically, a Composition is defined by a quadruple ⟨*members*, *strategist*, *mission*, *improvement_mode*⟩:

- **Members** are the domain-expert Agents that perform the substantive work. Each member declares typed input/output contracts, capability constraints, and a valence (affective signature) that shapes its collaborative behavior.

- **Strategist** is a coordination Agent that embodies a specific strategy for directing member work. The strategist is itself an Agent — it accumulates episodes, undergoes consolidation, and adapts over time. Three canonical strategist types are defined: the *coherence strategist* (maximises discourse coherence through TEC evaluation), the *MoE routing strategist* (routes each query to the most accurate expert), and the *debate strategist* (structures interactions as structured disagreement toward convergence).

- **Mission** is the goal the Composition exists to accomplish. It is a free-text specification that constrains the semantics of the strategist's coordination decisions and the evaluation of member outputs.

- **Improvement mode** specifies how the Composition learns over time. Two modes are defined: `cascade` (the strategist writes its coordination observations into each member's episodic memory, so members learn from the session through their own consolidation) and `tune_team` (the team's composition itself mutates across sessions through owner-accepted proposals).

A Composition is explicitly *not* a static pipeline. A pipeline has a fixed stage graph; a Composition has a coordination strategist that decides dynamically how to direct member activity, and an improvement loop that can change the team's structure over time.

> **Note, November 2026.** A fourth strategist type now exists: the *pipeline strategist*, which runs a composition as ordered stages. It is one strategy a composition can adopt, which is consistent with the claim above: a composition is not a pipeline, but it may seat a strategist that pipelines. Do not confuse it with a *compound agent* such as `social_media_studio`, which hard-wires its own delegation.

**Formal analogy:** A Composition maps onto a Beer VSM System 3 (control) + System 4 (intelligence) structure, with the strategist implementing the System 3 function (operational control and coordination) and the dreaming cycle implementing System 4 (environmental scanning and adaptation). The five feedback loops described in Section 3 together instantiate Beer's recursive viability conditions.

### 2.4 The Workspace

A Workspace is the *operational substrate* of a Composition. It provides the shared state that makes coordination possible: a persistent message thread (the conversation), a git-backed file system (shared artefacts), a credit budget (economic allocation), and an SSE broadcast channel (real-time event stream).

The Workspace is not the Composition. The Composition is the logical entity — the goal, the members, the strategist, the improvement mode. The Workspace is the runtime instance. The distinction matters because it allows the same logical Composition to be reinstantiated in different operational contexts, and because the Workspace's git-backed file system provides an audit trail that is independent of the Composition's memory.

**Key property:** The Workspace is the *assembly condition* for the Composition. Isolated Agents executing independently have no access to each other's state. The Workspace's shared message thread is the medium through which inter-agent communication occurs; the shared file system is the medium through which shared artefacts are produced and consumed. Remove the Workspace and the Composition dissociates into independent Agents — analogous to removing the non-covalent interactions that stabilise a protein oligomer (Labra, 2026a, §4).

### 2.5 The Feedback Loop

A Feedback Loop is a cyclic causal structure in which an Agent or Composition's outputs are used to modify its future behavior. ABW defines five named feedback loops operating at different timescales and system levels. These are described in detail in Section 3.

The formal property that distinguishes ABW's feedback loops from ad-hoc learning is that each loop has a defined *target*, a defined *signal*, a defined *correction path*, and a defined *gate* that prevents arbitrary modification of agent behavior. The gate is the mechanism by which the system remains trustworthy: no loop can produce unbounded adaptation, because every significant behavioral change requires human review, a coherence check, or a deterministic check against ground truth.

> **A gate that is declared is not a gate.** Each of the five was audited against
> its implementation in August 2026. A defined gate turns out to have three
> failure modes that are observationally identical to it working: it can be
> *absent* (declared here, scheduled nowhere), *fatal* (rejecting every input for
> arithmetic reasons — Loop 2's coherence gate did this to 100% of agent-wide
> interventions, which made the two-reviewer consensus path downstream of it
> unreachable), or *proxied* (asserting something cheaper than the property it
> claims). None of the three is visible from this document, and none was visible
> from the source either; all three required counting rows. The claim above
> should be read as a specification of what the gates are *for*, and
> `docs/HANDOFF_loops_and_gates.md` for which of them currently hold.

---

## 3. The RSI Primitive: Five Feedback Loops

The five feedback loops together constitute the RSI primitive. They operate at three nested levels (individual agent, composition, platform routing) and at timescales ranging from minutes to months.

### 3.1 Loop 1 — Individual Agent Learning

**Target:** The agent reasons correctly about its domain using accumulated experience.

**Signal:** Eval dimension scores written per execution. Two signal classes (Section 4):
- *LLM-judged*: relevance, accuracy, persona fidelity, assessed by an LLM evaluator
- *Hard-verified*: Brier score on resolved forecasts; projection accuracy from physical measurement delta

**Correction path:**
```
Execution → Episode → EvaluatorRegistry → eval_signals
  → ConsolidationWorker: DBSCAN cluster → semantic rules → KG mutation
  → KG context injected into next execution
```

**What changes:** The agent's semantic memory — the rules and facts enriching its prompt before each execution. The agent that has consolidated 50 market-analysis episodes reasons differently from an agent with no history — not because its system prompt changed, but because its prompt is augmented with distilled rules from prior experience.

**Timescale:** Hours to days (dreaming cycle cadence). Hard-verified signals may consolidate within the same session as the triggering event.

**Beer VSM mapping:** Loop 1 is System 1 → System 4 feedback. Operations (System 1) produce outputs; intelligence (System 4) extracts patterns from those outputs and updates the operational rules.

### 3.2 Loop 2 — Human-Gated Behavioral Correction

**Target:** Agent behavior aligns with human judgment on high-stakes or anomalous cases.

**Signal:** Anomaly events (drift, conflict, rupture, safety) surfaced to a human reviewer. Reviewer decisions: Approve, Relabel, Intervene.

**Correction path:**
```
Anomaly detected → HITL review queue
  → Reviewer: Intervene
  → CoherenceGate: settle the correction against the agent's world model
       · insufficient world model      ⟹ Undetermined (not a pass)
       · correction rejected by settling ⟹ Blocked
       · Γ(C) < 0.5                    ⟹ Blocked
  → SyntheticCorrection episode (authority_weight = 1.0)
  → Enters Loop 1 at highest authority weight
  → persona_version incremented (new drift baseline)
```

**On the gate condition.** This section previously stated the gate as `Γ(C) ≥
0.5` alone. That is not what discriminates, and the difference was measured
rather than reasoned about: **Γ is identical (0.632) whether the world model
absorbs the correction or rejects it**, because a system that rejects a
contradicting proposition stays perfectly coherent. Γ answers "is the resulting
belief set tidy", and a correction that bounces off leaves a very tidy one.

The discriminator is the correction's *own* post-settling activation: zero nodes
rejected when the model absorbs it, the correction itself rejected with mass
0.89 when it does not. Γ is retained as a secondary floor — it still catches a
correction that is absorbed but wrecks everything around it — but it is not the
primary condition and a gate written from it alone passes corrections the agent
has thrown out.

A third outcome exists and is not a verdict: if the agent has too little history
to settle against, the gate returns `Undetermined`. An unrunnable check must not
read as a pass.

**What changes:** The agent's effective belief system as encoded in its episodic memory. The correction is preserved in an immutable audit trail (`episode_corrections`). Agent-wide corrections require a second independent reviewer.

**Timescale:** Human-initiated. Effect propagates in the next dreaming cycle.

**Critical property:** Loop 2 cannot be bypassed by an agent that learns to produce plausible-sounding outputs. The signal is human judgment, not LLM judgment. The coherence gate prevents corrections that would create incoherence in the agent's world model. This is the mechanism by which the system resists value drift under adversarial or mistaken correction.

### 3.3 Loop 3 — Workspace Coherence

**Target:** The workspace conversation produces coherent, evidence-grounded outputs without suppressing productive disagreement.

**Signal:** Γ(C) — the global coherence score from TEC settling (Thagard, 1989; Thagard & Verbeurgt, 1998) — plus per-principle scores distinguishing productive incoherence (competing hypotheses both grounded in evidence) from destructive incoherence (disconnected utterances with no evidential engagement).

**Correction path (3.A, inner — per session):**
```
Workspace messages accumulate
  → TEC settling engine → Γ(C) + per-principle scores
  → registered coordination strategist (cohere_and_coordinate by default):
      Solicit member plans (Stage 0)
      Assess intention alignment
      Diagnose incoherence type (productive vs destructive)
      Post a coherence update to steer the session
  → record_coordination_observation → episode in each member's memory
  → member's consolidation → semantic rule → KG context next execution
```

> **Correction, August 2026.** Earlier revisions said agents read a coordination brief on their next turn. Nothing reads the brief: workspace auto-injection loads only `context/`, and consolidation reads episodes. The mechanism is a cascade into member memory, which makes Loop 3.A adaptive rather than advisory. The brief remains, for human readers. See `FEEDBACK_LOOPS.md`, Loop 3.

**Correction path (3.B, outer — across sessions):**
```
Session patterns accumulate in strategist memory
  → Composition Dreaming: detect chronic incoherence, valence homophily
  → propose_composition_change: new member roster
  → Owner accept/reject (rejection feeds back into Loop 1 for strategist)
```

**What changes:** Inner loop: the direction of the current conversation, and each member's memory through the cascade. Outer loop: a composition proposal, applied through Loop 4.A.

**Timescale:** Inner: minutes (within-session). Outer: weeks to months (requires sufficient session history).

**Critical design decision:** The taxonomy of incoherence types is essential. Naive coherence optimisation drives compositions toward agreement, suppressing the productive disagreement that improves collective epistemic performance (Page, 2007; Sunstein, 2002). The framework distinguishes four incoherence types by their formal signatures in TEC principle scores, ensuring that structurally productive disagreement is not penalised.

### 3.4 Loop 4 — Team Shape

**Target:** The composition's team structure improves over time to reduce chronic coordination failures (4.A, composition evolution), and each query reaches the member best able to answer it (4.B, routing accuracy — see §5.2).

**Signal:** Two generators feed one mechanism. The quantitative generator is *exact Shapley attribution* over resolved forecasts: each agent's marginal contribution φ, and a pairwise interaction index saying whether each pair is synergistic or redundant. The qualitative generator is Loop 3.B's tension audit over the strategist's memory: chronically weak TEC principles, collapsed valence spread, persistent destructive incoherence.

**Correction path (4.A):**
```
Resolved forecast → exact Shapley decomposition (src/attribution/)
  → forecast_agent_credit (φ) + forecast_agent_interactions
  → composition suggestions (mean credit < 0, at least 5 forecasts)
      or propose_composition_change from the strategist (3.B)
  → composition_versions row (pending)
  → Owner: Accept → workspace membership reconciled
         | Reject → correction episode in strategist memory
```

> **Correction, August 2026.** The June revision gave consolidated-memory patterns as the only signal. That path produced no proposals: `propose_composition_change` had no dispatch arm, and the accept path wrote to columns that do not exist. Both are fixed, and Shapley credit is now the primary signal. Proposals are never applied automatically. A negative φ can mean a weak agent, or a mis-specified driver in the forecast model, and automatic pruning would let a modelling error strip the roster. See `docs/architecture/COMBINATORIAL_CREDIT_ASSIGNMENT.md`.

> **Note.** The valence-homophily threshold (spread < 0.25) exists only as prompt text; no code computes it yet.

**What changes:** Team membership and composition structure.

**Timescale:** Weeks to months. The strategist needs sufficient session history to distinguish persistent patterns from noise.

### 3.5 Loop 5 — Calibration

**Target:** The platform measures how accurate its predictions were against resolved ground truth, so that the layers acting on that measurement — routing (Loop 4.B), model selection, parameter fitting — have something to act on.

**Signal:** 5.A (calibration measurement) has two hard-verified signal paths:
- *Forecast calibration*: Brier score on resolved `fermi_forecasts`
- *Projection accuracy*: SOSA observation delta against prior cascade projections

**Correction path:**
```
5.A (forecast path): forecast resolves → BrierEvaluator → eval_signals → calibration endpoint
  → moe_router_strategist reads calibration via get_agent_calibration tool
  → Loop 4.B: routing decisions annotated with outcome_quality on resolution
  → Loop 1: strategist consolidates routing episodes → routing rules in KG

5.A (projection path): real batch completes → ProjectionScoringEvaluator
  → projection_accuracy EvalSignal → ConsolidationWorker
  → semantic rules ("model X unreliable at condition Y") in dynamics_runner KG
  → model selection influenced on next execution
```

**What changes:** The calibration profiles the routing strategist reads when it selects an agent (Loop 4.B); which dynamics model is used in SimOps projections.

> **Correction, August 2026: a team score is not an agent score.** The forecast path wrote the forecast's Brier once for every agent in `agents_used` (`record_forecast_calibration_signals`). When every member is cited on every forecast, every member gets an identical score at any sample size: the roster matrix has rank 1, so per-agent skill cannot be identified from team outcomes. Per-agent calibration now comes from exact Shapley credit over counterfactual re-runs of the forecast model. It is reported alongside the team Brier, not in place of it, and read only where its two validity gates pass (efficiency residual, reconstruction error). See `docs/architecture/COMBINATORIAL_CREDIT_ASSIGNMENT.md`.

**Timescale:** Forecast path: months (forecast resolution cadence). Projection path: days to weeks (batch cycle cadence).

**Key insight:** Loop 5 closes through Loop 1. The MoE routing strategist is itself an Agent that learns via dreaming. The calibration signal is a new dimension of evidence that its episodic memory can consolidate. No separate routing table or classifier training is needed — the architecture reuses the existing memory and consolidation pipeline.

---

## 4. Signal Classes and Gate Requirements

Every feedback loop consumes eval signals. Two signal classes exist, with different epistemic properties and gate requirements:

### 4.1 LLM-Judged Signals

Produced by evaluators that use a language model to assess output quality: relevance, accuracy, completeness, persona fidelity. These signals are fast, domain-general, and scalable. Their limitation is that they inherit LLM non-determinism — the same output may receive different scores on different invocations — and they are potentially gameable by an agent that learns to produce outputs that score well without achieving the underlying quality target.

*Gate requirement:* LLM-judged signals that trigger anomaly events require human review (Loop 2) before propagating into permanent behavior change. The coherence gate provides an additional structural filter — see §3.2 for what it actually tests, which is whether the agent's world model rejects the correction, not Γ(C) alone.

### 4.2 Hard-Verified Signals

Produced by deterministic comparison against ground truth that resolves independently of the agent's output. The ground truth (a market resolution, a physical batch measurement) does not know what the agent predicted.

Formally, a hard-verified signal is produced by a function *f*(*predicted*, *actual*) → [0,1] where *actual* is observed independently of the agent's execution. Examples:

- **Brier score**: *1 − (p − o)²* where *p* is the predicted probability and *o* ∈ {0,1} is the observed outcome of a resolved forecast question. When *p* is the team's forecast rather than the agent's own, the score belongs to the team (§3.5).
- **Projection accuracy**: *1 − |predicted − actual| / |actual|* where *actual* is a physical measurement (yield in kg, temperature in °C) from a completed cultivation batch.

*Gate requirement:* Hard-verified signals do not require a coherence gate before propagating into Loop 1 consolidation. They are facts about the physical world, not judgments about output quality. They can be gamed only by falsifying the ground truth measurement, which carries real-world cost for the operator.

**Asymmetry:** Hard-verified signals are epistemically stronger but scarcer. They require resolved events (forecasts must resolve; batches must complete). LLM-judged signals are available immediately after every execution. The architecture uses both: LLM-judged signals for fast, broadly applicable feedback; hard-verified signals for slow, domain-specific ground-truth correction.

---

## 5. Domain-Constrained Mixture of Experts

### 5.1 The MoE structure

ABW's routing architecture instantiates a Mixture-of-Experts (MoE) model (Jacobs et al., 1991; Shazeer et al., 2017) at the composition level. A domain-constrained MoE composition consists of:

- A set of *expert agents*, each with declared capability contracts (`accepts`, `produces`, `skills`) and measured calibration profiles
- A *routing strategist* that classifies each incoming query against member capabilities and routes to the most appropriate expert(s)
- An *output contract* that defines the typed schema all member outputs must satisfy, enabling deterministic synthesis
- A *calibration signal* (Brier score or projection accuracy) that scores routing decisions against ground truth, feeding Loop 5.A; per-expert credit within a team forecast comes from Shapley attribution (§3.5)

The domain constraint distinguishes this from general MoE: the output contract pins the output space, making routing decisions scorable against a common ground truth. An unconstrained composition produces whatever format its members produce; a domain-constrained MoE produces outputs that resolve against a common evaluation criterion.

### 5.2 The routing policy

The routing strategist operates in three stages:

**Stage 0 — Classify:** For each incoming query, the strategist reads member capability declarations, queries their calibration profiles (`GET /api/agents/:id/calibration`), and searches its own episodic memory for past routing decisions with similar queries and their annotated outcomes. It ranks candidate members by calibration score, semantic match, and historical routing accuracy.

**Stage 1 — Route:** The strategist delegates to the selected member(s) via tool invocation. For multi-domain queries, it decomposes into sub-queries and routes each to the appropriate expert.

**Stage 2 — Synthesise:** The strategist combines member outputs according to its synthesis protocol, producing an output that satisfies the composition's output contract.

**Stage 3 — Record:** The strategist records the routing decision as an episode tagged `moe_routing_decision`, including the query type, selected member, rationale, and confidence. When the query resolves, the outcome quality is written back to this episode, enabling Loop 4.B consolidation.

> **Note, August 2026.** Routing is now measured outside the strategist's memory as well. Every episode carries route provenance tags (`route:{reason}`, `qsrc:*`, `ibind:*`). Agents declare the domains they serve, which is checked before the hardcoded domain table. SQL views join routing decisions to realised outcomes, with signed per-agent Shapley credit as the headline metric (`migrations/193_route_provenance_outcomes.sql`).

### 5.3 Calibration and cold start

A domain-constrained MoE without calibration data still functions — it routes based on semantic matching against capability declarations. This is the correct degradation: semantic matching is a reasonable prior. As ground truth resolves, routing weights shift toward demonstrated accuracy.

The cold-start progression:
- **Phase 0–2 months:** Semantic matching (deterministic, based on declared capabilities)
- **Phase 2–4 months:** Calibration-weighted routing (forecasts resolving, backtest data seeded)
- **Phase 4+ months:** Calibrated probabilistic routing, composition proposals informed by member accuracy

The architecture degrades gracefully at low data volume and compounds in value as data accumulates.

---

## 6. Theoretical Lineage

### 6.1 Complex Adaptive Systems

Holland (1995) identifies five structural properties of systems that exhibit adaptive behaviour regardless of substrate: (1) tagged classifiers that allow selective interaction; (2) credit assignment mechanisms that reward rules contributing to system goals; (3) rule discovery through recombination; (4) implicit parallelism from population-level diversity; and (5) internal models that support prediction.

ABW instantiates all five. Agent cards are tagged classifiers; the eval signal pipeline implements credit assignment; dreaming-cycle consolidation implements rule discovery; valence diversity across composition members implements population-level diversity; and the Agent's memory layer constitutes its internal model of its domain. The correspondence is not analogical — ABW was designed from CAS foundations (Labra, 2026a, §10).

### 6.2 Cybernetics and Viable Systems

Wiener (1948) establishes feedback as the substrate-independent structure of purposive behaviour. Ashby (1956) states the Law of Requisite Variety: effective regulation requires that the regulator's variety match the variety of disturbances it must absorb. Beer (1972, 1979) operationalises these principles as the Viable System Model (VSM): a recursive five-function architecture whose presence at every level of a system is necessary and sufficient for viability.

The five ABW feedback loops map directly onto Beer's VSM functions:

| VSM Function | ABW Equivalent |
|---|---|
| System 1 — Operations | Individual agent execution |
| System 2 — Coordination | Workspace + coherence evaluator (Loop 3.A) |
| System 3 — Control | eval_signals + anomaly events + HITL (Loops 1, 2) |
| System 4 — Intelligence | ADM dreaming + ontology snapshots + calibration (Loop 5.A) |
| System 5 — Policy | persona_version governance + composition evolution (Loop 4.A) |

The correspondence is structural: ABW was not designed by consulting the VSM, but by solving the same engineering problem Beer addressed — how to build a system that maintains its identity and improves under environmental pressure. The VSM is the theoretical articulation of the conditions that make this possible; ABW is an engineering instantiation of those conditions in software.

Ashby's Law of Requisite Variety appears in ABW as the model ladder: an agent operating at free tier has lower variety than one at premium tier, and the system degrades gracefully rather than catastrophically when variety is insufficient for the task at hand. The capability gate mechanism is a formal realisation of Ashby's insight: rather than attempting a task with insufficient variety, the system declares the task inapplicable at the current tier.

### 6.3 Ultra-Large-Scale Systems

Northrop et al. (2006) describe the architectural properties emergent in software-intensive systems above certain complexity thresholds: decentralised control, normal failures, eroded human/system boundary, and continuous evolution. Their central claim is that such systems require new architectural paradigms, not scaled-up versions of standard approaches.

ABW's distribution topology proposal (Labra, 2026b) traces the evolution from the current centralised substrate (T0) through federated state peers (T4) to a commons end-state (T5). Each stage is independently useful and each preserves the reachability of the next. The topology progression is the long-arc answer to the ULS problem: how does a system that starts as a centralised monolith evolve toward decentralised, resilient, governance-appropriate distribution without requiring a rebuild?

The IBM autonomic computing programme (IBM, 2006) defined the four self-* properties that ULS systems must exhibit. ABW's architecture, as described in this paper, contributes incrementally to all four: self-configuring (agent capability declaration enables automatic routing); self-healing (HITL loop corrects drift without operator intervention); self-optimising (five feedback loops continuously improve accuracy and coherence); self-protecting (coherence gate + HITL gate prevent value drift and adversarial degradation).

### 6.4 Explanatory Coherence

Thagard (1989) proposes a constraint-satisfaction account of belief revision: propositions cohere or incohere according to seven principles (Symmetry, Explanation, Analogy, Data Priority, Contradiction, Competition, Acceptability), and a belief system settles by maximising overall coherence. This theory, and its computational implementation ECHO, provides the formal foundation for Loop 3.

The application of TEC to multi-agent discourse is novel. TEC was developed for scientific theory choice and legal reasoning; its extension to real-time collaborative agent discourse is the contribution of the Coherence Improvement Loop framework (Labra, 2026c), which shows that TEC's constraint-satisfaction network maps naturally onto agent utterances when the seven principles are interpreted as discourse-level coherence criteria. The critical design decision — distinguishing productive incoherence from destructive incoherence by formal signature in TEC principle scores — prevents the system from optimising away the productive disagreement that improves collective epistemic performance (Page, 2007).

---

## 7. Open Formal Questions

### 7.1 Monad structure of the feedback composition

The five feedback loops can be described as a pipeline where the output of each loop becomes an input to downstream loops (Loop 2 feeds Loop 1; Loop 3.B feeds Loop 4.A; Loop 5.A feeds Loop 1). This has the shape of a monadic composition. Whether the composition satisfies the monad laws — left identity, right identity, associativity — is not verified. If it does, the pipeline is safe to refactor; if it does not, the order of composition matters and must be preserved. Verification in a proof assistant (Lean 4, Coq) would strengthen the architecture's formal guarantees.

### 7.2 Convergence of the dreaming cycle

The ConsolidationWorker applies DBSCAN clustering to episode embeddings and extracts semantic rules via LLM. This process is applied iteratively — subsequent dreaming cycles build on the rules produced by prior cycles. The convergence properties of this iteration are not formally characterised. Does the knowledge graph stabilise? Does it oscillate? Does it diverge under certain conditions (e.g., contradiction between high-authority correction episodes)? These are empirical questions that require longitudinal data from production deployments.

### 7.3 Requisite variety at scale

Ashby's Law requires that the regulator's variety match the variety of disturbances. As ABW scales to more domains, more agents, and more composition types, the variety of disturbances the feedback loops must absorb increases. Whether the current five-loop architecture maintains requisite variety at scale, or whether new loop types must be added, is an open question. The distribution topology proposal (Labra, 2026b) addresses the infrastructure scaling problem but not the variety adequacy problem.

### 7.4 Emergence characterisation

The allosteric paper (Labra, 2026a, §8) identifies three classes of behaviour predicted by the framework but not yet observed empirically: endogenous specialisation (agents developing specialised behavioral niches without explicit instruction), emergent cooperativity (compositions outperforming individual agents non-additively), and pathological attractors (dreaming cycles converging on locally optimal but globally suboptimal configurations). These are tractable experiments. The CIL coherence measurement framework and the observability stack provide the measurement instruments. The empirical programme remains to be executed.

### 7.5 Parameter-level learning (BayesOps)

The harness-level loops (Loops 1–4 and 5.A) make harness-level changes: they modify context, configuration, and routing. They do not modify the distribution parameters that govern the probabilistic simulations the system runs. BayesOps (Labra, 2026d) specifies Loop 5.B, shipped through its Phase 3 in June 2026 and fed today by workspace resolutions, which operates at the parameter level — fitting posterior distributions from operational data and updating FPL Driver parameters accordingly. Whether harness-level learning and parameter-level learning are compositionally sound — whether they can operate simultaneously without interfering — is an open question that depends on the relative timescales of the two loops and the degree to which parameter changes affect the signal quality of the harness loops.

---

## 8. Conclusion

ABW presents a coherent logical architecture for recursive self-improving multi-agent systems. Its distinguishing properties are:

1. **Five gated feedback loops** operating at individual, composition, and routing levels, each with a defined target, signal, correction path, and gate
2. **Two signal classes** (LLM-judged and hard-verified) with asymmetric epistemic properties and gate requirements
3. **Domain-constrained MoE** routing that calibrates itself against ground truth through Loop 5.A and Loop 4.B
4. **Grounding in CAS, cybernetic, and ULS theory** — the architecture instantiates Holland's adaptive classifier system, Beer's viable system, and Northrop's ULS architectural targets in a single coherent design

The architecture was designed to be *trustworthy under adaptation*: every path by which an agent's behavior can change permanently is gated by human review, a formal coherence check, or a deterministic check against ground truth. Fast adaptation (Loop 1 dreaming, Loop 3.A inner coordination) operates at timescales where errors are recoverable. Slow adaptation requires human acceptance: Loop 4.A composition changes always, and parameter-level Loop 5.B changes whenever their projected impact exceeds the auto-accept threshold. The architecture's trustworthiness follows structurally from these gate requirements, not from post-hoc safety measures.

The open questions in Section 7 are the research agenda that ABW's operational deployments will generate data to answer. The theoretical framework is in place; the empirical programme begins with deployment.

---

## References

1. Ashby, W. R. (1952). *Design for a Brain*. Chapman & Hall.

2. Ashby, W. R. (1956). *An Introduction to Cybernetics*. Chapman & Hall.

3. Beer, S. (1972). *Brain of the Firm*. Allen Lane / Penguin Press.

4. Beer, S. (1979). *The Heart of Enterprise*. Wiley.

5. Google. (2025). *Agent-to-Agent (A2A) Protocol Specification*. https://google.github.io/a2a

6. Holland, J. H. (1995). *Hidden Order: How Adaptation Builds Complexity*. Addison-Wesley.

7. IBM. (2006). *An Architectural Blueprint for Autonomic Computing* (4th ed.). IBM Corporation.

8. Jacobs, R. A., Jordan, M. I., Nowlan, S. J., & Hinton, G. E. (1991). Adaptive mixtures of local experts. *Neural Computation*, 3(1), 79–87.

9. Labra, I. (2026a). ABW as Allosteric Substrate: Signal Transduction Concepts in a Recursive Agent Architecture. *axelotl partners working paper*. `docs/papers/abw_as_allosteric_substrate.md`

10. Labra, I. (2026b). ABW Distribution Topology — Design Proposal. *Internal architecture document*. `docs/architecture/DISTRIBUTION_TOPOLOGY_PROPOSAL.md`

11. Labra, I. (2026c). Explanatory Coherence Modeling as an Improvement Loop in Agent-to-Agent and Agent-to-Human Collaboration. *axelotl partners working paper*. `docs/papers/coherence_improvement_loop.md`

12. Labra, I. (2026d). BayesOps: Data-Informed Distribution Fitting for Fermi. *Internal specification*. `docs/specs/14_BAYESOPS_SPEC.md`

13. Li, G., et al. (2023). CAMEL: Communicative Agents for "Mind" Exploration of Large Language Model Society. *NeurIPS 2023*.

14. Northrop, L., et al. (2006). *Ultra-Large-Scale Systems: The Software Challenge of the Future*. Carnegie Mellon University / Software Engineering Institute.

15. Page, S. E. (2007). *The Difference: How the Power of Diversity Creates Better Groups, Firms, Schools, and Societies*. Princeton University Press.

16. Shazeer, N., et al. (2017). Outrageously Large Neural Networks: The Sparsely-Gated Mixture-of-Experts Layer. *ICLR 2017*.

17. Sunstein, C. R. (2002). The Law of Group Polarization. *Journal of Political Philosophy*, 10(2), 175–195.

18. Talebirad, Y., & Nadiri, A. (2023). Multi-Agent Collaboration: Harnessing the Power of Intelligent LLM Agents. *arXiv:2306.03314*.

19. Thagard, P. (1989). Explanatory Coherence. *Behavioral and Brain Sciences*, 12(3), 435–467.

20. Thagard, P., & Verbeurgt, K. (1998). Coherence as Constraint Satisfaction. *Cognitive Science*, 22(1), 1–24.

21. Wiener, N. (1948). *Cybernetics: Or Control and Communication in the Animal and the Machine*. MIT Press.

---

*Working paper. Comments welcome. Not for external distribution without author consent.*
