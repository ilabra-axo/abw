//! Opening the ledger: pools and boot checks (target-system-design §2.3, §2.6, §3.7,
//! §13.2; ADR-0004 items 6 and 7).
//!
//! `Ledger::connect` refuses to start unless every check passes:
//!
//! 1. the singleton fence is held on this database (§2.6 step 2, SUB-01);
//! 2. the app role is not a superuser and owns no table, nor can it become an owner
//!    ("ownership bypasses REVOKE", §13.2);
//! 3. the read role is read-only: session default `default_transaction_read_only = on`,
//!    no write privilege on any ledger or projection table, and an INSERT into
//!    `ledger.events` is refused in a rolled-back probe transaction (§13.2);
//! 4. both handles point at the same database;
//! 5. the applied migrations equal this build's, name and checksum (SUB-02);
//! 6. the Rust event registry equals `ledger.event_types` (§3.7, OD-08).

use crate::crud::Crud;
use crate::error::BootError;
#[cfg(feature = "fault-injection")]
use crate::fault::FaultPoint;
use crate::fence::{FENCE_KEY, SingletonFence};
use crate::migrate;
use crate::read::ReadDb;
use crate::registry;
use abw_canon::sha256_jcs;
use serde_json::Value;
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use sqlx::types::Json;
use sqlx::{Connection, Executor, PgConnection, Row};
use std::str::FromStr;

/// Connection settings. The app URL is the `abw_app` role, the read URL `abw_read`.
#[derive(Clone)]
pub struct LedgerConfig {
    app_url: String,
    read_url: String,
}

impl std::fmt::Debug for LedgerConfig {
    // URLs carry passwords; never print them.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LedgerConfig { .. }")
    }
}

impl LedgerConfig {
    pub fn new(app_url: impl Into<String>, read_url: impl Into<String>) -> LedgerConfig {
        LedgerConfig {
            app_url: app_url.into(),
            read_url: read_url.into(),
        }
    }
}

/// The write boundary (§3.8). Its pool is private: no code outside this crate can name a
/// write-capable executor (§3.9 layer 1).
pub struct Ledger {
    pub(crate) pool: PgPool,
    read: ReadDb,
    crud: Crud,
    #[cfg(feature = "fault-injection")]
    pub(crate) fault: Option<FaultPoint>,
}

/// §2.3 LedgerPool: 10 max, 2 min.
const APP_POOL_MAX: u32 = 10;
const APP_POOL_MIN: u32 = 2;
/// §2.3 ReadDb: 20 max, 2 min.
const READ_POOL_MAX: u32 = 20;
const READ_POOL_MIN: u32 = 2;

impl Ledger {
    /// Opens the write pool and the read handle after the boot checks above.
    pub async fn connect(
        cfg: &LedgerConfig,
        fence: &SingletonFence,
    ) -> Result<(Ledger, ReadDb), BootError> {
        // Probe sessions first: nothing is pooled until the roles check out.
        let mut app = connect_one(&cfg.app_url, "connect the app handle").await?;
        check_fence(&mut app, fence).await?;
        check_app_role(&mut app).await?;
        // Schema before the read probe, so an unmigrated database reports NotMigrated.
        check_migrations(&mut app).await?;
        check_registry(&mut app).await?;

        let mut read = connect_one(&cfg.read_url, "connect the read handle").await?;
        check_read_role(&mut read).await?;
        check_same_database(&mut app, &mut read).await?;
        read.close()
            .await
            .map_err(BootError::db("close the read probe"))?;
        app.close()
            .await
            .map_err(BootError::db("close the app probe"))?;

        let pool = PgPoolOptions::new()
            .max_connections(APP_POOL_MAX)
            .min_connections(APP_POOL_MIN)
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    // §2.3 LedgerPool session settings; idle_in_transaction_session_timeout
                    // also kills any transaction held open across a non-ledger await.
                    conn.execute(sqlx::raw_sql(
                        "SET lock_timeout = '2s'; SET idle_in_transaction_session_timeout = '30s'",
                    ))
                    .await?;
                    Ok(())
                })
            })
            .connect(&cfg.app_url)
            .await
            .map_err(BootError::db("open the ledger pool"))?;
        let read_pool = PgPoolOptions::new()
            .max_connections(READ_POOL_MAX)
            .min_connections(READ_POOL_MIN)
            .connect(&cfg.read_url)
            .await
            .map_err(BootError::db("open the read pool"))?;
        let read = ReadDb::new(read_pool);
        Ok((
            Ledger {
                crud: Crud::new(pool.clone()),
                pool,
                read: read.clone(),
                #[cfg(feature = "fault-injection")]
                fault: None,
            },
            read,
        ))
    }

    /// A read handle on the same database.
    pub fn reader(&self) -> ReadDb {
        self.read.clone()
    }

    /// The CRUD domain's typed writes (target-system-design §14.5, ADR-0008 §4).
    pub fn crud(&self) -> &Crud {
        &self.crud
    }

    /// A ledger sharing this one's pools whose dispatches abort at `point` with
    /// `DispatchError::Injected(point)` (ADR-0004 item 4). Tests only.
    #[cfg(feature = "fault-injection")]
    pub fn inject_fault(&self, point: FaultPoint) -> Ledger {
        Ledger {
            pool: self.pool.clone(),
            read: self.read.clone(),
            crud: self.crud.clone(),
            fault: Some(point),
        }
    }
}

