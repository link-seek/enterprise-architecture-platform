//! Verifies that the full migrator creates the `organizations` table and that
//! the `e2e_verify_1` column added by `m20250101_000068` exists,
//! is nullable, and defaults to `NULL`.

use sea_orm_migration::sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::sea_orm::ConnectionTrait;
use sea_orm_migration::MigratorTrait;

#[tokio::test]
async fn migrator_creates_organizations_with_nullable_e2e_verify_1() {
    let opt = ConnectOptions::new("sqlite::memory:").to_owned();
    let db: DatabaseConnection = Database::connect(opt).await.expect("connect sqlite");

    migration::Migrator::up(&db, None)
        .await
        .expect("migrator up");

    let backend = sea_orm_migration::sea_orm::DatabaseBackend::Sqlite;
    let uuid_str = "00000000-0000-0000-0000-000000000007";

    // Insert a row omitting the test column; the column must be nullable with a
    // NULL default for this to succeed.
    let insert_sql = format!(
        r#"INSERT INTO "organizations"
           ("id","name","created_at","updated_at")
           VALUES ('{uuid}','Acme','2020-01-01 00:00:00','2020-01-01 00:00:00')"#,
        uuid = uuid_str
    );
    db.execute_raw(sea_orm_migration::sea_orm::Statement::from_string(
        backend,
        insert_sql,
    ))
    .await
    .expect("insert row without e2e_verify_1");

    // The omitted column should read back as NULL.
    let row = db
        .query_one_raw(sea_orm_migration::sea_orm::Statement::from_string(
            backend,
            r#"SELECT "e2e_verify_1" FROM "organizations" WHERE "id" = '00000000-0000-0000-0000-000000000007'"#
                .to_string(),
        ))
        .await
        .expect("select")
        .expect("row exists");

    let value: Option<String> = row.try_get("", "e2e_verify_1").expect("get col");
    assert!(
        value.is_none(),
        "e2e_verify_1 should default to NULL when omitted"
    );

    // Updating the column to a non-null value should also work.
    db.execute_raw(sea_orm_migration::sea_orm::Statement::from_string(
        backend,
        r#"UPDATE "organizations" SET "e2e_verify_1" = 'test-value' WHERE "id" = '00000000-0000-0000-0000-000000000007'"#
            .to_string(),
    ))
    .await
    .expect("update e2e_verify_1");

    let row = db
        .query_one_raw(sea_orm_migration::sea_orm::Statement::from_string(
            backend,
            r#"SELECT "e2e_verify_1" FROM "organizations" WHERE "id" = '00000000-0000-0000-0000-000000000007'"#
                .to_string(),
        ))
        .await
        .expect("select")
        .expect("row exists");

    let value: String = row.try_get("", "e2e_verify_1").expect("get col");
    assert_eq!(value, "test-value");
}
