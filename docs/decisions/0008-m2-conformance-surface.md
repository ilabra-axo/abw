# ADR-0008: M2 conformance surface: agents, episodes, allowance, tenancy and egress

- Status: proposed
- Date: 2026-11-11
- Decider: Ivan Labra
- Follows: ADR-0004 (the M1 surface; everything there stands), ADR-0005 item 1 (restored here as SUB-06), ADR-0006 (allowance in compute units)

## Context

M2 builds agent versions, the episode pipeline (TX-A/B/C), the allowance aggregate, tenancy on reads, the outbound guard and the import of legacy cards (`docs/PLAN.md` §7 M2). Its tests are written before the code, so they pin a public surface. This ADR records that surface at the level of detail of ADR-0004, so the Builder implements against the tests without guessing. Sources: target design §4, §8.1, §11.2, §12.5, §13.1, §14.5 and §15.1 (rank 4); `economy.md` (rank 2, replaces design §9 and T9). Where the design says wallet, this ADR says allowance.

Tests for crates that don't exist yet live under the future crate's `tests/` directory (`crates/abw-agent/tests/`, `crates/abw-http/tests/`); cargo ignores a directory with no manifest, so they stay out of the build until the product PR adds the manifest. Tests for new `abw-ledger` modules sit in `crates/abw-ledger/tests/` behind `#![cfg(any())]`, which the product PR deletes (`cargo xtask pr-scope` accepts that deletion, see item 12).

## Decision

### 1. Scope decisions

1. **CAP-01.04 (Remember) moves to M5**, with PROT-07: memory is ported with consolidation.
2. **CAP-12.04 (Fund a spawn) moves to M3.** Workspaces arrive in M3, and under `economy.md` §5 spawning moves no value; it is met by workspace spend caps. In M2 `allowance.hold_placed.workspace_id` is always `null` and no spend-cap check exists.
3. **CAP-14.06 (Keep secrets) moves to M4**, with the ACN key store. M2 stores no tenant secrets; provider keys come from the environment. API keys (item 10) are stored only as hashes.
4. **CAP-12.03 is met by the ECO-4 and ECO-6 tests**: no transfer exists, and retries never charge twice. Those tests carry `// @req CAP-12.03` too.
5. **SUB-06 (new, M2):** no ledger transaction spans a non-ledger await. `Command` decisions are synchronous and pure, and no public API hands a caller a transaction, connection or pool, so an LLM, tool or HTTP call can't run inside a ledger transaction. This restores ADR-0005 item 1 without a `!Send` type: `dispatch` stays `Send` (the M1 `tokio::spawn` test stands), and the guarantee is that nothing reachable from outside `abw-ledger` holds a transaction.
   - `abw_ledger::Command` **stays sealed** in M2. Every M2 aggregate (allowance, agent, episode, policy) lives in `abw-ledger`, so nothing outside the crate implements it.
   - `decide` is a plain `fn` (never `async fn`, no `.await` in its body, no `Future` in its signature). `cargo xtask check` enforces this over `crates/*/src` (check `sync-decide`).
   - `Ledger`, `ReadDb` and `Crud` have no method named `begin`, `acquire`, `transaction`, `tx`, `pool` or `connection`, and no method returning a sqlx transaction, connection or pool. `abw-ledger/tests/compile_fail_sub06.rs` proves each with twins.
   - This supersedes `Crud::tx(&self) -> CrudTx` of design §14.5: CRUD writes are typed methods on `Crud` (item 10).
6. **Allowance replaces wallets** (`economy.md` §3). `allowance.funded` (ECO-2) arrives with M4: in M2 it is **not registered** in `ledger.event_types`, so no M2 command can append it, and the `purchased` bucket stays 0. M4 replaces that assertion with the ECO-2 tests.
7. **Grant and price policies are policy versions** (`ledger.policy_versions`, stream kind `policy`), accepted only by a human actor. Tests seed placeholder values; real values are policy data (`economy.md` §8 item 1).
8. **LLM calls are never real in tests.** `abw-agent` exposes the `LlmProvider` trait and a deterministic `ScriptedProvider` (item 8).
9. **Background reapers are callable functions** (`abw_ledger::reaper::run_once`). M2 needs no product `tokio::spawn`.
10. **Legacy cards are data** under `agents/<slug>/agent_card.json` at the repository root (copied by the Builder from legacy `agents/curated/` and `~/kask/agents/kask_advisor/`, phantom tools stripped per DECISIONS.md D5). The expected set is the `port` and `port:tenant-*` rows of `docs/harvest/agents.md`, plus `kask_advisor`, minus DECISIONS.md D1.

### 2. Stream kinds and event types

All new kinds have `exact` concurrency. All event types are `schema_version` 1.

| Stream kind | Stream id | Holds |
| --- | --- | --- |
| `policy` | `policy:<policy_id>` (uuid, hyphenated) | One policy's versions (design §3.1.7, §11.2) |
| `agent` | `agent:<agent_id>` | One agent's versions (design §4.2) |
| `episode` | `episode:<episode_id>` (uuid, hyphenated) | One run (design §4.6) |
| `allowance` | `allowance:<owner_id>` | One owner's allowance (`economy.md` §3) |

`agent_id` and `owner_id` follow the stream-id grammar `[A-Za-z0-9._@-]{1,128}`. `owner_id` is globally unique across tenants [I].