async fn connect_one(url: &str, what: &'static str) -> Result<PgConnection, BootError> {
    let opts = PgConnectOptions::from_str(url).map_err(BootError::db(what))?;
    PgConnection::connect_with(&opts)
        .await
        .map_err(BootError::db(what))
}

async fn check_fence(app: &mut PgConnection, fence: &SingletonFence) -> Result<(), BootError> {
    let row = sqlx::query(
        "SELECT current_database()::text AS db,
                EXISTS (SELECT 1 FROM pg_locks l
                         WHERE l.locktype = 'advisory' AND l.granted AND l.pid = $1
                           AND l.database = (SELECT oid FROM pg_database
                                              WHERE datname = current_database())
                           AND l.objsubid = 1
                           AND l.objid::bigint = (hashtext($2)::bigint & 4294967295)
                           AND l.classid::bigint = ((hashtext($2)::bigint >> 32) & 4294967295))
                  AS held",
    )
    .bind(fence.pid)
    .bind(FENCE_KEY)
    .fetch_one(&mut *app)
    .await
    .map_err(BootError::db("check the fence"))?;
    let db: String = row
        .try_get("db")
        .map_err(BootError::db("check the fence"))?;
    let held: bool = row
        .try_get("held")
        .map_err(BootError::db("check the fence"))?;
    if db != fence.database {
        return Err(BootError::FenceNotHeld(format!(
            "the fence is on database {}, the ledger on {db}",
            fence.database
        )));
    }
    if !held {
        return Err(BootError::FenceNotHeld(format!(
            "no session {} holds the lock on {db}",
            fence.pid
        )));
    }
    Ok(())
}

async fn check_app_role(app: &mut PgConnection) -> Result<(), BootError> {
    let row = sqlx::query(
        "SELECT current_user::text AS role, rolsuper OR rolbypassrls AS privileged
           FROM pg_roles WHERE rolname = current_user",
    )
    .fetch_one(&mut *app)
    .await
    .map_err(BootError::db("inspect the app role"))?;
    let role: String = row
        .try_get("role")
        .map_err(BootError::db("inspect the app role"))?;
    let privileged: bool = row
        .try_get("privileged")
        .map_err(BootError::db("inspect the app role"))?;
    if privileged {
        return Err(BootError::AppRolePrivileged(role));
    }
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT n.nspname || '.' || c.relname
           FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
          WHERE c.relkind IN ('r', 'p', 'v', 'm', 'f', 'S')
            AND n.nspname NOT IN ('pg_catalog', 'information_schema')
            AND n.nspname NOT LIKE 'pg\\_%'
            AND pg_has_role(current_user, c.relowner, 'MEMBER')
          ORDER BY 1",
    )
    .fetch_all(&mut *app)
    .await
    .map_err(BootError::db("inspect table ownership"))?;
    if !tables.is_empty() {
        return Err(BootError::AppRoleOwnsTables { role, tables });
    }
    Ok(())
}

