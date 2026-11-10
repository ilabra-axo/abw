# ADR-0004: M1 conformance surface: the probe aggregate and the API the tests pin

- Status: accepted
- Date: 2026-11-10
- Decider: Ivan Labra

## Context

M1 builds the event store (`abw-canon`, `abw-ledger`) before any domain aggregate exists. Its conformance tests (T1–T6, SUB-01..03, SUB-05) must exercise append, projection, artifacts, outbox and idempotency through the real dispatch path. The target design gives the API shape (§3.8) but leaves some test-facing details open, and §3.8 and §3.9 disagree on one point.

## Decision

1. **A conformance probe aggregate ships in `abw-ledger`** as module `abw_ledger::probe`:
   - stream kind `probe` (concurrency `exact`);
   - event type `probe.recorded` v1, epistemic `structural`, payload any JSON object;
   - inline projection `proj.probes (stream_id text UNIQUE, seq bigint, body jsonb, applied_pos bigint)` with the T3 justification trigger;
   - command `probe::Append::new(id, Expected, body)` with builder methods `.idempotency_key(k)`, `.artifact(PendingArtifact)` and `.outbox(OutboxJob)`; output `probe::Appended { stream_id, seq, global_pos, payload_sha }`.

   Domain code never uses it: `cargo xtask check` fails if anything outside `crates/abw-ledger` and tests names `abw_ledger::probe`. It exists so the substrate is proven before the first real aggregate (allowance, agent, episode) arrives in M2.
2. **`ReadDb` has no method that returns a sqlx executor.** Reads go only through `fetch_one`, `fetch_optional` and `fetch_all`, each taking a `ReadQuery` built by `read_query!`. This follows §3.9 layer 2 and the T1 acceptance ("fails to compile"); it supersedes `as_executor` in §3.8.
3. **Migrations live in `crates/abw-ledger/migrations/`**, embedded in the crate, and are recorded in `ledger.schema_migrations (name text PRIMARY KEY, sha text NOT NULL, applied_at timestamptz)`. `abw_ledger::migrate::run(owner_url)` returns `MigrationReport { applied: Vec<String> }`.
4. **Fault injection** for T5 is behind the cargo feature `fault-injection`: `Ledger::inject_fault(FaultPoint) -> Ledger` makes the next dispatches fail at that point with `DispatchError::Injected(FaultPoint)`. `FaultPoint` has at least `AfterDedup`, `AfterLock`, `AfterHeads`, `AfterArtifacts`, `AfterEvents`, `AfterProjections`, `AfterOutbox` and `BeforeCommit`, and a `FaultPoint::ALL` slice. The feature is never enabled outside tests.
5. **Projection rebuild** is `abw_ledger::admin::rebuild_diff(owner_url, "proj.probes") -> Result<ProjectionDiff, AdminError>`, with `ProjectionDiff::is_empty()`.
6. **Provisioning is outside migrations.** Roles (`abw_owner`, `abw_app`, `abw_read`, `abw_crud`) are cluster-level and created by deployment provisioning; `abw_read` has `default_transaction_read_only = on`. The test harness provisions them itself. `Ledger::connect` performs the §13.2 boot checks.
7. **Public test-facing types.** Besides the names above, the tests use: `Ledger::{connect, dispatch, reader, inject_fault}`; `LedgerConfig::new(app_url, read_url)`; `SingletonFence::acquire(url)`, which holds the lock until dropped and is checked against the target database by `connect`; `ReadDb::{fetch_all, resolve}` (`ReadDb: Clone`); `CommandCtx::new(tenant, Actor).with_correlation(Uuid)`; `Actor::{human, agent}`; `Expected::{NoStream, Exact(u64)}` (`Copy + PartialEq + Debug`); `Dispatched<O> { output, replayed }` (`Debug`); `DispatchError::{Rejected(_), Conflict { expected, actual, .. }, IdempotencyMismatch { .. }, Injected(FaultPoint)}`; `artifact_bytes(Vec<u8>, Canon, &str)`, `artifact_json(&Value, &str)`, `PendingArtifact::sha()`; `OutboxJob::new(topic, dedupe_key, payload)`; `Canon::{JcsRfc8785, RawUtf8, RawBytes}`. From `abw-canon`: `parse`, `jcs`, `sha256_jcs`, `sha256_raw`, `Sha256Ref::{parse, as_str}` (`Display`, `Clone`, `PartialEq`).
8. **Test database.** Database tests read a superuser URL from `ABW_TEST_PG_URL` and **fail** if it is unset. A skipped database test would be a check that passes without running. Each test creates its own database.

## Consequences

- The M1 tests fix this surface; the implementation PR conforms to it.
- `proj.probes` and the `probe` kind exist in production schemas. They are inert: no domain path writes them.
- Changing any item above needs a new ADR and a test PR.