| Event type | Stream kind | Epistemic | Payload fields the tests read |
| --- | --- | --- | --- |
| `policy.adjusted` | `policy` | `decision` | `policy_id`, `version`, `policy_kind`, `doc_sha`, `accepted_by`, `rationale`, `active_from` |
| `agent.version_created` | `agent` | `structural` | `agent_id`, `version_number`, `card_sha`, `prompt_sha`, `reason` |
| `agent.retired` | `agent` | `structural` | `agent_id`, `reason` |
| `episode.opened` | `episode` | `evidentiary` | `episode_id`, `tenant_id`, `owner_id`, `agent_id`, `version_number`, `card_sha`, `input_sha`, `hold_id`, `hold_cu`, `price_version`, `tools_allowed`, `opened_at` |
| `tool.invoked` | `episode` | `evidentiary` | `iteration`, `call_index`, `tool_name`, `tool_version`, `allowed`, `input_sha`, `output_sha`, `duration_ms`, `error_class` (null unless the output is an error) |
| `gate.decided` | `episode` | `decision` | `gate` (`tool_allowlist` in M2), `decision` (`refused`), `reason_class` (`policy`), `subject` (`tool:<name>`), `episode_id` |
| `episode.closed` | `episode` | `evidentiary` | `status`, `reason_class`, `raw_output_sha`, `tool_call_shas`, `tokens_in`, `tokens_out`, `tool_calls`, `consumed_cu` |
| `allowance.opened` | `allowance` | `evidentiary` | `owner_id`, `idempotency_key` = `open:<owner>` |
| `allowance.granted` | `allowance` | `evidentiary` | `owner_id`, `kind` (`onboarding`, `free_tier`, `operator`), `amount_cu`, `grant_policy_version`, `reason` (operator), `grant_id` (operator), `period` (free tier, `YYYY-MM`), `idempotency_key` = `onboarding:<owner>`, `free_tier:<owner>:<YYYY-MM>` or `operator:<grant_id>` |
| `allowance.hold_placed` | `allowance` | `evidentiary` | `owner_id`, `episode_id`, `hold_id`, `amount_cu`, `workspace_id` (null), `idempotency_key` = `episode:<eid>:hold` |
| `allowance.consumed` | `allowance` | `evidentiary` | `owner_id`, `episode_id`, `hold_id`, `held_cu`, `priced_cu`, `consumed_cu`, `released_cu`, `split {free_tier, granted, purchased}`, `usage {input_tokens, output_tokens, tool_calls}`, `price_version`, `idempotency_key` = `episode:<eid>:settle` |
| `allowance.hold_released` | `allowance` | `evidentiary` | `owner_id`, `episode_id`, `hold_id`, `amount_cu`, `reason_class`, `idempotency_key` = `episode:<eid>:release` |

`grant_policy_version` and `price_version` are `"<policy_id>@<version>"`. All CU amounts are JSON integers ≥ 0. Every allowance payload names exactly one owner, its stream's. The set of registered `allowance.*` types is exactly the five above. `gate.decided` for the credit gate (design §8.1, stream `gate:credit:<tenant>:<day>`) arrives with §8 in M3; in M2 an over-budget open is a `Rejection` with nothing written.

### 3. Projections and DDL the tests read

Each table below has the §3.4.2 justification trigger (`ABW-T3`) on its position column. Column names are pinned; extra columns are allowed.

- `proj.allowances (owner_id text PRIMARY KEY, stream_id text UNIQUE, tenant_id text, free_tier, granted, purchased, held, available, total_granted_free_tier, total_granted, total_purchased, total_consumed bigint NOT NULL, applied_pos bigint)`, with `CHECK (x >= 0)` on every amount and `CHECK (available = free_tier + granted + purchased - held)` (ECO-7).
- `proj.allowance_holds (hold_id uuid PRIMARY KEY, owner_id text, episode_id uuid UNIQUE, tenant_id text, amount_cu bigint CHECK (amount_cu > 0), status text CHECK (status IN ('open','consumed','released')), applied_pos bigint)`, with `FOREIGN KEY (episode_id, owner_id) REFERENCES proj.episodes (episode_id, owner_id)` (deferrable is allowed) (ECO-4).
- `proj.episodes`: design §4.6 columns, `agent_id` as text, plus `owner_id text NOT NULL`, `UNIQUE (episode_id, owner_id)`, `hold_cu bigint`, `consumed_cu bigint`, `reason_class text`, `opened_at timestamptz`; `status IN ('running','success','partial','failure')`.
- `proj.agent_versions (agent_id text, tenant_id text, version_number int, card_sha text, prompt_sha text, reason text, actor text, event_pos bigint)`, immutable by `ledger.forbid_mutation()` (`ABW-T6`).
- `proj.agent_heads (agent_id text PRIMARY KEY, tenant_id text, version_number int, card_sha text, retired boolean NOT NULL, applied_pos bigint)`.
- `ledger.policy_versions.policy_kind` accepts `grant` and `price` as well as the §3.1.7 kinds.
- `crud.api_keys (key_id uuid PRIMARY KEY, tenant_id text, owner_id text, actor_id text, key_sha text UNIQUE CHECK (key_sha ~ '^sha256:[0-9a-f]{64}$'), label text, created_at timestamptz, revoked_at timestamptz)`. The plaintext key is never stored.
- `abw_ledger::admin::rebuild_diff` accepts `proj.allowances`, `proj.allowance_holds`, `proj.episodes`, `proj.agent_versions` and `proj.agent_heads`.

### 4. `abw-ledger` surface added in M2

Dependencies: `chrono` (with `serde`) for `DateTime<Utc>`, the type design §3.8 names. Everything from ADR-0004 stands. Additions to `Rejection` (all `Debug + Clone + PartialEq`; tests match with `{ .. }`):

