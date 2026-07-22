//! Verifies that the full migrator creates the `organizations` table and that
//! the `description` column added by `m20250101_000015` exists, is nullable,
//! and defaults to `NULL`.

use sea_orm_migration::sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::sea_orm::ConnectionTrait;
use sea_orm_migration::MigratorTrait;

#[tokio::test]
async fn migrator_creates_organizations_with_nullable_description() {
    let opt = ConnectOptions::new("sqlite::memory:").to_owned();
    let db: DatabaseConnection = Database::connect(opt).await.expect("connect sqlite");

    migration::Migrator::up(&db, None)
        .await
        .expect("migrator up");

    let backend = sea_orm_migration::sea_orm::DatabaseBackend::Sqlite;
    let uuid_str = "00000000-0000-0000-0000-000000000002";

    // Insert a row omitting `description`; the column must be nullable with a
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
    .expect("insert row without description");

    // The omitted description should read back as NULL.
    let row = db
        .query_one_raw(sea_orm_migration::sea_orm::Statement::from_string(
            backend,
            r#"SELECT "description" FROM "organizations" WHERE "id" = '00000000-0000-0000-0000-000000000002'"#
                .to_string(),
        ))
        .await
        .expect("select")
        .expect("row exists");

    let description: Option<String> = row.try_get("", "description").expect("get col");
    assert!(
        description.is_none(),
        "description should default to NULL when omitted"
    );

    // Updating the description to a non-null value should also work.
    db.execute_raw(sea_orm_migration::sea_orm::Statement::from_string(
        backend,
        r#"UPDATE "organizations" SET "description" = 'An enterprise architecture org' WHERE "id" = '00000000-0000-0000-0000-000000000002'"#
            .to_string(),
    ))
    .await
    .expect("update description");

    let row = db
        .query_one_raw(sea_orm_migration::sea_orm::Statement::from_string(
            backend,
            r#"SELECT "description" FROM "organizations" WHERE "id" = '00000000-0000-0000-0000-000000000002'"#
                .to_string(),
        ))
        .await
        .expect("select")
        .expect("row exists");

    let description: String = row.try_get("", "description").expect("get col");
    assert_eq!(description, "An enterprise architecture org");
}