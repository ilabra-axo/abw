//! The single-writer boot fence (target-system-design §2.6, SUB-01).
//!
//! Step 1: the process takes `pg_try_advisory_lock(hashtext('abw-api-singleton'))` on a
//! dedicated session connection and holds it for its life; if the lock is held, boot
//! fails. Step 2: `Ledger::connect` refuses to open the write pool unless this fence holds
//! the lock on the same database (`boot.rs`). Dropping the fence closes its session, and
//! Postgres releases the lock when the backend exits.

use sqlx::postgres::PgConnectOptions;
use sqlx::{Connection, PgConnection, Row};
use std::str::FromStr;

pub(crate) const FENCE_KEY: &str = "abw-api-singleton";

#[derive(Debug, thiserror::Error)]
pub enum FenceError {
    #[error("invalid database URL: {0}")]
    Url(#[source] sqlx::Error),
    #[error("fence connection failed: {0}")]
    Db(#[from] sqlx::Error),
    /// Another process holds the fence: a second writer must not start (§2.6 step 1).
    #[error("the singleton fence on database {database} is held by another process")]
    Held { database: String },
}

/// Holds the singleton advisory lock until dropped. Must outlive the `Ledger` it opened.
pub struct SingletonFence {
    // Never read: it holds the session, and with it the lock.
    #[allow(dead_code)]
    conn: PgConnection,
    pub(crate) pid: i32,
    pub(crate) database: String,
}

impl std::fmt::Debug for SingletonFence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SingletonFence")
            .field("pid", &self.pid)
            .field("database", &self.database)
            .finish()
    }
}

impl SingletonFence {
    /// Takes the fence on the database at `url`, or fails if another session holds it.
    pub async fn acquire(url: &str) -> Result<SingletonFence, FenceError> {
        let opts = PgConnectOptions::from_str(url).map_err(FenceError::Url)?;
        let mut conn = PgConnection::connect_with(&opts).await?;
        let row = sqlx::query(
            "SELECT pg_try_advisory_lock(hashtext($1)) AS locked,
                    pg_backend_pid() AS pid, current_database()::text AS db",
        )
        .bind(FENCE_KEY)
        .fetch_one(&mut conn)
        .await?;
        let locked: bool = row.try_get("locked")?;
        let database: String = row.try_get("db")?;
        if !locked {
            conn.close().await?;
            return Err(FenceError::Held { database });
        }
        Ok(SingletonFence {
            pid: row.try_get("pid")?,
            database,
            conn,
        })
    }
}
