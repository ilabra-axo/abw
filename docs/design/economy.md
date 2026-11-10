# ABW Economy: Allowance and ACN

Nov 10, 2026 · @ivan Labra · Status: **normative** (rank 2 in `docs/PLAN.md` §3) · Amended by ADR-0006 (compute units, grant kinds)

This document replaces §9 ("Money") and invariant T9 of the target system design. Wallets, the issuer wallet, double-entry credit transfers, Stripe, royalties, platform sponsorship, debt and reconciliation are **withdrawn**. The earlier intent of an internal credit economy with transfers between owners is withdrawn with them (ADR-0003).

## 1. The model in one paragraph

ABW holds no internal currency. Each owner has an **allowance** of **compute units (CU)**: a non-monetary right to consume compute inside their own estate. CU come from operator grants (onboarding, a monthly free tier, ad-hoc grants) or from an ACN purchase settled in sats. They are consumed only by that owner's own episodes, and can't be transferred or redeemed. Value between owners (selling, renting or licensing agents, results and services) moves only as sats through ACN commitments, outside the allowance.

## 2. Nouns

| Noun | Definition |
| --- | --- |
| Owner | The principal whose estate an allowance belongs to: a user or an organisation |
| Allowance | One event stream `allowance:<owner_id>`, with a projection `proj.allowances` holding three buckets and their totals (§3) |
| Unit | Integer **compute units** (CU). CU are not money: they have no cash value, can't be transferred and can't be redeemed. No floats anywhere in allowance payloads (target design OD-07) |
| Bucket | Where CU came from, kept apart so free and paid use can be reported separately: `free_tier` (the monthly free tier), `granted` (onboarding and ad-hoc operator grants), `purchased` (ACN purchases) |
| Grant policy | A versioned policy document setting the onboarding grant, the monthly free-tier cap and the ad-hoc grant limits, per tenant |
| Hold | A reservation against an allowance for one episode, with a hard ceiling |
| Spend cap | A limit on how much one workspace or app may hold or consume from its owner's allowance per period. A cap is policy, never a balance |
| Price policy | A versioned policy document mapping metered usage (model tokens, tool calls, kernel time) to CU, and CU to sats for ACN purchases. Every consumption and purchase cites its `price_version` |

## 3. Events

All events are on `allowance:<owner_id>` and are appended through `abw-ledger` with exact expected-version CAS. Every event carries `idempotency_key`, `owner_id`, integer CU amounts (≥ 0), and `price_version` or `grant_policy_version` where it applies.

| Event | Effect on the projection | Required references | Idempotency key |
| --- | --- | --- | --- |
| `allowance.opened` | creates the row at zero | owner | `open:<owner>` |
| `allowance.funded` | `purchased += n` | `acn_settlement { commitment_id, settlement_ref, mint, amount_sat }`, verified by `abw-acn`; `price_version` (sats per CU) | `acn:<settlement_ref>` |
| `allowance.granted` | `free_tier += n`, or `granted += n` | `kind` (`onboarding`, `free_tier`, `operator`); `grant_policy_version`; actor; `reason` for `operator` | `onboarding:<owner>`, `free_tier:<owner>:<period>`, `operator:<grant_id>` |
| `allowance.hold_placed` | `held += h` | `episode_id` owned by `owner_id`; `workspace_id`; cap check result | `episode:<eid>:hold` |
| `allowance.consumed` | `held −= h`; each bucket decreases by its share of `c ≤ h` | `episode_id`, `hold_id`, usage breakdown, per-bucket split, `price_version` | `episode:<eid>:settle` |
| `allowance.hold_released` | `held −= h` (no consumption: failure, abandonment, reaper) | `episode_id`, `hold_id`, `reason_class` | `episode:<eid>:release` |

`available = free_tier + granted + purchased − held`, where each bucket is its remaining balance. The projection also keeps lifetime totals (`total_granted_free_tier`, `total_granted`, `total_purchased`, `total_consumed`) for reporting.

**Grant kinds.**

- **Onboarding:** once per owner, the amount set by the grant policy. Goes into `granted` and never expires.
- **Free tier:** once per period (calendar month, UTC) per owner. It **tops the `free_tier` bucket up to the policy cap**: `n = cap − free_tier remaining`, or 0 if it is already at the cap. Unused free units therefore don't accumulate beyond the cap, with no event that takes units away. It is applied lazily, as the first event of the TX-A that places the owner's first hold in a new period, by the platform actor under the active grant policy.
- **Operator:** an ad-hoc grant by a human operator, for example for a pilot tenant. It needs a reason, goes into `granted`, never expires, and stays within the grant policy's per-owner limit.

