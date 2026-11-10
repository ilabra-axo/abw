//! SUB-01: single-writer boot fence. SUB-02: migrations run once, are checksummed, and
//! gate boot.

mod common;

use abw_ledger::{Ledger, SingletonFence, migrate};
use common::TestDb;

// @req SUB-01
#[tokio::test]
async fn a_second_process_cannot_take_the_fence() {
    let db = TestDb::migrated().await;
    let first = match SingletonFence::acquire(&db.app_url).await {
        Ok(f) => f,
        Err(e) => panic!("{e:?}"),
    };
    assert!(
        SingletonFence::acquire(&db.app_url).await.is_err(),
        "two fences held at once"
    );
    drop(first);
    // Released when the holder goes away (its session ends).
    let mut taken = None;
    for _ in 0..50 {
        if let Ok(f) = SingletonFence::acquire(&db.app_url).await {
            taken = Some(f);
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(
        taken.is_some(),
        "the fence was not released after its holder dropped it"
    );
    drop(taken);
    db.finish().await;
}

// @req SUB-01
#[tokio::test]
async fn the_ledger_refuses_to_open_without_its_fence() {
    let db = TestDb::migrated().await;
    let other = TestDb::migrated().await;
    // A fence on another database does not authorise writes to this one.
    let foreign = match SingletonFence::acquire(&other.app_url).await {
        Ok(f) => f,
        Err(e) => panic!("{e:?}"),
    };
    assert!(Ledger::connect(&db.config(), &foreign).await.is_err());
    drop(foreign);
    other.finish().await;
    db.finish().await;
}

// @req SUB-02
#[tokio::test]
async fn migrations_apply_once() {
    let db = TestDb::new().await;
    let first = match migrate::run(&db.owner_url).await {
        Ok(r) => r,
        Err(e) => panic!("{e:?}"),
    };
    assert!(
        !first.applied.is_empty(),
        "a fresh database applied no migrations"
    );
    let recorded = db
        .scalar_i64("SELECT count(*) FROM ledger.schema_migrations")
        .await;
    assert_eq!(recorded, first.applied.len() as i64);
    let second = match migrate::run(&db.owner_url).await {
        Ok(r) => r,
        Err(e) => panic!("{e:?}"),
    };
    assert!(
        second.applied.is_empty(),
        "re-run applied {:?}",
        second.applied
    );
    db.finish().await;
}

// @req SUB-02
#[tokio::test]
async fn an_unmigrated_database_does_not_boot() {
    let db = TestDb::new().await;
    // The fence is an advisory lock and needs no schema; boot must fail on the schema.
    let fence = match SingletonFence::acquire(&db.app_url).await {
        Ok(f) => f,
        Err(e) => panic!("the fence must not depend on migrations: {e:?}"),
    };
    assert!(Ledger::connect(&db.config(), &fence).await.is_err());
    drop(fence);
    db.finish().await;
}

// @req SUB-02
#[tokio::test]
async fn a_changed_checksum_stops_migration_and_boot() {
    let db = TestDb::migrated().await;
    let mut admin = db.session(&db.admin_url).await;
    common::exec(
        &mut admin,
        "SET session_replication_role = replica;
         UPDATE ledger.schema_migrations
            SET sha = 'sha256:0000000000000000000000000000000000000000000000000000000000000000'
          WHERE name = (SELECT min(name) FROM ledger.schema_migrations);
         SET session_replication_role = origin;",
    )
    .await;
    assert!(
        migrate::run(&db.owner_url).await.is_err(),
        "a tampered checksum was accepted"
    );
    let fence = match SingletonFence::acquire(&db.app_url).await {
        Ok(f) => f,
        Err(e) => panic!("{e:?}"),
    };
    assert!(
        Ledger::connect(&db.config(), &fence).await.is_err(),
        "boot accepted a tampered checksum"
    );
    drop(fence);
    db.finish().await;
}
