//! Verifies migration `m20250101_000045_backfill_entity_owners`:
//! rows with `owner_id IS NULL` in the three business tables
//! (`value_streams`, `business_capabilities`, `business_processes`) are
//! backfilled to the first `role='owner'` member of their space.

use sea_orm_migration::sea_orm::{ConnectOptions, Database, DatabaseConnection, Statement};
use sea_orm_migration::sea_orm::ConnectionTrait;
use sea_orm_migration::MigratorTrait;

/// Binary literal matching how migration `m20250101_000029` stores the test
/// space id in SQLite (`Uuid` column type → 16-byte blob).
const TEST_SPACE_ID_BIN: &str = "X'00000000000000000000000000000010'";
const OWNER_USER_ID: &str = "00000000-0000-0000-0000-000000000001";
const VS_ID: &str = "00000000-0000-0000-0000-0000000000c1";
const CAP_ID: &str = "00000000-0000-0000-0000-0000000000c2";
const PROC_ID: &str = "00000000-0000-0000-0000-0000000000c3";

/// Run the first 44 migrations only — the owner backfill (migration 45)
/// stays pending so we can seed `owner_id IS NULL` rows first.
async fn setup_before_backfill() -> DatabaseConnection {
    let opt = ConnectOptions::new("sqlite::memory:").to_owned();
    let db: DatabaseConnection = Database::connect(opt).await.expect("connect sqlite");
    migration::Migrator::up(&db, Some(44)).await.expect("migrator up (first 44)");
    db
}

async fn insert_space_owner(db: &DatabaseConnection) {
    let backend = sea_orm_migration::sea_orm::DatabaseBackend::Sqlite;
    db.execute_raw(Statement::from_string(
        backend,
        format!(
            r#"INSERT INTO "users" ("id","email","name","password_hash","role","status","created_at","updated_at")
               VALUES ('{OWNER_USER_ID}','owner@example.com','owner','hash','Viewer','active','2020-01-01 00:00:00','2020-01-01 00:00:00')"#
        ),
    ))
    .await
    .expect("insert user");

    db.execute_raw(Statement::from_string(
        backend,
        format!(
            r#"INSERT INTO "space_members" ("space_id","user_id","role","created_at","updated_at")
               VALUES ({TEST_SPACE_ID_BIN},'{OWNER_USER_ID}','owner','2020-01-01 00:00:00','2020-01-01 00:00:00')"#
        ),
    ))
    .await
    .expect("insert space owner");
}

async fn insert_unowned_rows(db: &DatabaseConnection) {
    let backend = sea_orm_migration::sea_orm::DatabaseBackend::Sqlite;

    db.execute_raw(Statement::from_string(
        backend,
        format!(
            r#"INSERT INTO "value_streams" ("id","logical_id","business_version","status","name","description","triggering_event","end_deliverable","owner_id","importance","stakeholders","performance_metrics","created_at","updated_at","space_id")
               VALUES ('{VS_ID}','{VS_ID}','1.0.0','active','Backfill VS',NULL,NULL,NULL,NULL,'High','[]','{{}}','2020-01-01 00:00:00','2020-01-01 00:00:00',{TEST_SPACE_ID_BIN})"#
        ),
    ))
    .await
    .expect("insert value stream");

    db.execute_raw(Statement::from_string(
        backend,
        format!(
            r#"INSERT INTO "business_capabilities" ("id","logical_id","business_version","status","name","description","level","maturity","business_value","cost","owner_id","created_at","updated_at","space_id")
               VALUES ('{CAP_ID}','{CAP_ID}','1.0.0','active','Backfill Cap','desc','L1','M1','High','Medium',NULL,'2020-01-01 00:00:00','2020-01-01 00:00:00',{TEST_SPACE_ID_BIN})"#
        ),
    ))
    .await
    .expect("insert capability");

    db.execute_raw(Statement::from_string(
        backend,
        format!(
            r#"INSERT INTO "business_processes" ("id","logical_id","business_version","status","name","description","owner_id","created_at","updated_at","space_id")
               VALUES ('{PROC_ID}','{PROC_ID}','1.0.0','active','Backfill Proc','desc',NULL,'2020-01-01 00:00:00','2020-01-01 00:00:00',{TEST_SPACE_ID_BIN})"#
        ),
    ))
    .await
    .expect("insert process");
}

#[tokio::test]
async fn backfill_assigns_space_owner_to_unowned_rows() {
    let db = setup_before_backfill().await;
    let backend = sea_orm_migration::sea_orm::DatabaseBackend::Sqlite;

    insert_space_owner(&db).await;
    insert_unowned_rows(&db).await;

    // Run the remaining (backfill) migrations.
    migration::Migrator::up(&db, None).await.expect("migrator up (backfill)");

    for (table, id) in [
        ("value_streams", VS_ID),
        ("business_capabilities", CAP_ID),
        ("business_processes", PROC_ID),
    ] {
        let sql = format!(
            r#"SELECT "owner_id" FROM "{table}" WHERE "id" = '{id}'"#
        );
        let row = db
            .query_one_raw(Statement::from_string(backend, sql))
            .await
            .expect("query owner_id")
            .expect("row");
        let owner_id: String = row.try_get_by_index(0).expect("owner_id");
        assert_eq!(
            owner_id, OWNER_USER_ID,
            "{table} owner_id should be backfilled to the space owner"
        );
    }
}

#[tokio::test]
async fn backfill_skips_rows_when_space_has_no_owner() {
    let db = setup_before_backfill().await;
    let backend = sea_orm_migration::sea_orm::DatabaseBackend::Sqlite;

    // No space owner member: only the value stream row, space_id set to the
    // test space which has no `space_members` rows at all.
    db.execute_raw(Statement::from_string(
        backend,
        format!(
            r#"INSERT INTO "value_streams" ("id","logical_id","business_version","status","name","description","triggering_event","end_deliverable","owner_id","importance","stakeholders","performance_metrics","created_at","updated_at","space_id")
               VALUES ('{VS_ID}','{VS_ID}','1.0.0','active','NoOwner VS',NULL,NULL,NULL,NULL,'High','[]','{{}}','2020-01-01 00:00:00','2020-01-01 00:00:00',{TEST_SPACE_ID_BIN})"#
        ),
    ))
    .await
    .expect("insert value stream");

    // Must not fail: the migration skips rows whose space has no owner member.
    migration::Migrator::up(&db, None).await.expect("migrator up (backfill)");
}