```rust
NotHuman { actor_kind: ActorKind }, ReasonRequired,
PolicyNotFound { policy: String }, PolicyKindMismatch { expected: PolicyKind, found: PolicyKind },
PolicyNotActive { policy: String }, PolicyInvalid(String),
AllowanceNotOpen { owner_id: String }, AllowanceAlreadyOpen { owner_id: String },
InsufficientAllowance { available: u64, requested: u64 }, HoldAboveCap { cap: u64, requested: u64 },
GrantLimitExceeded { limit: u64, requested_total: u64 },
AgentNotFound { agent_id: String }, AgentRetired { agent_id: String }, InvalidCard(String),
EpisodeNotFound { episode_id: Uuid }, EpisodeClosed { episode_id: Uuid },
ToolNotAllowed { tool_name: String }, InvalidToolRecord(String), InvalidOutcome(String),
```

`ActorKind` gains `Hash`; `PolicyKind` is `Debug + Clone + Copy + PartialEq + Eq`.

**`abw_ledger::PLATFORM_TENANT: &str = "platform"`.** Policies accepted under this tenant are platform defaults (`policy_versions.tenant_id IS NULL`); platform policy streams are readable by every tenant's commands.

**`abw_ledger::policy`**

```rust
pub enum PolicyKind { Grant, Price }                 // as_str: "grant", "price"
pub struct PolicyRef { pub policy_id: Uuid, pub version: u32, pub doc_sha: Sha256Ref } // Debug, Clone, PartialEq, Eq; Display "<id>@<version>"
pub struct Accept;   // Accept::new(policy_id: Uuid, kind: PolicyKind, doc: Value, rationale: impl Into<String>, active_from: DateTime<Utc>) -> Accept
                     // .expected(Expected) -> Accept; Output = PolicyRef
pub struct GrantPolicy { pub onboarding_cu: u64, pub free_tier_cap_cu: u64, pub operator_limit_cu: u64 }
pub struct PricePolicy { pub cu_per_input_token: u64, pub cu_per_output_token: u64, pub cu_per_tool_call: u64,
                         pub hold_gas_cu: u64, pub hold_cap_cu: u64 }
impl PricePolicy { pub fn price(&self, usage: &episode::Usage) -> u64 }  // in*cu_in + out*cu_out + calls*cu_call (saturating)
pub async fn active(read: &ReadDb, kind: PolicyKind, tenant: &str, at: DateTime<Utc>) -> Result<Option<PolicyRef>, ReadError>;
pub async fn load_price(read: &ReadDb, r: &PolicyRef) -> Result<Option<PricePolicy>, ReadError>;
```

- Documents: grant `{"policy_kind":"grant","onboarding_cu":n,"free_tier_cap_cu":n,"operator_limit_cu":n}`; price `{"policy_kind":"price","cu_per_input_token":n,"cu_per_output_token":n,"cu_per_tool_call":n,"hold_gas_cu":n,"hold_cap_cu":n}`. Exactly these keys, non-negative integers, `policy_kind` equal to the command's kind; anything else is `PolicyInvalid`.
- `Accept` is human-only (`NotHuman`), needs a non-empty rationale (`ReasonRequired`), appends `policy.adjusted` and writes the `policy_versions` row in one transaction (design §11.4). `version` is the stream seq.
- **Resolution** (design §11.2): the tenant's policy of that kind if one exists, else the platform default; within it, the highest version with `active_from ≤ at`. A command citing a `PolicyRef` that doesn't exist, whose `doc_sha` differs, or that belongs to another (non-platform) tenant is refused with `PolicyNotFound` (tests also accept `TenantMismatch` for the cross-tenant case); a wrong kind is `PolicyKindMismatch`; an existing ref that isn't the active one at `at` is `PolicyNotActive`.

**`abw_ledger::allowance`**

```rust
pub struct Open;          // Open::new(owner_id: impl Into<String>, grant_policy: PolicyRef); Output = Opened
pub struct Opened { pub owner_id: String, pub onboarding_cu: u64 }
pub struct OperatorGrant; // OperatorGrant::new(owner_id, grant_id: Uuid, amount_cu: u64, reason: impl Into<String>, grant_policy: PolicyRef); Output = Granted
pub struct Granted { pub owner_id: String, pub amount_cu: u64, pub seq: u64 }
pub fn period(at: DateTime<Utc>) -> String;   // "YYYY-MM", UTC
```

- `Open` appends `allowance.opened` and, if the policy's `onboarding_cu > 0`, `allowance.granted { kind: onboarding }` into `granted`, in one transaction. It is the only onboarding path, so onboarding happens once per owner. Its idempotency is automatic: an identical retry returns `replayed: true`; any other `Open` for the same owner returns `DispatchError::IdempotencyMismatch`.
- `OperatorGrant`: human actor only (`NotHuman`), non-empty reason (`ReasonRequired`), allowance must exist (`AllowanceNotOpen`), lifetime operator grants for the owner plus this one ≤ `operator_limit_cu` (`GrantLimitExceeded`). Idempotent on `operator:<grant_id>`: an identical retry replays; a different request with the same `grant_id` is `IdempotencyMismatch`. There is no public way to request an `onboarding` or `free_tier` grant.
- **Free tier** (`economy.md` §3): appended lazily as the first event of the TX-A (`episode::Open`) that places the owner's first hold in a calendar month, under the cited grant policy: `amount_cu = free_tier_cap_cu − free_tier` (0 if at or above the cap; the event is appended even when 0, so the month is marked) [I].
- Command idempotency keys are the payload keys of §2 prefixed with the stream id, e.g. `allowance:<owner>/operator:<grant_id>`, so `command_dedup`'s global key can't collide across streams [I].

**`abw_ledger::agent`**

