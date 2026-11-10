# ABW Verification Lattice — Specification v0.1

Sep 30, 2026 · @ivan Labra

## Purpose and scope

The lattice turns an agent's claim into a five-dimensional evidence record that informs the next action: continue, correct, or ask a human to endorse or sign off. It does not open or shut a gate. Deciding what to do with the evidence belongs to a routing policy, which is separate from the lattice and set per tenant.

In scope: claims agents make about their own work inside ABW, the evidence record, the diagnoses attached to failures, routing, and the events all of this emits. Out of scope: schema validation at the CQRS command boundary, which stays a fail-fast pre-filter ahead of the lattice.

## Principles

1. **Evidence, not verdict.** The lattice outputs a verdict vector with diagnoses. Policy chooses the action; the lattice never does.
2. **Lattice, not ladder.** Dimensions are evaluated independently, with no short-circuit, so one evaluation yields the full diagnosis. Where one dimension depends on another, the dependency is declared, not implied by order.
3. **A failure is a diagnosis.** Every non-pass carries a failure class, the evidence behind it, and a candidate corrective action. No failure is terminal. The worst case is a claim parked with its diagnosis, waiting for its owner.
4. **Humans endorse; they do not re-verify.** A human receives the vector, the diagnoses and the correction history, and supplies the judgment the lattice cannot. Their decision is an event with the same standing as a lattice result.
5. **Residual risk is explicit.** A fully passing vector carries a per-dimension false-pass rate, not a guarantee.
6. **Binding is registration.** It is the point where a claim becomes scoreable. It is checked at outcome time, not at submission.

## The five dimensions

Each dimension answers one question about the claim and rules out one way a claim can look valid while being hollow. Instruments are proposals.

| Dimension | Question | Rules out | Proposed instrument | Result |
| --- | --- | --- | --- | --- |
| Presence | Did the referenced work occur? | Fabricated or hallucinated completions | Event store and artifact hash lookup | Deterministic |
| Liveness | Is the claim still current? | Acting on superseded state | Cited stream version against head; conflicting appends since | Deterministic |
| Truth | Does the content match the artifact's actual state? | Consistent but factually wrong claims | Field-level diff of claim against artifact | Score against threshold θ |
| Grounding | Does it trace to a source outside the agent's say-so? | Self-report posing as evidence | ΞPROV chain walk to an external origin | Score against threshold θ |
| Binding | Does it create an obligation that can be checked later? | Descriptions with no accountability | Deontic evaluator: forecast, outcome condition and deadline registered | Registered now, Brier-scored at outcome |

### Verdict states

- **pass** and **fail**, with a score and θ where the dimension is scored.
- **unsupported**: the dimension passed, but a dimension it depends on failed.
- **not evaluated**: a precondition is missing, so the check could not run.

### Dependencies

- Truth requires Presence. With no artifact there is nothing to diff, so Truth is not evaluated.
- Truth is qualified by Grounding. If Truth passes and Grounding fails, Truth is unsupported: the claim accurately reports something with no external source.
- Liveness and Binding are independent. A claim can be bound while other dimensions fail; routing decides whether that is allowed.

## The judge report

Every claim gets a mandatory LLM-judge report, generated alongside the lattice evaluation. The report is interpretation. It is neither evidence nor a verdict: it contains findings, and routing decides what they mean.

The judge is separate from the lattice because its output is a different kind of thing. The dimensions test the claim against the store, the stream, the artifact, the provenance trail and the evaluator. The judge reads the task, the claim and the artifact, and says what it makes of them.

### Findings

| Kind | Question |
| --- | --- |
| Intent adequacy | Does the claim do what was asked? No dimension covers this. |
| Plausibility | Is the claim coherent, on task, and credible on its face? |
| Truth cross-check | Does the content match the artifact? Only when the judge is given the artifact. |
| Binding observability | Can the outcome condition actually be observed? |
| Draft diagnosis | A plain-language account of what to fix, drawing on the lattice's diagnoses. |

