//! The read-only handle (target-system-design §3.8, §3.9 layer 2; ADR-0004 item 2).
//!
//! `ReadDb` is a pool connected as `abw_read`, whose sessions are read-only by role default
//! (checked at boot). It exposes no executor: reads go only through `fetch_one`,
//! `fetch_optional` and `fetch_all`, each taking a [`ReadQuery`] built by `read_query!`,
//! which refuses non-SELECT SQL at compile time.

use crate::error::ReadError;
use crate::sql_check;
use abw_canon::{Sha256Ref, sha256_raw};
use sqlx::postgres::{PgArguments, PgPool, PgRow};
use sqlx::{Arguments, Encode, Postgres, Row, Type};

#[derive(Clone)]
pub struct ReadDb {
    pool: PgPool,
}

impl std::fmt::Debug for ReadDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ReadDb")
    }
}

/// A read-only query. Build it with `abw_ledger::read_query!("SELECT …")`, then `.bind()`
/// its parameters in order.
pub struct ReadQuery {
    sql: &'static str,
    args: PgArguments,
    bind_error: Option<String>,
}

impl ReadQuery {
    /// Used by the `read_query!` expansion, which has already checked `sql`. Not an API:
    /// `ReadDb` re-checks every query, so calling this directly gains nothing.
    #[doc(hidden)]
    pub fn __from_checked_literal(sql: &'static str) -> ReadQuery {
        ReadQuery {
            sql,
            args: PgArguments::default(),
            bind_error: None,
        }
    }

    /// Binds the next `$n` parameter.
    pub fn bind<T>(mut self, value: T) -> ReadQuery
    where
        T: for<'q> Encode<'q, Postgres> + Type<Postgres> + Send + 'static,
    {
        if self.bind_error.is_none()
            && let Err(e) = self.args.add(value)
        {
            self.bind_error = Some(e.to_string());
        }
        self
    }

    pub fn sql(&self) -> &'static str {
        self.sql
    }

    fn checked(self) -> Result<(&'static str, PgArguments), ReadError> {
        sql_check::check(self.sql).map_err(ReadError::NotReadOnly)?;
        if let Some(e) = self.bind_error {
            return Err(ReadError::Bind(e));
        }
        Ok((self.sql, self.args))
    }
}

impl ReadDb {
    pub(crate) fn new(pool: PgPool) -> ReadDb {
        ReadDb { pool }
    }

    pub async fn fetch_all(&self, q: ReadQuery) -> Result<Vec<PgRow>, ReadError> {
        let (sql, args) = q.checked()?;
        Ok(sqlx::query_with(sql, args).fetch_all(&self.pool).await?)
    }

    pub async fn fetch_one(&self, q: ReadQuery) -> Result<PgRow, ReadError> {
        let (sql, args) = q.checked()?;
        Ok(sqlx::query_with(sql, args).fetch_one(&self.pool).await?)
    }

    pub async fn fetch_optional(&self, q: ReadQuery) -> Result<Option<PgRow>, ReadError> {
        let (sql, args) = q.checked()?;
        Ok(sqlx::query_with(sql, args)
            .fetch_optional(&self.pool)
            .await?)
    }

    /// The bytes of the artifact addressed by `sha`, or `None` if it is not stored
    /// (§3.8 "artifacts"). The bytes are re-hashed before they are returned.
    pub async fn resolve(&self, sha: &Sha256Ref) -> Result<Option<Vec<u8>>, ReadError> {
        let row = sqlx::query("SELECT bytes, storage_ref FROM ledger.artifacts WHERE sha = $1")
            .bind(sha.as_str())
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let bytes: Option<Vec<u8>> = row.try_get("bytes")?;
        let Some(bytes) = bytes else {
            return Err(ReadError::Artifact {
                sha: sha.to_string(),
                reason: "stored in object storage, which this build does not configure".into(),
            });
        };
        if sha256_raw(&bytes) != *sha {
            return Err(ReadError::Artifact {
                sha: sha.to_string(),
                reason: "stored bytes do not hash to their key".into(),
            });
        }
        Ok(Some(bytes))
    }
}