```rust
pub const CARD_MEDIA_TYPE: &str = "application/vnd.abw.agent-card+json";
pub enum VersionReason { Seed, OwnerEdit, AdoptFile, Revert }   // "seed", "owner_edit", "adopt_file", "revert"
pub struct PutVersion;   // PutVersion::new(agent_id, card: Value, reason: VersionReason); .expected(Expected); Output = Versioned
pub struct Versioned { pub agent_id: String, pub version_number: u64, pub card_sha: Sha256Ref, pub prompt_sha: Sha256Ref, pub created: bool }
pub struct Retire;       // Retire::new(agent_id, reason: impl Into<String>); Output = Retired
pub struct Retired { pub agent_id: String, pub seq: u64 }
pub struct AgentHead { pub agent_id: String, pub version_number: u64, pub card_sha: Sha256Ref, pub retired: bool }
pub async fn head(read: &ReadDb, tenant: &str, agent_id: &str) -> Result<Option<AgentHead>, ReadError>;
```

- `card` is an already resolved card (item 7 resolves legacy cards): a JSON object whose `system_prompt` is a string, else `InvalidCard`. `card_sha = sha256_jcs(card)`; the JCS bytes are stored as an artifact with `CARD_MEDIA_TYPE` in the same transaction; `prompt_sha = sha256_raw(system_prompt)`.
- `version_number` is the stream seq of the `agent.version_created` event. If `card_sha` equals the head's, nothing is appended and the output is the head with `created: false` (design §4.2 no-op edits). An older card's sha is a new version (a revert).
- `Retire` appends `agent.retired`. Versions stay; afterwards `PutVersion`, `Retire` and `episode::Open` for the agent are refused with `AgentRetired` (soft retire).
- The tool allowlist is `capabilities.tools_allowed` (array of strings) in the card; absent means empty (design §4.3).

**`abw_ledger::episode`**

```rust
pub struct Usage { pub input_tokens: u64, pub output_tokens: u64, pub tool_calls: u64 }   // Default, Clone, Copy, PartialEq
pub struct Split { pub free_tier: u64, pub granted: u64, pub purchased: u64 }            // Debug, Clone, Copy, PartialEq
pub enum EpisodeStatus { Success, Partial, Failure }   // as_str "success", "partial", "failure"; Debug, Clone, Copy, PartialEq
pub enum ReasonClass { BudgetExhausted, IterationsExhausted, ProviderError, LedgerError, AbandonedTimeout, AbandonedDropped }
                                                      // as_str: snake_case of the variant
pub struct OpenEpisode { pub episode_id: Uuid, pub owner_id: String, pub agent_id: String, pub input: Value,
                         pub hold_cu: u64, pub price_policy: PolicyRef, pub grant_policy: PolicyRef, pub at: DateTime<Utc> }
pub struct Open;        // Open::new(OpenEpisode); Output = Opened
pub struct Opened { pub episode_id: Uuid, pub hold_id: Uuid, pub version_number: u64, pub card_sha: Sha256Ref,
                    pub seq: u64, pub free_tier_granted_cu: Option<u64> }
pub enum ToolOutcome { Ok(Value), Err { class: String, message: String } }
pub struct ToolRecord { pub iteration: u32, pub call_index: u32, pub tool_name: String, pub tool_version: String,
                        pub allowed: bool, pub input: Value, pub output: ToolOutcome, pub duration_ms: u64 }
pub struct RecordTool;  // RecordTool::new(episode_id: Uuid, expected: Expected, record: ToolRecord); Output = ToolRecorded
pub struct ToolRecorded { pub seq: u64, pub input_sha: Sha256Ref, pub output_sha: Sha256Ref }
pub struct CloseOutcome { pub status: EpisodeStatus, pub reason_class: Option<ReasonClass>, pub raw_output: Option<String>, pub usage: Usage }
pub struct Close;       // Close::new(episode_id, expected: Expected, outcome: CloseOutcome); Output = Closed
pub struct Closed { pub status: EpisodeStatus, pub consumed_cu: u64, pub released_cu: u64, pub split: Split, pub seq: u64 }
pub struct Abandon;     // Abandon::new(episode_id, expected: Expected, reason: ReasonClass); Output = Abandoned
pub struct Abandoned { pub released_cu: u64, pub seq: u64 }
```

