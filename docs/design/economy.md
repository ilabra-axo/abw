# ABW Economy: Allowance and ACN

Nov 10, 2026 · @ivan Labra · Status: **normative** (rank 2 in `docs/PLAN.md` §3)

This document replaces §9 ("Money") and invariant T9 of the target system design. Wallets, the issuer wallet, double-entry credit transfers, Stripe, royalties, platform sponsorship, debt and reconciliation are **withdrawn**. The earlier intent of an internal credit economy with transfers between owners is withdrawn with them (ADR-0003).

## 1. The model in one paragraph

ABW holds no internal currency. Each owner has an **allowance**: the prepaid right to consume compute inside their own estate. An allowance is funded only by a settled ACN purchase (or a capped operator grant), consumed only by that owner's own episodes, and can't be transferred. Value between owners (selling, renting or licensing agents, results and services) moves only through ACN commitments settled in Cashu, outside the allowance.

## 2. Nouns

| Noun | Definition |
| --- | --- |
| Owner | The principal whose estate an allowance belongs to: a user or an organisation |
| Allowance | One event stream `allowance:<owner_id>`, with a projection `proj.allowances(owner_id, funded, granted, consumed, held, available)` |
| Unit | Integer **millisatoshi** (msat) of allowance (pending owner decision 4; see §8). No floats anywhere in allowance payloads (target design OD-07) |
| Hold | A reservation against an allowance for one episode, with a hard ceiling |
| Spend cap | A limit on how much one workspace or app may hold or consume from its owner's allowance per period. A cap is policy, never a balance |
| Price policy | A versioned policy document mapping metered usage (model tokens, tool calls, kernel time) to msat. Every consumption event cites its `price_version` |

## 3. Events

All events are on `allowance:<owner_id>` and are appended through `abw-ledger` with exact expected-version CAS. Every event carries `idempotency_key`, `owner_id`, an integer `amount_msat` (≥ 0), and `price_version` where it applies.

| Event | Effect on the projection | Required references | Idempotency key |
| --- | --- | --- | --- |
| `allowance.opened` | creates the row at zero | owner | `open:<owner>` |
| `allowance.funded` | `funded += n` | `acn_settlement { commitment_id, settlement_ref, mint, amount_msat }`, verified by `abw-acn` | `acn:<settlement_ref>` |
| `allowance.granted` | `granted += n` | operator actor; `grant_policy_version`; `reason` | `grant:<reason>:<owner>` |
| `allowance.hold_placed` | `held += h` | `episode_id` owned by `owner_id`; `workspace_id`; cap check result | `episode:<eid>:hold` |
| `allowance.consumed` | `held −= h`, `consumed += c`, with `c ≤ h` | `episode_id`, `hold_id`, usage breakdown, `price_version` | `episode:<eid>:settle` |
| `allowance.hold_released` | `held −= h` (no consumption: failure, abandonment, reaper) | `episode_id`, `hold_id`, `reason_class` | `episode:<eid>:release` |

`available = funded + granted − consumed − held`.

There is **no** event that decreases `funded` or `granted`, moves allowance between streams, or creates allowance without one of the two references above.

## 4. Invariants

Each has a register entry (`docs/requirements/register.toml`) and conformance tests.

- **ECO-1. Allowance changes only through its own events.** `proj.allowances` is written only by the inline projector in the append transaction. The database refuses a projection change in a transaction that appended no justifying event (target design §3.4.2 mechanism).
- **ECO-2. Funding requires a settled ACN commitment.** `allowance.funded` is accepted only from the `acn.settle` command handler, which verifies the Cashu redemption via `abw-acn` before appending. It is idempotent on `settlement_ref`, with a unique index on `(stream_id, idempotency_key)`.
- **ECO-3. Grants are operator-only and capped.** `allowance.granted` requires `actor_kind = operator`, and the total granted per owner stays within the active grant policy.
- **ECO-4. No value moves between owners inside ABW.** No allowance event names a second owner. `hold_placed` and `consumed` must cite an episode whose `owner_id` equals the stream's owner, checked in `decide` and by a database constraint on the projection.
- **ECO-5. Hold before work, settle at close, no debt.** The episode pipeline places the hold in TX-A, before any LLM or tool call. The executor stops when metered usage reaches the hold. TX-C appends `consumed` with `c ≤ h`, in the same transaction as `episode.closed`. Usage above the hold can't happen, so debt doesn't exist.
- **ECO-6. Idempotent allowance events.** A retried command returns the stored result (`command_dedup`). A different command reusing a key fails with `Conflict(idempotent_replay)`.
- **ECO-7. Non-negative available and held.** `CHECK (available >= 0 AND held >= 0 AND consumed >= 0)` on the projection, plus a check in `decide`.
- **ECO-8. Value between owners only through ACN.** Cross-owner sale, rental and licensing are ACN commitments. Only allowlisted event types reach the relay outbox. The requester, not the performer, declares satisfaction. A Lattice verdict vector is delivery evidence, never satisfaction.
- **ECO-9. ACN isolation and key hygiene.** Only `abw-acn` depends on Nostr, Cashu or key-store crates (`cargo deny`). Private keys live in an encrypted key store and never appear in DB columns, logs or error strings.