**Consumption order.** A settlement draws from `free_tier` first, then `granted`, then `purchased`, so free units are used before paid ones. The `consumed` event records the split.

There is **no** event that removes units from a bucket other than by consumption, moves allowance between streams, or creates allowance without a settled ACN purchase or a grant under a policy version.

## 4. Invariants

Each has a register entry (`docs/requirements/register.toml`) and conformance tests.

- **ECO-1. Allowance changes only through its own events.** `proj.allowances` is written only by the inline projector in the append transaction. The database refuses a projection change in a transaction that appended no justifying event (target design §3.4.2 mechanism).
- **ECO-2. Purchases require a settled ACN commitment.** `allowance.funded` is accepted only from the `acn.settle` command handler, which verifies the Cashu redemption via `abw-acn` before appending, and converts sats to CU at the cited `price_version`. It is idempotent on `settlement_ref`, with a unique index on `(stream_id, idempotency_key)`.
- **ECO-3. Grants follow the grant policy.** Every `allowance.granted` cites an active, human-accepted `grant_policy_version` and stays within it: onboarding once per owner; the free tier once per period and never above the cap; operator grants only by a human operator with a reason, within the per-owner limit. No other actor or kind can grant.
- **ECO-4. No value moves between owners inside ABW.** No allowance event names a second owner. `hold_placed` and `consumed` must cite an episode whose `owner_id` equals the stream's owner, checked in `decide` and by a database constraint on the projection.
- **ECO-5. Hold before work, settle at close, no debt.** The episode pipeline places the hold in TX-A, before any LLM or tool call. The executor stops when metered usage reaches the hold. TX-C appends `consumed` with `c ≤ h`, in the same transaction as `episode.closed`. Usage above the hold can't happen, so debt doesn't exist.
- **ECO-6. Idempotent allowance events.** A retried command returns the stored result (`command_dedup`). A different command reusing a key fails with `Conflict(idempotent_replay)`.
- **ECO-7. Non-negative balances.** `CHECK` constraints on the projection keep every bucket, `held` and `available` at or above zero, plus a check in `decide`.
- **ECO-8. Value between owners only through ACN.** Cross-owner sale, rental and licensing are ACN commitments. Only allowlisted event types reach the relay outbox. The requester, not the performer, declares satisfaction. A Lattice verdict vector is delivery evidence, never satisfaction.
- **ECO-9. ACN isolation and key hygiene.** Only `abw-acn` depends on Nostr, Cashu or key-store crates (`cargo deny`). Private keys live in an encrypted key store and never appear in DB columns, logs or error strings.

## 5. Flows

**Get allowance.** A new owner receives the onboarding grant when their allowance is opened. Each month their first episode tops the free tier up to the cap. To get more, the owner opens an ACN purchase commitment to the tenant operator for N CU at the current price in sats. On settlement, the outbox consumer in `abw-acn` verifies the Cashu redemption and dispatches `acn.settle`, which appends `allowance.funded` in one transaction.

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
| Minting credits without a counter-leg (G01) | No event creates allowance without a verified settlement or a grant under a human-accepted grant policy |
| Non-atomic charge after work (G04) | The hold precedes work; consumption commits with `episode.closed` |
| Errors downgraded to zero charge | A failed TX-C leaves the hold open; the reaper releases it and records the abandonment |
| Transfers between owners leaking value | No transfer event exists |
| Balance written directly | `available` is a projection guarded by the database |
| Debt silently forgiven | Debt can't arise |

## 8. Open decisions

Decided (ADR-0006): the unit is compute units; grants are onboarding, a monthly free tier topped up to a cap, and ad-hoc operator grants; free-tier units don't accumulate beyond the cap, and granted and purchased units don't expire.

Still open:

1. **Policy values.** The onboarding amount, the free-tier cap, the operator per-owner limit, and the CU price of each model. These are policy data, set before M2 closes.
2. **Spend-cap period.** Per day, per month, or per workspace lifetime.
3. **Regulatory treatment.** Non-monetary, non-transferable, non-redeemable units bought with sats; counsel to confirm for the EU and Switzerland before any sale (M4).
4. **Mint and custody.** Which Cashu mint, who operates it, and how the operator key is held (M4).
