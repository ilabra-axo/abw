//! Test database harness for the ledger conformance tests (ADR-0004 items 6 and 8).
//!
//! Each test gets its own database on the cluster named by `ABW_TEST_PG_URL` (a superuser
//! URL). The four roles of target-system-design §13.2 are provisioned here because roles
//! are cluster-level and live outside migrations. A missing `ABW_TEST_PG_URL` fails the
//! test: a database check that silently skips is a check that passes without running.

#![allow(dead_code)] // each test binary uses a subset

use abw_ledger::probe::{Append, Appended};
use abw_ledger::{Actor, CommandCtx, Expected, Ledger, LedgerConfig, ReadDb, SingletonFence};
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use sqlx::{Connection, PgConnection, Row};
use std::str::FromStr;

pub const OWNER: (&str, &str) = ("abw_owner", "abw_owner_test_pw");
pub const APP: (&str, &str) = ("abw_app", "abw_app_test_pw");
pub const READ: (&str, &str) = ("abw_read", "abw_read_test_pw");

pub fn admin_url() -> String {
    match std::env::var("ABW_TEST_PG_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => panic!(
            "ABW_TEST_PG_URL is not set. Database conformance tests must run against a real \
             PostgreSQL 16 cluster (superuser URL), e.g.\n  docker run -d --name abw-pg -e \
             POSTGRES_PASSWORD=postgres -p 55432:5432 postgres:16\n  export \
             ABW_TEST_PG_URL=postgres://postgres:postgres@localhost:55432/postgres"
        ),
    }
}

/// `scheme://user:pass@host:port/db?query` with user, password and database replaced.
pub fn url_for(base: &str, user: &str, password: &str, db: &str) -> String {
    let (scheme, rest) = match base.split_once("://") {
        Some(x) => x,
        None => panic!("ABW_TEST_PG_URL is not a URL: {base}"),
    };
    let host_part = rest.rsplit_once('@').map_or(rest, |(_, h)| h);
    let (host, query) = match host_part.split_once('?') {
        Some((h, q)) => (h, Some(q)),
        None => (host_part, None),
    };
    let host = host.split_once('/').map_or(host, |(h, _)| h);
    let mut url = format!("{scheme}://{user}:{password}@{host}/{db}");
    if let Some(q) = query {
        url.push('?');
        url.push_str(q);
    }
    url
}

fn db_of(url: &str) -> String {
    let after = url.split_once("://").map_or(url, |(_, r)| r);
    let after = after.rsplit_once('@').map_or(after, |(_, h)| h);
    let path = after.split_once('/').map_or("postgres", |(_, p)| p);
    path.split('?').next().unwrap_or("postgres").to_string()
}

async fn connect(url: &str) -> PgConnection {
    match PgConnectOptions::from_str(url) {
        Ok(opts) => match PgConnection::connect_with(&opts).await {
            Ok(c) => c,
            Err(e) => panic!("connect {url}: {e}"),
        },
        Err(e) => panic!("bad url {url}: {e}"),
    }
}

pub async fn exec(conn: &mut PgConnection, sql: &str) {
    if let Err(e) = sqlx::raw_sql(sql).execute(&mut *conn).await {
        panic!("{sql}\n  failed: {e}");
    }
}

/// Provisions roles once per cluster, serialised across test processes by an advisory lock.
async fn provision_roles(admin: &mut PgConnection) {
    exec(admin, "SELECT pg_advisory_lock(7212026)").await;
    let sql = format!(
        "DO $$ BEGIN
           IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '{o}') THEN
             CREATE ROLE {o} LOGIN PASSWORD '{op}'; END IF;
           IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '{a}') THEN
             CREATE ROLE {a} LOGIN PASSWORD '{ap}'; END IF;
           IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = '{r}') THEN
             CREATE ROLE {r} LOGIN PASSWORD '{rp}'; END IF;
           IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'abw_crud') THEN
             CREATE ROLE abw_crud NOLOGIN; END IF;
         END $$;
         GRANT abw_crud TO {a};
         ALTER ROLE {r} SET default_transaction_read_only = on;
         ALTER ROLE {r} SET statement_timeout = '60s';",
        o = OWNER.0,
        op = OWNER.1,
        a = APP.0,
        ap = APP.1,
        r = READ.0,
        rp = READ.1,
    );
    let result = sqlx::raw_sql(&sql).execute(&mut *admin).await;
    exec(admin, "SELECT pg_advisory_unlock(7212026)").await;
    if let Err(e) = result {
        panic!("provisioning roles failed: {e}");
    }
}

pub struct TestDb {
    pub name: String,
    pub admin_url: String,
    pub owner_url: String,
    pub app_url: String,
    pub read_url: String,
    /// Superuser pool on the test database, for inspection and for crafting violations.
    pub admin: PgPool,
}

