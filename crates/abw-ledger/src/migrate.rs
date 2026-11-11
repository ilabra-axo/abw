//! The migration runner (target-system-design §14.6 note "G16 runner", ADR-0004 item 3,
//! SUB-02).
//!
//! Migrations are embedded in the crate and run as `abw_owner`. Each runs once, in its own
//! transaction together with its `ledger.schema_migrations` row (name and `sha256:` of the
//! file bytes). A recorded migration whose checksum differs from this build, or a recorded
//! name this build does not know, stops the runner before anything is applied; the same
//! comparison gates boot (`boot.rs`).

use abw_canon::sha256_raw;
use sqlx::postgres::PgConnectOptions;
use sqlx::{Connection, PgConnection, Row};
use std::str::FromStr;

pub(crate) struct Migration {
    pub(crate) name: &'static str,
    pub(crate) sql: &'static str,
}

/// Every migration, in order. Names sort in application order.
pub(crate) const MIGRATIONS: &[Migration] = &[
    Migration {
        name: "0001_ledger.sql",
        sql: include_str!("../migrations/0001_ledger.sql"),
    },
    Migration {
        name: "0002_probe.sql",
        sql: include_str!("../migrations/0002_probe.sql"),
    },
    Migration {
        name: "0003_m2.sql",
        sql: include_str!("../migrations/0003_m2.sql"),
    },
];

/// Serialises concurrent runners on one database.
const MIGRATE_LOCK_KEY: &str = "abw-migrate";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    /// Names applied by this run, in order. Empty when the database was up to date.
    pub applied: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum MigrateError {
    #[error("invalid database URL: {0}")]
    Url(#[source] sqlx::Error),
    #[error("{what}: {source}")]
    Db {
        what: &'static str,
        #[source]
        source: sqlx::Error,
    },
    #[error("migration {name} was applied with {recorded}, but this build has {embedded}")]
    ChecksumMismatch {
        name: String,
        recorded: String,
        embedded: String,
    },
    #[error("applied migrations {recorded:?} are not a prefix of this build's {embedded:?}")]
    UnknownHistory {
        recorded: Vec<String>,
        embedded: Vec<String>,
    },
    #[error("migration {name} failed: {source}")]
    Failed {
        name: String,
        #[source]
        source: sqlx::Error,
    },
}

fn db(what: &'static str) -> impl FnOnce(sqlx::Error) -> MigrateError {
    move |source| MigrateError::Db { what, source }
}

/// `(name, sha)` of every embedded migration.
pub(crate) fn embedded() -> Vec<(String, String)> {
    MIGRATIONS
        .iter()
        .map(|m| {
            (
                m.name.to_string(),
                sha256_raw(m.sql.as_bytes()).as_str().to_string(),
            )
        })
        .collect()
}

/// `(name, sha)` recorded in the database, in name order, or `None` if the record table
/// does not exist (an unmigrated database).
pub(crate) async fn recorded(
    conn: &mut PgConnection,
) -> Result<Option<Vec<(String, String)>>, sqlx::Error> {
    let exists: bool =
        sqlx::query_scalar("SELECT to_regclass('ledger.schema_migrations') IS NOT NULL")
            .fetch_one(&mut *conn)
            .await?;
    if !exists {
        return Ok(None);
    }
    let rows = sqlx::query("SELECT name, sha FROM ledger.schema_migrations ORDER BY name")
        .fetch_all(&mut *conn)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        out.push((r.try_get("name")?, r.try_get("sha")?));
    }
    Ok(Some(out))
}

/// Checks that `recorded` is a prefix of `embedded`, name and checksum.
pub(crate) fn verify_prefix(
    recorded: &[(String, String)],
    embedded: &[(String, String)],
) -> Result<(), MigrateError> {
    for (i, (name, sha)) in recorded.iter().enumerate() {
        match embedded.get(i) {
            Some((n, s)) if n == name => {
                if s != sha {
                    return Err(MigrateError::ChecksumMismatch {
                        name: name.clone(),
                        recorded: sha.clone(),
                        embedded: s.clone(),
                    });
                }
            }
            _ => {
                return Err(MigrateError::UnknownHistory {
                    recorded: recorded.iter().map(|(n, _)| n.clone()).collect(),
                    embedded: embedded.iter().map(|(n, _)| n.clone()).collect(),
                });
            }
        }
    }
    Ok(())
}

/// Applies every pending migration to the database at `owner_url` (role `abw_owner`).
pub async fn run(owner_url: &str) -> Result<MigrationReport, MigrateError> {
    let opts = PgConnectOptions::from_str(owner_url).map_err(MigrateError::Url)?;
    let mut conn = PgConnection::connect_with(&opts)
        .await
        .map_err(db("connect as owner"))?;
    sqlx::query("SELECT pg_advisory_lock(hashtext($1))")
        .bind(MIGRATE_LOCK_KEY)
        .execute(&mut conn)
        .await
        .map_err(db("take the migration lock"))?;
    let result = apply_pending(&mut conn).await;
    // Closing the session releases the advisory lock as well; unlock explicitly first so
    // a pooled or proxied session cannot keep it.
    let unlocked = sqlx::query("SELECT pg_advisory_unlock(hashtext($1))")
        .bind(MIGRATE_LOCK_KEY)
        .execute(&mut conn)
        .await
        .map_err(db("release the migration lock"));
    let closed = conn.close().await.map_err(db("close the owner connection"));
    let report = result?;
    unlocked?;
    closed?;
    Ok(report)
}

async fn apply_pending(conn: &mut PgConnection) -> Result<MigrationReport, MigrateError> {
    let embedded = embedded();
    let recorded = recorded(&mut *conn)
        .await
        .map_err(db("read ledger.schema_migrations"))?
        .unwrap_or_default();
    verify_prefix(&recorded, &embedded)?;

    let mut applied = Vec::new();
    for (m, (name, sha)) in MIGRATIONS.iter().zip(&embedded).skip(recorded.len()) {
        let failed = |source| MigrateError::Failed {
            name: name.clone(),
            source,
        };
        let mut tx = conn.begin().await.map_err(failed)?;
        sqlx::raw_sql(m.sql)
            .execute(&mut *tx)
            .await
            .map_err(failed)?;
        sqlx::query("INSERT INTO ledger.schema_migrations (name, sha) VALUES ($1, $2)")
            .bind(name)
            .bind(sha)
            .execute(&mut *tx)
            .await
            .map_err(failed)?;
        tx.commit().await.map_err(failed)?;
        applied.push(name.clone());
    }
    Ok(MigrationReport { applied })
}
