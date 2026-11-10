//! Inline projections (target-system-design §3.4, step 9 of §3.2.1).
//!
//! An inline projector runs inside the append transaction, after the events are inserted,
//! and writes its rows citing a `global_pos` appended by this transaction. The database
//! checks that citation at COMMIT (`proj.row_justified_by_event`, T3); [`ProjTx`] checks it
//! first in Rust, so a projector bug surfaces as a typed error instead of a failed commit.

use crate::command::Recorded;
use crate::error::DispatchError;
use crate::probe;
use crate::registry::PROBE_RECORDED;
use sqlx::PgConnection;
use sqlx::types::Json;

/// The append transaction as seen by a projector (§3.8 `ProjTx`): obtainable only inside
/// dispatch after step 8, and exposing only writer functions keyed by a position this
/// transaction appended.
pub(crate) struct ProjTx<'t> {
    conn: &'t mut PgConnection,
    appended: &'t [Recorded],
}

impl<'t> ProjTx<'t> {
    pub(crate) fn new(conn: &'t mut PgConnection, appended: &'t [Recorded]) -> ProjTx<'t> {
        ProjTx { conn, appended }
    }

    fn cite(&self, pos: i64) -> Result<(), DispatchError> {
        if self.appended.iter().any(|r| r.global_pos == pos) {
            Ok(())
        } else {
            Err(DispatchError::Integrity(format!(
                "projector cited event {pos}, which this transaction did not append"
            )))
        }
    }

    /// Upserts the `proj.probes` row of `stream_id`.
    async fn write_probe(
        &mut self,
        stream_id: &str,
        row: &probe::ProbeRow,
    ) -> Result<(), DispatchError> {
        self.cite(row.applied_pos)?;
        let seq = i64::try_from(row.seq)
            .map_err(|_| DispatchError::Integrity(format!("seq {} out of range", row.seq)))?;
        sqlx::query(
            "INSERT INTO proj.probes (stream_id, seq, body, applied_pos) VALUES ($1, $2, $3, $4)
             ON CONFLICT (stream_id) DO UPDATE
               SET seq = EXCLUDED.seq, body = EXCLUDED.body, applied_pos = EXCLUDED.applied_pos",
        )
        .bind(stream_id)
        .bind(seq)
        .bind(Json(&row.body))
        .bind(row.applied_pos)
        .execute(&mut *self.conn)
        .await?;
        Ok(())
    }
}

/// Applies every inline projector that handles `rec.event_type`.
pub(crate) async fn apply_inline(tx: &mut ProjTx<'_>, rec: &Recorded) -> Result<(), DispatchError> {
    if rec.event_type == PROBE_RECORDED.event_type {
        // probe.recorded replaces the whole row, so the fold needs no previous state.
        let row = probe::apply(None, rec.seq, &rec.payload, rec.global_pos);
        tx.write_probe(&rec.stream_id, &row).await?;
    }
    Ok(())
}
