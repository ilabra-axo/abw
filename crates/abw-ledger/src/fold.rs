//! Step 4 of the append (target-system-design §3.2.1): reading the locked streams.
//!
//! A command's `fold` loads the events of the streams it locked, on the dispatch
//! connection and after `lock_streams`, so the state `decide` sees is exactly the state the
//! append extends. There are no snapshots yet (`ledger.aggregate_snapshots` stays empty);
//! M2 streams are short enough to fold from their first event [I].

use crate::command::BoxFuture;
use crate::error::DispatchError;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{PgConnection, Row};
use std::collections::BTreeMap;

/// One stored event of a locked stream.
#[derive(Debug, Clone)]
pub struct StoredEvent {
    pub(crate) event_type: String,
    pub(crate) payload: Value,
}

impl StoredEvent {
    /// The payload as `T`. A stored payload that doesn't decode is a ledger bug.
    pub(crate) fn decode<T: DeserializeOwned>(&self) -> Result<T, DispatchError> {
        serde_json::from_value(self.payload.clone()).map_err(|e| {
            DispatchError::Integrity(format!("stored {} payload: {e}", self.event_type))
        })
    }
}

/// The events of each loaded stream, in sequence order. Absent streams have no entry.
pub(crate) type Streams = BTreeMap<String, Vec<StoredEvent>>;

pub(crate) async fn load_streams(
    conn: &mut PgConnection,
    ids: &[String],
) -> Result<Streams, DispatchError> {
    let rows = sqlx::query(
        "SELECT stream_id, event_type, payload FROM ledger.events
          WHERE stream_id = ANY($1) ORDER BY stream_id, seq",
    )
    .bind(ids)
    .fetch_all(&mut *conn)
    .await?;
    let mut out = Streams::new();
    for r in rows {
        let stream: String = r.try_get("stream_id")?;
        out.entry(stream).or_default().push(StoredEvent {
            event_type: r.try_get("event_type")?,
            payload: r.try_get::<Json<Value>, _>("payload")?.0,
        });
    }
    Ok(out)
}

/// An already-resolved step, for commands that read nothing.
pub(crate) fn ready<'a, T: Send + 'a>(v: T) -> BoxFuture<'a, Result<T, DispatchError>> {
    Box::pin(async move { Ok(v) })
}

/// Serde for `Sha256Ref` as its `sha256:<hex>` string.
pub(crate) mod sha {
    use abw_canon::Sha256Ref;
    use serde::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(sha: &Sha256Ref, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(sha.as_str())
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Sha256Ref, D::Error> {
        let text = String::deserialize(d)?;
        Sha256Ref::parse(&text).map_err(serde::de::Error::custom)
    }
}

/// Serde for `Option<Sha256Ref>`.
pub(crate) mod opt_sha {
    use abw_canon::Sha256Ref;
    use serde::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(
        sha: &Option<Sha256Ref>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        match sha {
            Some(v) => s.serialize_str(v.as_str()),
            None => s.serialize_none(),
        }
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<Sha256Ref>, D::Error> {
        match Option::<String>::deserialize(d)? {
            Some(t) => Sha256Ref::parse(&t)
                .map(Some)
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

/// Serde for `Vec<Sha256Ref>`.
pub(crate) mod vec_sha {
    use abw_canon::Sha256Ref;
    use serde::ser::SerializeSeq;
    use serde::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S: Serializer>(v: &[Sha256Ref], s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(v.len()))?;
        for x in v {
            seq.serialize_element(x.as_str())?;
        }
        seq.end()
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Sha256Ref>, D::Error> {
        Vec::<String>::deserialize(d)?
            .iter()
            .map(|t| Sha256Ref::parse(t).map_err(serde::de::Error::custom))
            .collect()
    }
}

/// `at` truncated to the microsecond precision of `timestamptz`, as RFC 3339 in UTC, so a
/// timestamp in a payload and the same timestamp in a projection compare equal.
pub(crate) fn micros(at: chrono::DateTime<chrono::Utc>) -> chrono::DateTime<chrono::Utc> {
    use chrono::SubsecRound;
    at.trunc_subsecs(6)
}

pub(crate) fn rfc3339(at: chrono::DateTime<chrono::Utc>) -> String {
    micros(at).to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
}
