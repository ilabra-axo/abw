//! The CRUD domain's write path (target-system-design §14.5; ADR-0008 §4 `crud`).
//!
//! CRUD tables live in schema `crud` (core-delta §2.4) and are written only here, as role
//! `abw_crud` (`SET LOCAL ROLE` inside one transaction per write). Writes are typed methods;
//! no transaction, connection or pool is handed out (SUB-06, which supersedes §14.5's
//! `Crud::tx`).

use crate::ctx::Actor;
use abw_canon::sha256_raw;
use sqlx::postgres::PgPool;
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

/// Typed CRUD writes. Obtained from [`crate::Ledger::crud`].
#[derive(Clone)]
pub struct Crud {
    pool: PgPool,
}

impl std::fmt::Debug for Crud {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Crud")
    }
}

/// A newly issued API key. `secret` is shown once; only its hash is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedKey {
    pub key_id: Uuid,
    pub secret: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CrudError {
    #[error("invalid {field}: {value:?}")]
    Invalid { field: &'static str, value: String },
    #[error("API key {0} does not exist")]
    NotFound(Uuid),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

/// The prefix of every API key secret, so a leaked one is recognisable.
const SECRET_PREFIX: &str = "abw_";

fn ok_id(field: &'static str, v: &str) -> Result<(), CrudError> {
    if v.trim().is_empty() || v.len() > 256 || v.chars().any(char::is_control) {
        Err(CrudError::Invalid {
            field,
            value: v.to_string(),
        })
    } else {
        Ok(())
    }
}

impl Crud {
    pub(crate) fn new(pool: PgPool) -> Crud {
        Crud { pool }
    }

    /// Runs `f` in one transaction as `abw_crud` (§14.5 "SET LOCAL ROLE abw_crud").
    async fn as_crud<T, F>(&self, f: F) -> Result<T, CrudError>
    where
        F: for<'c> FnOnce(
            &'c mut PgConnection,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<T, CrudError>> + Send + 'c>,
        >,
    {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        sqlx::query("SET LOCAL ROLE abw_crud")
            .execute(&mut *tx)
            .await?;
        let out = f(&mut tx).await;
        match out {
            Ok(v) => {
                tx.commit().await?;
                Ok(v)
            }
            Err(e) => {
                tx.rollback().await?;
                Err(e)
            }
        }
    }

    /// Issues an API key for `owner_id` in `tenant`, created by `actor`. The secret has 244
    /// random bits (two v4 UUIDs) and is stored only as `sha256_raw(secret)`.
    pub async fn issue_api_key(
        &self,
        tenant: &str,
        owner_id: &str,
        actor: &Actor,
        label: &str,
    ) -> Result<IssuedKey, CrudError> {
        ok_id("tenant", tenant)?;
        ok_id("owner_id", owner_id)?;
        ok_id("actor", &actor.id)?;
        let key_id = Uuid::new_v4();
        let secret = format!(
            "{SECRET_PREFIX}{}{}",
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple()
        );
        let key_sha = sha256_raw(secret.as_bytes());
        let (tenant, owner, actor_id, label) = (
            tenant.to_string(),
            owner_id.to_string(),
            actor.id.clone(),
            label.to_string(),
        );
        self.as_crud(move |conn| {
            Box::pin(async move {
                sqlx::query(
                    "INSERT INTO crud.api_keys (key_id, tenant_id, owner_id, actor_id, key_sha, label)
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(key_id)
                .bind(tenant)
                .bind(owner)
                .bind(actor_id)
                .bind(key_sha.as_str())
                .bind(label)
                .execute(&mut *conn)
                .await?;
                Ok(())
            })
        })
        .await?;
        Ok(IssuedKey { key_id, secret })
    }

    /// Revokes a key. Revoking a revoked key is a no-op; an unknown key is `NotFound`.
    pub async fn revoke_api_key(&self, key_id: Uuid) -> Result<(), CrudError> {
        self.as_crud(move |conn| {
            Box::pin(async move {
                let exists: Option<bool> = sqlx::query_scalar(
                    "UPDATE crud.api_keys SET revoked_at = coalesce(revoked_at, now())
                      WHERE key_id = $1 RETURNING true",
                )
                .bind(key_id)
                .fetch_optional(&mut *conn)
                .await?;
                match exists {
                    Some(_) => Ok(()),
                    None => Err(CrudError::NotFound(key_id)),
                }
            })
        })
        .await
    }
}