async fn check_read_role(read: &mut PgConnection) -> Result<(), BootError> {
    let what = "inspect the read role";
    let row = sqlx::query(
        "SELECT current_setting('default_transaction_read_only') AS ro,
                r.rolsuper AS su
           FROM pg_roles r WHERE r.rolname = current_user",
    )
    .fetch_one(&mut *read)
    .await
    .map_err(BootError::db(what))?;
    let ro: String = row.try_get("ro").map_err(BootError::db(what))?;
    let su: bool = row.try_get("su").map_err(BootError::db(what))?;
    if su {
        return Err(BootError::ReadRoleCanWrite("it is a superuser".into()));
    }
    if ro != "on" {
        return Err(BootError::ReadRoleCanWrite(format!(
            "default_transaction_read_only is {ro}, not on"
        )));
    }
    let writable: Vec<String> = sqlx::query_scalar(
        "SELECT n.nspname || '.' || c.relname
           FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
          WHERE c.relkind IN ('r', 'p')
            AND n.nspname IN ('ledger', 'proj', 'prov', 'crud')
            AND (has_table_privilege(c.oid, 'INSERT') OR has_table_privilege(c.oid, 'UPDATE')
                 OR has_table_privilege(c.oid, 'DELETE') OR has_table_privilege(c.oid, 'TRUNCATE')
                 OR has_any_column_privilege(c.oid, 'INSERT')
                 OR has_any_column_privilege(c.oid, 'UPDATE'))
          ORDER BY 1",
    )
    .fetch_all(&mut *read)
    .await
    .map_err(BootError::db(what))?;
    if !writable.is_empty() {
        return Err(BootError::ReadRoleCanWrite(format!(
            "it has write privileges on {writable:?}"
        )));
    }
    // §13.2: "abw_read must not be able to INSERT into ledger.events, which is probed with
    // a rolled-back transaction". Refusal must come from the read-only session (25006) or
    // from missing grants (42501); any other outcome means the write got further.
    let mut tx = read.begin().await.map_err(BootError::db(what))?;
    let probe = sqlx::query(
        "INSERT INTO ledger.events (stream_id, seq, event_type, schema_version, payload,
            payload_sha, command_id, correlation_id, actor, actor_kind)
         VALUES ('probe:boot-check', 1, 'probe.recorded', 1, '{}',
            'sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a',
            gen_random_uuid(), gen_random_uuid(), 'boot', 'platform')",
    )
    .execute(&mut *tx)
    .await;
    tx.rollback().await.map_err(BootError::db(what))?;
    match probe {
        Err(e)
            if e.as_database_error()
                .and_then(|d| d.code())
                .is_some_and(|c| c == "25006" || c == "42501") =>
        {
            Ok(())
        }
        Err(e) => Err(BootError::ReadRoleCanWrite(format!(
            "the INSERT probe was not refused by role or session: {e}"
        ))),
        Ok(_) => Err(BootError::ReadRoleCanWrite(
            "the INSERT probe succeeded (rolled back)".into(),
        )),
    }
}

async fn check_same_database(
    app: &mut PgConnection,
    read: &mut PgConnection,
) -> Result<(), BootError> {
    let q = "SELECT current_database()::text || ':' || coalesce(inet_server_port()::text, '')";
    let a: String = sqlx::query_scalar(q)
        .fetch_one(&mut *app)
        .await
        .map_err(BootError::db("identify the app database"))?;
    let r: String = sqlx::query_scalar(q)
        .fetch_one(&mut *read)
        .await
        .map_err(BootError::db("identify the read database"))?;
    if a != r {
        return Err(BootError::DatabaseMismatch { app: a, read: r });
    }
    Ok(())
}

async fn check_migrations(app: &mut PgConnection) -> Result<(), BootError> {
    let recorded = migrate::recorded(&mut *app)
        .await
        .map_err(BootError::db("read ledger.schema_migrations"))?
        .ok_or(BootError::NotMigrated)?;
    let embedded = migrate::embedded();
    if recorded != embedded {
        let show = |v: &[(String, String)]| {
            v.iter()
                .map(|(n, s)| format!("{n}={s}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        return Err(BootError::MigrationMismatch(format!(
            "database has [{}], this build has [{}]",
            show(&recorded),
            show(&embedded)
        )));
    }
    Ok(())
}

async fn check_registry(app: &mut PgConnection) -> Result<(), BootError> {
    let what = "read ledger.event_types";
    let rows = sqlx::query(
        "SELECT event_type, schema_version, stream_kind, epistemic, json_schema, json_schema_sha
           FROM ledger.event_types",
    )
    .fetch_all(&mut *app)
    .await
    .map_err(BootError::db(what))?;
    let mut table = Vec::with_capacity(rows.len());
    for r in rows {
        let event_type: String = r.try_get("event_type").map_err(BootError::db(what))?;
        let sha: String = r.try_get("json_schema_sha").map_err(BootError::db(what))?;
        let schema: Json<Value> = r.try_get("json_schema").map_err(BootError::db(what))?;
        // The stored sha must describe the stored schema, or the comparison means nothing.
        let actual = sha256_jcs(&schema.0)
            .map_err(|e| BootError::InvalidRegistry(format!("{event_type}: {e}")))?;
        if actual.as_str() != sha {
            return Err(BootError::InvalidRegistry(format!(
                "{event_type}: json_schema_sha {sha} does not match its schema ({actual})"
            )));
        }
        table.push(registry::row_key(
            &event_type,
            r.try_get("schema_version").map_err(BootError::db(what))?,
            &r.try_get::<String, _>("stream_kind")
                .map_err(BootError::db(what))?,
            &r.try_get::<String, _>("epistemic")
                .map_err(BootError::db(what))?,
            &sha,
        ));
    }
    table.sort();
    let rust = registry::registry_rows().map_err(BootError::InvalidRegistry)?;
    if table != rust {
        return Err(BootError::RegistryMismatch {
            missing_in_table: rust
                .iter()
                .filter(|k| !table.contains(k))
                .cloned()
                .collect(),
            unknown_to_build: table
                .iter()
                .filter(|k| !rust.contains(k))
                .cloned()
                .collect(),
        });
    }
    Ok(())
}
