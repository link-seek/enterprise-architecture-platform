//! Temporary verification that the full migrator (including the new
//! `capability_status` column) applies cleanly against an in-memory SQLite
//! database and that the column exists with the expected default.

use sea_orm_migration::sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::sea_orm::ConnectionTrait;
use sea_orm_migration::MigratorTrait;

#[tokio::test]
async fn migrator_applies_and_capability_status_column_exists() {
    let opt = ConnectOptions::new("sqlite::memory:").to_owned();
    let db: DatabaseConnection = Database::connect(opt).await.expect("connect sqlite");

    migration::Migrator::up(&db, None)
        .await
        .expect("migrator up");

    let uuid_str = "00000000-0000-0000-0000-000000000001";
    let insert_sql = format!(
        r#"INSERT INTO "business_capabilities"
           ("id","logical_id","business_version","status","name","description",
            "level","maturity","business_value","cost","created_at","updated_at")
           VALUES ('{uuid}','{uuid}','1.0.0','active','n','d',
                   'level_1','initial','medium','medium','2020-01-01 00:00:00','2020-01-01 00:00:00')"#,
        uuid = uuid_str
    );
    let backend = sea_orm_migration::sea_orm::DatabaseBackend::Sqlite;
    db.execute_raw(sea_orm_migration::sea_orm::Statement::from_string(backend, insert_sql))
        .await
        .expect("insert row");

    let result = db
        .query_one_raw(sea_orm_migration::sea_orm::Statement::from_string(
            backend,
            r#"SELECT "capability_status" FROM "business_capabilities" WHERE "id" = '00000000-0000-0000-0000-000000000001'"#.to_string(),
        ))
        .await
        .expect("select")
        .expect("row exists");

    let status: String = result.try_get("", "capability_status").expect("get col");
    assert_eq!(status, "active", "default capability_status should be 'active'");
}