## 5. Flows

**Buy allowance.** The owner opens an ACN purchase commitment to the tenant operator for N msat. On settlement, the outbox consumer in `abw-acn` verifies the Cashu redemption and dispatches `acn.settle`, which appends `allowance.funded` in one transaction.

**Run an episode** (target design §4.5, money legs only):

1. **TX-A.** `episode.opened` plus `allowance.hold_placed { h }`. `h` is an estimate from the agent version's iteration budget and the price policy, bounded by `available` and the workspace spend cap. Refused with `InsufficientAllowance` or `SpendCapReached` before any work.
2. **During the run.** The executor meters usage against `h` and stops at the ceiling. Stopping is a normal close with `reason_class = budget_exhausted`.
3. **TX-C.** `episode.closed` plus `allowance.consumed { c ≤ h }`, which releases the remainder in the same event.
4. **Crash.** The hold reaper appends `allowance.hold_released` together with `episode.closed { abandoned }`. Usage already spent on a crashed run is a platform loss bounded by one hold, and visible as an abandoned episode.

**Spawn an app workspace.** No allowance moves. The workspace gets a spend cap (from the app template, adjustable by the owner). Spawning is free; running costs the owner's allowance. This removes the G01 class of findings: there is no budget to mint.

**Sell, rent or license to another owner.** This is an ACN commitment with conditions of satisfaction, which are visible to the performer, judge rubric and threshold included. The hosting model is declared per listing:
- **Renter-hosted.** The asset runs in the renter's workspace on the renter's allowance. The ACN price is the owner's licence or outcome fee.
- **Owner-hosted.** The asset runs in the owner's estate on the owner's allowance. The ACN price covers compute plus margin.

**Within one owner's team.** Agents hiring agents inside one estate is not commerce. It consumes the same owner's allowance through ordinary holds and never touches ACN.

## 6. Event mapping with ACN

| ACN step | ABW event | Evidence attached |
| --- | --- | --- |
| Offer / counter-offer with CoS | `commitment.proposed` | CoS template and rubric hash |
| Accept | `commitment.bound` | Both keys' signatures; deadline |
| Deliver | `commitment.delivered` | Artifact hashes; verdict vector hash |
| Declare satisfaction | `commitment.satisfied` or `commitment.disputed` | Requester signature |
| Settle | `commitment.settled` | Cashu redemption reference; for a purchase, the `allowance.funded` event |

ABW events carry ACN event ids; ACN deliveries carry ABW artifact hashes. The ACN implementation is `github.com/ilabra-axo/acn` (`acn-core`, `acn-wallet`, `acn-judge`); `abw-acn` binds to it.

## 7. What this rules out by construction

| Legacy finding class | Why it can't occur |
| --- | --- |
| Minting credits without a counter-leg (G01) | No event creates allowance without a verified settlement or a capped operator grant |
| Non-atomic charge after work (G04) | The hold precedes work; consumption commits with `episode.closed` |
| Errors downgraded to zero charge | A failed TX-C leaves the hold open; the reaper releases it and records the abandonment |
| Transfers between owners leaking value | No transfer event exists |
| Balance written directly | `available` is a projection guarded by the database |
| Debt silently forgiven | Debt can't arise |

## 8. Open decisions

1. **Unit.** msat priced by a versioned price policy (recommended), or abstract compute units with a sats price. Decide before M2.
2. **Grants.** Whether onboarding grants exist, and the cap per owner. Decide before M2.
3. **Spend-cap period.** Per day, per month, or per workspace lifetime.
4. **Regulatory treatment.** Prepaid, non-transferable, non-redeemable allowance; counsel to confirm for the EU and Switzerland before any sale (M4).
5. **Mint and custody.** Which Cashu mint, who operates it, and how the operator key is held (M4).