impl TestDb {
    /// A fresh, empty database owned by `abw_owner`. Not migrated.
    pub async fn new() -> TestDb {
        let base = admin_url();
        let mut server = connect(&base).await;
        provision_roles(&mut server).await;
        let name = format!("abw_t_{}", uuid::Uuid::new_v4().simple());
        exec(
            &mut server,
            &format!("CREATE DATABASE {name} OWNER {}", OWNER.0),
        )
        .await;
        exec(
            &mut server,
            &format!(
                "REVOKE ALL ON DATABASE {name} FROM PUBLIC;
                 GRANT CONNECT ON DATABASE {name} TO {}, {}, {}",
                OWNER.0, APP.0, READ.0
            ),
        )
        .await;
        if let Err(e) = server.close().await {
            panic!("closing the server connection failed: {e}");
        }

        let (user, pass) = superuser(&base);
        let admin_url = url_for(&base, &user, &pass, &name);
        let admin = match PgPoolOptions::new()
            .max_connections(8)
            .connect(&admin_url)
            .await
        {
            Ok(p) => p,
            Err(e) => panic!("connect admin pool: {e}"),
        };
        TestDb {
            owner_url: url_for(&base, OWNER.0, OWNER.1, &name),
            app_url: url_for(&base, APP.0, APP.1, &name),
            read_url: url_for(&base, READ.0, READ.1, &name),
            admin_url,
            name,
            admin,
        }
    }

    /// A fresh database with every ledger migration applied.
    pub async fn migrated() -> TestDb {
        let db = TestDb::new().await;
        if let Err(e) = abw_ledger::migrate::run(&db.owner_url).await {
            panic!("migrate::run failed: {e:?}");
        }
        db
    }

    pub fn config(&self) -> LedgerConfig {
        LedgerConfig::new(&self.app_url, &self.read_url)
    }

    /// Holds the singleton fence and connects. The fence must outlive the ledger.
    pub async fn ledger(&self) -> (Ledger, ReadDb, SingletonFence) {
        let fence = match SingletonFence::acquire(&self.app_url).await {
            Ok(f) => f,
            Err(e) => panic!("fence: {e:?}"),
        };
        match Ledger::connect(&self.config(), &fence).await {
            Ok((ledger, read)) => (ledger, read, fence),
            Err(e) => panic!("Ledger::connect: {e:?}"),
        }
    }

    /// A single-connection session as the given role.
    pub async fn session(&self, url: &str) -> PgConnection {
        connect(url).await
    }

    pub async fn scalar_i64(&self, sql: &str) -> i64 {
        match sqlx::query(sql).fetch_one(&self.admin).await {
            Ok(row) => match row.try_get::<i64, _>(0) {
                Ok(v) => v,
                Err(e) => panic!("{sql}: {e}"),
            },
            Err(e) => panic!("{sql}: {e}"),
        }
    }

    /// Drops the database. Call at the end of a passing test; a failing test leaves its
    /// database behind for inspection.
    pub async fn finish(self) {
        self.admin.close().await;
        let mut server = connect(&admin_url()).await;
        exec(
            &mut server,
            &format!("DROP DATABASE IF EXISTS {} WITH (FORCE)", self.name),
        )
        .await;
    }
}

fn superuser(base: &str) -> (String, String) {
    let rest = base.split_once("://").map_or(base, |(_, r)| r);
    let userinfo = rest.rsplit_once('@').map_or("", |(u, _)| u);
    let (u, p) = userinfo.split_once(':').unwrap_or((userinfo, ""));
    (u.to_string(), p.to_string())
}

/// Runs `sql` as one transaction on `conn` and returns the error (including errors raised
/// by deferred triggers at COMMIT), or `None` if it committed.
pub async fn try_tx(conn: &mut PgConnection, sql: &str) -> Option<sqlx::Error> {
    let wrapped = format!("BEGIN; {sql}; COMMIT;");
    match sqlx::raw_sql(&wrapped).execute(&mut *conn).await {
        Ok(_) => None,
        Err(e) => {
            // The block is aborted (statement error) or already ended (COMMIT error); either
            // way ROLLBACK succeeds, and a failure here means the session itself is broken.
            if let Err(re) = sqlx::raw_sql("ROLLBACK").execute(&mut *conn).await {
                panic!("ROLLBACK after {e} failed: {re}");
            }
            Some(e)
        }
    }
}

/// The SQLSTATE and message of a database error.
pub fn db_error(e: &sqlx::Error) -> (String, String) {
    match e.as_database_error() {
        Some(d) => (
            d.code().map(|c| c.to_string()).unwrap_or_default(),
            d.message().to_string(),
        ),
        None => (String::new(), e.to_string()),
    }
}

/// Asserts `sql` fails as one transaction and the error message contains `needle`.
pub async fn assert_rejected(conn: &mut PgConnection, sql: &str, needle: &str) {
    match try_tx(conn, sql).await {
        None => panic!("expected rejection containing {needle:?}, but it committed:\n  {sql}"),
        Some(e) => {
            let (code, msg) = db_error(&e);
            assert!(
                msg.contains(needle) || code == needle,
                "expected {needle:?}, got [{code}] {msg}\n  {sql}"
            );
        }
    }
}

pub fn db_name(url: &str) -> String {
    db_of(url)
}

pub fn ctx() -> CommandCtx {
    CommandCtx::new("tenant-a", Actor::human("ivan"))
}

/// Appends one `probe.recorded` event and returns its output; panics on any error.
pub async fn append(
    ledger: &Ledger,
    id: &str,
    expected: Expected,
    body: serde_json::Value,
) -> Appended {
    match ledger
        .dispatch(Append::new(id, expected, body), &ctx())
        .await
    {
        Ok(d) => d.output,
        Err(e) => panic!("probe append to {id} failed: {e:?}"),
    }
}
