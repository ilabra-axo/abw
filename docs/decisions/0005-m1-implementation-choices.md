# ADR-0005: M1 implementation choices and one deviation from the design

- Status: accepted
- Date: 2026-11-10
- Decider: Ivan Labra

## Context

The M1 product PR (#3) implemented `abw-canon` and `abw-ledger` against the conformance tests of PR #2. The builder resolved nine points the design left open or that the tests constrained. They are recorded here so that later work treats them as decisions, not accidents. The tags are the builder's: [V] means verified against the design or tests, [I] means inferred.

## Decision

**Deviation (to revisit in M2).**

1. **There is no `!Send` transaction type** [V]. Target design §3.9 layer 4 makes "an LLM call inside a ledger transaction" a compile error by making `Tx` `!Send`. The M1 test `concurrent_appends_at_the_same_version_have_exactly_one_winner` runs `dispatch` inside `tokio::spawn`, so the dispatch future must be `Send`. For now the guard is the database-side `idle_in_transaction_session_timeout = 30s` only. **Follow-up for M2**, before the episode pipeline makes LLM calls: restore a compile-time guard that keeps `dispatch` `Send` but makes no transaction handle reachable outside the ledger, or change the test to drop `tokio::spawn`. This needs a test PR first.

**Accepted choices.**

2. **The idempotency record is written last** (§3.2.1 step 12, not step 2) [I]. Its stored result contains the event position, which exists only after the insert. Step 2 is a lookup that replays or rejects with `IdempotencyMismatch`.
3. **Event positions are allocated before the artifact inserts** [I]. This keeps the design's step order (artifacts at step 7, events at step 8) and the deferred `created_pos` foreign key. It needs `USAGE` on the events sequence for `abw_app` and `OVERRIDING SYSTEM VALUE` on the event insert, a grant §13.2 doesn't list.
4. **DDL beyond §3.1** [V]:
   - `ledger.schema_migrations` (ADR-0004), made immutable;
   - TRUNCATE triggers on every history table;
   - `proj.probes` gets `NOT NULL` and a foreign key on `applied_pos`, but no foreign key on `stream_id`, because the T3 test expects `ABW-T3` for a row citing a stream that doesn't exist.
5. **Only the `probe` stream kind is seeded** [I]. Other kinds arrive with their milestones; `wallet` lapsed with target design §9 (ADR-0003).
6. **`Expected::Exact(0)` means `NoStream`** [I].
7. **`Command` is sealed in M1** [I], with the probe as its only implementation. Fold (step 4) and provenance (step 10) are empty until real aggregates arrive. Commands refuse streams owned by another tenant.
8. **`rebuild_diff` replays in memory** [I], in one read-only snapshot as owner, and compares rows byte-exactly via canonical JSON. This replaces §3.4.3's shadow table.
9. **The app pool has no `statement_timeout`** [I]. The value is lost in the PDF text extraction of §2.3. `lock_timeout` and `idle_in_transaction_session_timeout` are set. Confirm the value from the PDF in M2.

**Deferred, by milestone:** `abw_ledger::crud` (M2, with the first CRUD table); async projectors (M3, SUB-04); object storage (artifacts over 256 KiB are refused until a milestone needs them).

## Consequences

The register's current milestone moves to M1, so CI now blocks any regression in T1–T6, T16, SUB-01..03 and SUB-05. Item 1 is tracked as an M2 entry condition.