Each finding carries a kind, a severity (info, warn or block), a confidence, and text.

### Who reads it

- **Routing.** A block-severity intent finding prevents Proceed. This is a policy rule, set per tenant and adjustable by the outer loop, not a property of the judge.
- **Humans.** The report is the readable account of the claim in every endorsement and sign-off packet.
- **The proposer.** Corrective feedback uses the draft diagnosis.
- **The middle loop.** Findings are scored against outcomes, so the judge has a measured reliability like any agent.

### Interpretive, never provenance

The report is typed as interpretive. ΞPROV records that it exists and where it came from: model, prompt version, inputs. It can never be a link in a claim's chain of sources, never satisfy Grounding, and never be cited as evidence by another claim. A claim whose only support is a judge report is ungrounded by definition.

### Rules

1. **Independence.** A different model, or at least a different prompt lineage, from the proposer. Claim text is data, never instructions.
2. **Versioned.** Every report records judge model and prompt version, so a change in behaviour traces to a change in the judge.
3. **No certifying authority.** Nothing in the report can raise a dimension to pass or stand in for one.

This is the line between ABW and judge-only designs. There, a model's opinion is the check. Here it is one input among six, with no power to certify, measured against outcomes like everything else.

## Diagnosis and corrective action

Every non-pass produces a diagnosis the proposer can act on. A diagnosis that only says "rejected" is a defect in the lattice, not a result.

| Dimension | Failure class | Diagnosis carries | Corrective action | Acts |
| --- | --- | --- | --- | --- |
| Presence | Artifact missing; reference malformed | The reference that did not resolve | Produce the artifact or fix the reference | Proposing agent |
| Liveness | Superseded | Cited version, head version, events since | Rebase on the current projection and re-derive the claim | Proposing agent |
| Truth | Mismatch | Field-level diff: claimed against observed | Amend the claim, or fix the artifact | Proposing agent |
| Grounding | Ungrounded; chain broken | Where the provenance chain ends | Attach an external source, run the check that produces evidence, or declare the claim an assumption | Agent; human if no source exists |
| Binding | Unbound | Missing forecast, outcome condition or deadline | Add them, or reclassify the claim as a non-binding description | Proposing agent |

Each diagnosis record holds: dimension, failure class, evidence reference, expected, observed, suggested action, and remaining correction budget. When the budget runs out, the claim escalates to a human. It is never silently dropped.

## Routing

A routing policy reads the verdict vector and the claim's stakes, and picks one of five actions. The lattice is identical across tenants; the policy is not. Routing waits for both the lattice evaluation and the judge report. A block-severity finding in the report overrides every row below: the claim goes to correction first.

| Vector | Stakes | Action |
| --- | --- | --- |
| All pass, Binding registered | Routine | **Proceed.** Downstream may rely on the claim at its stated residual risk. |
| All pass | High: irreversible, external money, or tenant-flagged | **Sign-off.** A human authorises the action on a clean vector. |
| Fails on Presence, Liveness, Truth or Binding, budget left | Any | **Correct.** Diagnoses go back to the proposer. |
| Grounding fails, no external source obtainable | Any | **Endorse.** A human attests as the source, recorded as human-grounded. |
| Correction budget exhausted | Any | **Park.** Held with its diagnosis; owner notified. |

### Endorsement and sign-off

These are different acts. **Endorsement** supplies judgment the lattice lacks: a human vouches for an ungrounded claim or accepts a known risk on one dimension. **Sign-off** supplies accountability: a human authorises an action the vector already supports, because the stakes demand a named person.

Both receive the same packet: the claim, the vector with scores and θ, the diagnoses, the correction history, the residual risk, the proposing agent's calibration, and the action requested. A human can endorse or sign, return with a note, or decline with a note. Declining a clean vector is logged as a possible false pass; endorsing a failed one as a possible false fail.

