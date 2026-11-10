//! Statement boundaries of the append (target-system-design §3.2.1), named so the T5
//! tests can abort a dispatch at each one (ADR-0004 item 4).
//!
//! The type always exists so the dispatch path reads the same with and without the
//! `fault-injection` feature; only that feature exports it and lets a fault be injected.

/// A point in `Ledger::dispatch` after which an injected fault aborts the transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FaultPoint {
    /// After step 2, the idempotency lookup.
    AfterDedup,
    /// After step 3, `lock_streams`.
    AfterLock,
    /// After step 6, the head compare-and-swap (and new stream rows).
    AfterHeads,
    /// After step 7, the artifact inserts.
    AfterArtifacts,
    /// After step 8, the event inserts.
    AfterEvents,
    /// After step 9, the inline projections.
    AfterProjections,
    /// After step 11, the outbox inserts.
    AfterOutbox,
    /// After step 12 (the dedup row) and step 14 (NOTIFY), immediately before COMMIT.
    BeforeCommit,
}

impl FaultPoint {
    /// Every point, in dispatch order.
    #[cfg_attr(not(feature = "fault-injection"), allow(dead_code))]
    pub const ALL: &'static [FaultPoint] = &[
        FaultPoint::AfterDedup,
        FaultPoint::AfterLock,
        FaultPoint::AfterHeads,
        FaultPoint::AfterArtifacts,
        FaultPoint::AfterEvents,
        FaultPoint::AfterProjections,
        FaultPoint::AfterOutbox,
        FaultPoint::BeforeCommit,
    ];
}