- **TX-A, `Open`:** W = {`episode:<eid>` (new), `allowance:<owner>`}, R = {`agent:<id>`, both policy streams}. Appends, in order: the lazy free-tier grant if due, `allowance.hold_placed { hold_cu }`, `episode.opened` (with the head's version, card sha, `tools_allowed`, the input artifact `artifact_json(input)`, `opened_at = at`). Refusals, with nothing written: `AgentNotFound`, `AgentRetired`, `AllowanceNotOpen`, `TenantMismatch` (allowance, agent or episode stream of another tenant), policy refusals, `HoldAboveCap` (`hold_cu > hold_cap_cu`), `InsufficientAllowance` (`available` after the top-up `< hold_cu`, or `hold_cu = 0`). `hold_id` is new. Idempotent on `episode:<eid>:hold`.
- **TX-B, `RecordTool`:** one `tool.invoked` with two artifacts: input `artifact_json(input)` and output `artifact_json({"ok": v})` or `artifact_json({"error": {"class": c, "message": m}})`. When `allowed` is false, the same transaction also appends `gate.decided { gate: tool_allowlist, decision: refused, reason_class: policy, subject: "tool:<name>" }`, and the output must be `Err { class: "tool_allowlist", .. }` (else `InvalidToolRecord`). A record with `allowed: true` for a tool not in the episode's `tools_allowed` is `ToolNotAllowed`. Idempotent on `episode:<eid>:tool:<iteration>:<call_index>` [I].
- **TX-C, `Close`:** W = {`episode:<eid>`, `allowance:<owner>`}. Appends `episode.closed` and `allowance.consumed` in one transaction. `priced_cu = price(usage)` under the price version cited at open; `consumed_cu = min(priced_cu, hold_cu)`; `released_cu = hold_cu − consumed_cu`; the split draws `free_tier`, then `granted`, then `purchased`. Outcome rules: `Success` has no reason; `Partial` and `Failure` need one; `ProviderError` means `Failure`; `Abandoned*` and `LedgerError` are not accepted by `Close` (`InvalidOutcome`). Idempotent on `episode:<eid>:settle`: an identical retry replays; a different outcome is `IdempotencyMismatch`.
- **`Abandon`:** appends `episode.closed { status: failure, reason_class }` and `allowance.hold_released { amount_cu: hold_cu }` in one transaction (design §4.6, `economy.md` §5 step 4). Idempotent on `episode:<eid>:release`.
- Any command on a closed episode is refused. With a stale `expected`, tests accept either `Conflict` or `Rejected(EpisodeClosed)`; both write nothing.

**`abw_ledger::reaper`**

```rust
pub const EPISODE_MAX_RUNTIME: std::time::Duration;   // 30 minutes (design §4.5 ⟨6⟩)
pub struct ReaperReport { pub abandoned: Vec<Uuid>, pub raced: Vec<Uuid> }
pub enum ReaperError { .. }                           // Debug, thiserror
pub async fn run_once(ledger: &Ledger, now: DateTime<Utc>) -> Result<ReaperReport, ReaperError>;
```

One pass is both the episode reaper and the hold reaper, because closing and releasing are one transaction: every `running` episode with `opened_at + EPISODE_MAX_RUNTIME < now` gets `Abandon { reason: AbandonedTimeout }` at its current head. A `Conflict` or `EpisodeClosed` means another recoverer won; the episode goes in `raced` and nothing is written (design §4.7).

**`abw_ledger::crud`** (deferred from M1 by ADR-0005)

```rust
impl Ledger { pub fn crud(&self) -> &Crud; }
pub struct Crud;
pub struct IssuedKey { pub key_id: Uuid, pub secret: String }   // secret has ≥ 32 characters, shown once
pub enum CrudError { .. }                                        // Debug, thiserror
impl Crud {
    pub async fn issue_api_key(&self, tenant: &str, owner_id: &str, actor: &Actor, label: &str) -> Result<IssuedKey, CrudError>;
    pub async fn revoke_api_key(&self, key_id: Uuid) -> Result<(), CrudError>;
}
```

`key_sha = sha256_raw(secret.as_bytes())`. `execute_crud!` and general CRUD tables arrive when a milestone needs them.

### 5. `abw-agent` (new crate)

Dependencies: `abw-ledger`, `abw-canon`, `serde_json`, `uuid`, `chrono`, `tokio`, `reqwest` (only in the egress module, item 9), `thiserror`. Feature `fault-injection` enables `abw-ledger/fault-injection` and the test hooks below; it is never enabled outside tests. Dev-dependencies: itself with `fault-injection`, `sqlx` (as in `abw-ledger`, for the shared test harness `crates/abw-ledger/tests/common/`), `tokio` (`macros`, `rt-multi-thread`, `net`), `uuid`, `chrono`, `serde_json`.

```rust
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;
```

### 6. `abw_agent::card` and `abw_agent::import` (PROT-09)

```rust
pub const EXCLUDED_FIELDS: &[&str] = &["performance", "usage", "ontology_stats", "wallet", "declared_prompt_sha256"];
pub struct ResolvedCard;
impl ResolvedCard { pub fn agent_id(&self) -> &str; pub fn json(&self) -> &Value; pub fn card_sha(&self) -> Sha256Ref;
                    pub fn prompt_sha(&self) -> Sha256Ref; pub fn tools_allowed(&self) -> Vec<String>; }
pub enum CardError { .. }
pub fn resolve(raw: &Value) -> Result<ResolvedCard, CardError>;

// abw_agent::import
pub struct ImportedAgent { pub agent_id: String, pub version_number: u64, pub card_sha: Sha256Ref, pub created: bool }
pub struct ImportReport { pub agents: Vec<ImportedAgent> }
pub enum ImportError { .. }
pub fn read_dir(dir: &Path) -> Result<Vec<(String, Value)>, ImportError>;           // (slug, raw card), sorted by slug
pub async fn import_dir(ledger: &Ledger, dir: &Path, ctx: &CommandCtx) -> Result<ImportReport, ImportError>;
// ResolvedCard, CardError, ImportedAgent, ImportReport and ImportError derive Debug.
```

**Resolution rules** (design §4.2, §4.3), applied in this order and nothing else:
1. The raw card is a JSON object with a string `agent_id` that fits the stream-id grammar.
2. Remove each `EXCLUDED_FIELDS` key at the top level.
3. Set `"$excluded"` to `EXCLUDED_FIELDS` as an array of strings, in that order.
4. If `system_prompt` is absent or null, set it to `""`.
5. If `capabilities.tools_allowed` is absent, set it to the `name` of each `capabilities.mcp_tools` entry, in order, without duplicates (the one-time backfill of §4.3; an absent `capabilities` gets `{"tools_allowed": []}`).

`card_sha = sha256_jcs(json)`, which equals `agent::PutVersion`'s `card_sha` for the same JSON. `read_dir` reads `<dir>/<slug>/agent_card.json` and refuses a card whose `agent_id` differs from its directory name. `import_dir` dispatches `agent::PutVersion` per card (`Seed` for a new agent, `AdoptFile` otherwise) under `ctx`; a re-import of unchanged cards creates nothing. Field-origin merging (design §4.2 seed table) is deferred to the milestone that adds owner edits through HTTP [I].

### 7. `abw_agent::llm`, `abw_agent::tools`, `abw_agent::budget`, `abw_agent::executor` (CAP-01.02, CAP-01.03, ECO-5)

```rust
// llm
pub struct ToolSpec { pub name: String, pub description: String, pub input_schema: Value }
pub struct ToolCall { pub id: String, pub name: String, pub input: Value }
pub struct TokenUsage { pub input_tokens: u64, pub output_tokens: u64 }                  // Default
pub struct Turn { pub text: String, pub tool_calls: Vec<ToolCall>, pub usage: TokenUsage }
pub enum Message { User(String), Assistant { text: String, tool_calls: Vec<ToolCall> },
                   ToolResult { call_id: String, content: String, is_error: bool } }
pub struct LlmRequest { pub model: String, pub system: String, pub messages: Vec<Message>, pub tools: Vec<ToolSpec>, pub max_tokens: u32 }
pub enum LlmError { Transport(String), Provider { status: u16, message: String }, RateLimited, ScriptExhausted }
pub trait LlmProvider: Send + Sync {
    fn complete<'a>(&'a self, req: &'a LlmRequest) -> BoxFuture<'a, Result<Turn, LlmError>>;
}
pub enum Scripted { Turn(Turn), Error(LlmError) }
pub struct ScriptedProvider;   // Send + Sync
impl ScriptedProvider { pub fn new(script: Vec<Scripted>) -> ScriptedProvider;
                        pub fn calls(&self) -> usize; pub fn remaining(&self) -> usize; pub fn requests(&self) -> Vec<LlmRequest>; }
// complete() returns the next scripted item, records the request, and returns Err(ScriptExhausted) past the end.
// All llm types derive Debug, Clone, PartialEq.

// tools
pub struct ToolError { pub class: String, pub message: String }
pub trait Tool: Send + Sync {
    fn name(&self) -> &str; fn version(&self) -> &str; fn description(&self) -> &str; fn input_schema(&self) -> Value;
    fn call<'a>(&'a self, input: &'a Value) -> BoxFuture<'a, Result<Value, ToolError>>;
}
pub struct ToolRegistry;       // Default; new(); register(&mut self, tool: Arc<dyn Tool>)

// budget
pub const MAX_ITERATIONS: u32 = 5;   // design §4.1
pub const FLUSH_TURNS: u32 = 1;
pub fn estimate_hold(price: &PricePolicy, max_tokens: u32) -> u64;
// = min(hold_cap_cu, (cu_per_input_token + cu_per_output_token) * max_tokens * (MAX_ITERATIONS + FLUSH_TURNS) + hold_gas_cu)

// executor
pub struct RunRequest { pub episode_id: Uuid, pub owner_id: String, pub agent_id: String, pub task: String,
                        pub max_tokens: u32, pub at: DateTime<Utc> }
pub struct RunOutcome { pub episode_id: Uuid, pub status: EpisodeStatus, pub reason_class: Option<ReasonClass>,
                        pub llm_calls: u32, pub tool_calls: u32, pub held_cu: u64, pub consumed_cu: u64, pub output: Option<String> }
pub enum RunError { Rejected(Rejection), Ledger(DispatchError), Read(ReadError),
                    #[cfg(feature = "fault-injection")] Crashed(PipelineFault) }
pub struct Executor;
impl Executor {
    pub fn new(ledger: Arc<Ledger>, provider: Arc<dyn LlmProvider>, tools: ToolRegistry) -> Executor;
    #[cfg(feature = "fault-injection")] pub fn with_fault(self, fault: PipelineFault) -> Executor;
    pub async fn run(&self, req: RunRequest, ctx: &CommandCtx) -> Result<RunOutcome, RunError>;
}
#[cfg(feature = "fault-injection")]
pub enum PipelineFault { TxA(FaultPoint), TxB { call: u32, point: FaultPoint }, TxC(FaultPoint),
                         CrashAfterTxA, CrashAfterTxB { call: u32 }, CrashBeforeTxC }   // Debug, Clone, Copy, PartialEq
// RunOutcome and RunError derive Debug; ToolError derives Debug, Clone, PartialEq.
```

**Run semantics** (design §4.4–§4.7, `economy.md` §5):

1. Resolve the active grant and price policies for `ctx.tenant` at `req.at` (`policy::active`); a missing one is `RunError::Rejected(PolicyNotFound)`. `hold_cu = estimate_hold(price, max_tokens)`.
2. TX-A dispatches `episode::Open` with `input = {"task": task}`. A refusal returns `RunError::Rejected(..)`; a ledger failure returns `RunError::Ledger(..)`. **No provider call happens before TX-A commits.**
3. Loop: at most `MAX_ITERATIONS` LLM calls offering `tools = registry ∩ tools_allowed`, then at most one flush call offering no tools. A turn without tool calls ends the run with `Success` and its text as output. Tool calls in a turn run sequentially; each one is one TX-B. A call naming a tool not on the allowlist is not executed: it is recorded with `allowed: false` and the model gets a tool error. Tool calls returned by the flush turn are not executed or recorded, and the run closes `Partial` with `IterationsExhausted` [I].
4. **Metering.** Usage accumulates as `episode::Usage`; after every LLM call and every tool call, if `price(usage) ≥ hold_cu` the run stops at once (no further tool call, LLM call or flush) and closes `Partial` with `BudgetExhausted`.
5. A provider error closes `Failure` with `ProviderError` through TX-C, charging the usage metered so far. It never closes `Success`.
6. TX-C dispatches `episode::Close`; every run that commits TX-C returns `Ok(RunOutcome)`, whatever its status. `episode.closed.tool_call_shas` lists the `output_sha` of each recorded call in order [I]. If TX-C fails, the executor **does not** abandon: the episode stays running and the hold stays open for the reaper (`economy.md` §7, "a failed TX-C leaves the hold open") and the run returns `RunError::Ledger`.
7. If a TX-B fails, the run stops, dispatches `Abandon { LedgerError }`, and returns `RunError::Ledger`.
8. `PipelineFault` (tests only): `TxA/TxB/TxC(point)` make that dispatch fail at `point` (via `Ledger::inject_fault`); `call` counts tool calls in the run from 0. `Crash*` stop the run at that point as if the process died (no close, no abandon) and return `RunError::Crashed`.

### 8. `abw_agent::egress` (T18, CAP-14.05)

The design's `net_guard` (§12.5) is the module `abw_agent::egress`, files under `crates/abw-agent/src/egress*`. It is the only product code that constructs an outbound HTTP client or opens an outbound socket (`cargo xtask check`, check `egress`; `reqwest` and other HTTP client crates may only be dependencies of `abw-agent`).

```rust
pub struct OutboundPolicy { pub allow_http: bool }                    // Default: false
pub trait Resolver: Send + Sync { fn resolve<'a>(&'a self, host: &'a str) -> BoxFuture<'a, std::io::Result<Vec<IpAddr>>>; }
pub struct SystemResolver;                                            // impl Resolver
pub struct GuardedUrl;                                                // private fields; only check() builds it
impl GuardedUrl { pub fn url(&self) -> &str; pub fn host(&self) -> &str; pub fn pinned(&self) -> SocketAddr;
                  #[cfg(feature = "fault-injection")] pub fn pinned_for_test(url: &str, addr: SocketAddr) -> GuardedUrl; }
pub enum NetGuardError { InvalidUrl, Scheme, Unresolvable, Forbidden { addr: IpAddr, class: &'static str },
                         Port, Redirect { location: String }, Http(String) }   // Debug, Clone, PartialEq
pub fn classify(ip: IpAddr) -> Option<&'static str>;                  // Some(class) when forbidden
pub async fn check(url: &str, policy: &OutboundPolicy, resolver: &dyn Resolver) -> Result<GuardedUrl, NetGuardError>;
pub struct Fetched { pub status: u16, pub body: Vec<u8> }
pub async fn fetch(g: &GuardedUrl, timeout: Duration) -> Result<Fetched, NetGuardError>;   // GET to the pinned address, redirects refused
pub fn fixed_host_client(timeout: Duration) -> Result<reqwest::Client, NetGuardError>;   // for platform-fixed hosts (LLM APIs)
```

- Classes (exact strings): `loopback` (127/8, ::1), `unspecified` (0.0.0.0/8, ::), `private` (10/8, 172.16/12, 192.168/16), `cgnat` (100.64/10), `link_local` (169.254/16 including 169.254.169.254, fe80::/10), `ula` (fc00::/7), `multicast` (224/4, ff00::/8), `broadcast` (255.255.255.255), `documentation` (192.0.2/24, 198.51.100/24, 203.0.113/24, 2001:db8::/32). IPv4-mapped (`::ffff:a.b.c.d`) and IPv4-compatible (`::a.b.c.d`) addresses take the class of the embedded IPv4 address.
- `check`: scheme `https` (port 443) or, with `allow_http`, `http` (port 80); another scheme is `Scheme`, another port (including `http` without `allow_http`) is `Port`. IP literals are classified directly; names are resolved through `resolver`, and **any** forbidden address in the answer refuses the URL. No answer or a resolver error is `Unresolvable`. The result pins the first address with the URL's port. `check` runs at config write and again at call time, so a resolver that changes its answer (rebinding) is caught by the second call.
- `fetch` connects only to `pinned()`, sends `Host` from the URL, never follows a redirect: any 3xx is `Redirect { location }`. A caller that wants to follow re-runs `check` on the location.

### 9. `abw-http` (new crate; T17, CAP-14.04)

Dependencies: `abw-ledger`, `abw-canon`, `axum` 0.8, `serde`, `serde_json`, `uuid`, `tokio`; no `sqlx` (T1). Dev-dependencies: `tower` (`util`), `sqlx`, `tokio`, `uuid`, `chrono`, `serde_json`, `abw-ledger`.

```rust
#[derive(Clone)] pub struct AppState { pub ledger: Arc<Ledger>, pub db: ReadDb }
pub fn router(state: AppState) -> axum::Router;
pub struct RouteInfo { pub method: &'static str, pub path: &'static str }
pub fn route_table() -> Vec<RouteInfo>;   // the same table `router` is built from; axum 0.8 `{param}` syntax
```

Handlers use only `ReadDb` (via `read_query!`) and `Ledger::dispatch`. The principal comes from `Authorization: Bearer <secret>`: the handler hashes the secret and looks up a non-revoked `crud.api_keys` row, giving `Principal { tenant, owner_id, actor_id }`. A missing, unknown or revoked key is 401. Authorisation is one function per object kind (design §13.1); a caller who may not see an object gets **404 with the same body as for an id that doesn't exist**, never 403.

| Method | Path | Visible to | 200 body (fields tests read) |
| --- | --- | --- | --- |
| GET | `/health` | anyone | — |
| GET | `/api/v1/agents/{agent_id}` | the agent's tenant | `agent_id`, `version_number`, `card_sha`, `retired` |
| GET | `/api/v1/agents/{agent_id}/versions` | the agent's tenant | `versions: [{version_number, card_sha, reason}]` |
| GET | `/api/v1/agents/{agent_id}/versions/{version_number}` | the agent's tenant | `agent_id`, `version_number`, `card_sha`, `card` |
| GET | `/api/v1/episodes/{episode_id}` | the episode's owner | `episode_id`, `agent_id`, `status`, `reason_class`, `hold_cu`, `consumed_cu` |
| GET | `/api/v1/episodes/{episode_id}/tool-calls` | the episode's owner | `tool_calls: [{seq, tool_name, allowed, input_sha, output_sha}]` |
| GET | `/api/v1/episodes/{episode_id}/artifacts/{sha}` | the episode's owner, for a sha the episode cites | the artifact bytes |
| GET | `/api/v1/allowances/{owner_id}` | that owner | `owner_id`, `free_tier`, `granted`, `purchased`, `held`, `available` |

The IDOR test reads every GET route with a `{…}` parameter from `route_table()` and knows the parameters `agent_id`, `version_number`, `episode_id`, `sha` and `owner_id`. A new parameter name fails the test until a fixture for it is added; exemptions need a written reason in the test.

### 10. Fault points for the episode pipeline (T5)

The pipeline's crash points are design §4.7's C1–C3, reachable through `PipelineFault`: C1 = `CrashAfterTxA`; C2 = `CrashAfterTxB { call }`; C3 = `TxC(point)` for every `point` in `FaultPoint::ALL` (and `CrashBeforeTxC`); plus `TxA(point)` and `TxB { call, point }`. After C1, C2 or C3 the episode is running and the hold open; `reaper::run_once` later closes it `failure`/`abandoned_timeout` and releases the whole hold, with no consumption, in one transaction. Ledger-level tests reach the same states with `Ledger::inject_fault` on `episode::Close`.

### 11. Static checks added to `cargo xtask check`

- `sync-decide` (SUB-06): in `crates/*/src`, a `fn decide` may not be `async`, name `Future` in its signature, or contain `.await` in its body.
- `egress` (T18): in `crates/*/src` outside `crates/abw-agent/src/egress*`, no `reqwest::Client::new`, `reqwest::Client::builder`, `reqwest::ClientBuilder`, `reqwest::get`, `reqwest::blocking`, no `Client::new(`/`Client::builder(`/`ClientBuilder::new(` in a file that uses `reqwest`, no `hyper::client`, `hyper_util::client`, `ureq::`, `isahc::`, `surf::`, `attohttpc::`, `curl::`, and no `TcpStream::connect` or `UdpSocket::connect`.
- Dependencies: `reqwest`, `ureq`, `isahc`, `surf` and `attohttpc` only in `abw-agent` (T18).

### 12. Harness change

`cargo xtask pr-scope` accepts a product PR deleting the `#![cfg(any())]` line from a test file, as it accepts deleting `#[ignore = "pending ...` lines (AGENTS.md "How work is done"). Nothing else changes in a verification file of a product PR.

## Ambiguities resolved

Each takes the stricter reading. All are [I].

1. **Over-budget opens are refused, not shrunk.** `economy.md` §5 says the hold is "bounded by available"; design §4.5 ⟨3⟩ refuses when available is below the estimate. A hold is never silently reduced: `available < hold_cu` is `InsufficientAllowance`.
2. **Overshoot inside one turn is clamped, never charged.** One LLM response can push metered usage past the hold. `consumed_cu = min(priced_cu, hold_cu)`, with `priced_cu` recorded, so actual charge never exceeds the hold (ECO-5, no debt) and the overshoot is visible.
3. **The ledger prices the usage.** `Close` takes usage, not an amount, and prices it under the version cited at open, so every charge is recomputable from recorded data.
4. **A failed TX-C is left to the reaper**, not abandoned by the executor (`economy.md` §7, rank 2, over design §4.5 ⟨6⟩ "every ?-error path calls abandon"). A failed TX-B is abandoned (design ⟨6⟩), because the run can't continue without its evidence.
5. **Tool calls in the flush turn are neither executed nor recorded**, and the run closes `Partial`/`IterationsExhausted`: the flush offers no tools, and recording them as allowlist refusals would misstate the allowlist.
6. **The ledger enforces the allowlist on records too:** `allowed: true` for a tool off the episode's allowlist is refused, so a record can't claim an allowed call that wasn't.
7. **After retirement nothing is accepted** for the agent (edits, a second retire, new runs). History stays readable.
8. **The free-tier event is appended even when it grants 0**, so "first hold in the month" is decided by the stream alone.
9. **Visibility in M2:** agents are visible tenant-wide; episodes, their artifacts and allowances only to their owner. Sharing rules arrive with workspaces (M3).
10. **Any forbidden address in a DNS answer refuses the URL**, not only the first.
11. **Owners are globally unique ids**, so allowance idempotency keys can't collide across tenants; command keys are also prefixed by stream id.

## Consequences

- The M2 tests fix this surface; the product PR conforms to it. Changing an item needs a new ADR and a test PR.
- The register moves CAP-01.04 to M5, CAP-12.04 to M3 and CAP-14.06 to M4, adds SUB-06, and notes CAP-12.03.
- `ledger.policy_versions` gains two policy kinds through a migration; `proj.*` gains five inline projections and `crud` its first table.
- Tests tagged with an M1 requirement (T5) that can't run until the product PR are `#[ignore = "pending M2: ..."]`, so M1 stays green; GOV-05 makes sure they never count before they run.