## Event schema

One `lattice.evaluated` event carries the whole vector. A lattice has no meaningful order, so emitting one event per dimension would reintroduce the ladder. Per-dimension spans can be added later for latency tracing, as metadata only.

| Event | Key fields |
| --- | --- |
| `claim.submitted` | cid, agent, attempt, claim, refs (artifacts, cited stream version), forecast p, outcome condition, deadline |
| `lattice.evaluated` | cid, attempt, vector \[dim, state, score, θ, diagnosis\], residual risk |
| `judge.reported` | cid, attempt, findings \[kind, severity, confidence, text\], judge model, prompt version, type: interpretive (never citable as provenance) |
| `route.decided` | cid, action, policy id, reason |
| `correction.requested` | cid, dims, diagnoses, budget left |
| `endorsement.requested` | cid, kind (endorse or sign-off), packet ref, assignee |
| `human.decided` | cid, kind, decision (endorse, sign, return, decline), dims, note, actor |
| `claim.parked` | cid, diagnosis ref, owner |
| `commitment.bound` | cid, p, outcome condition, deadline |
| `outcome.observed` | cid, ok, evidence ref |
| `calibration.updated` | subject (agent, endorser or judge), a, b, brier, mean brier |
| `false_pass.detected` | cid, dim, source (outcome or human) |
| `false_fail.detected` | cid, dim, source (human) |
| `policy.adjusted` | dim or route, from, to, cause |

Example `lattice.evaluated`:

```json
{"type":"lattice.evaluated","cid":"#412","attempt":2,
 "vector":[
  {"dim":"presence","state":"pass"},
  {"dim":"liveness","state":"pass"},
  {"dim":"truth","state":"unsupported","score":0.81,"theta":0.70},
  {"dim":"grounding","state":"fail","score":0.52,"theta":0.65,
   "diagnosis":{"class":"chain_broken","evidence_ref":"ξ:7f3a",
    "expected":"external origin","observed":"agent output at step 2",
    "suggested_action":"attach sensor log for run 22"}},
  {"dim":"binding","state":"pass"}],
 "residual_risk":{"presence":0.002,"liveness":0.004,"truth":0.03,"grounding":0.05}}
```

## Loops and gate metrics

The lattice is only trustworthy if it measures itself. Each loop has an input, a thing it changes, and a metric.

| Loop | Input | Changes | Metric |
| --- | --- | --- | --- |
| Inner, per claim | Diagnoses | The claim, on resubmission | Recovery rate and mean attempts per dimension; which diagnosis classes lead to a fix |
| Middle, per agent and endorser | Observed outcomes | Reliability posterior; forecast bias | Brier score and bias per agent, and per human endorser |
| Outer, per dimension and route | False passes and false fails, from outcomes and human decisions | Thresholds θ and routing policy | False-pass and false-fail rate per dimension; escalation rate; human load; θ history |

The outer loop optimises two costs at once. Tightening θ lowers the false-pass rate but sends more claims to humans. A policy change is accepted only when it improves one without breaking an agreed ceiling on the other.

Human endorsers sit in the middle loop too. An endorsement is a forecast that the claim will hold, and it is scored against outcomes like any other.

## Open questions

- [ ] Should Truth take an unsupported state, or should routing read Truth and Grounding together and leave both raw?
- [ ] May Binding register on a vector with failures, as a conditional commitment?
- [ ] Is the correction budget per claim or per dimension?
- [ ] Which stakes classes trigger sign-off, and who defines them per tenant?
- [ ] Does a human attestation count as a Grounding pass with source human, or as a separate attested state?
- [ ] Is Truth one continuous score or a pass/fail per claimed field?
- [ ] What ceiling on human load does the outer loop respect, and who sets it?

* [ ] Does a blocking intent finding go back to the proposer, or to whoever issued the task, since the task itself may be ambiguous?
* [ ] Which judge model and prompt lineage count as independent of each proposer?
