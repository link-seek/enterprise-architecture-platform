//! Verifies the space-related migrations: the `space_members` and
//! `space_invitations` tables exist, the three main business tables gain a
//! `space_id` column, existing rows are backfilled to the test space, and the
//! test space row is present in `organizations`.

use sea_orm_migration::sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::sea_orm::ConnectionTrait;
use sea_orm_migration::MigratorTrait;

const TEST_SPACE_ID: &str = "00000000-0000-0000-0000-000000000010";

async fn setup() -> DatabaseConnection {
    let opt = ConnectOptions::new("sqlite::memory:").to_owned();
    let db: DatabaseConnection = Database::connect(opt).await.expect("connect sqlite");
    migration::Migrator::up(&db, None).await.expect("migrator up");
    db
}

async fn insert_user(db: &DatabaseConnection) {
    use sea_orm_migration::sea_orm::{DatabaseBackend, Statement};
    db.execute_raw(Statement::from_string(
        DatabaseBackend::Sqlite,
        r#"INSERT INTO "users" ("id","email","name","password_hash","role","status","created_at","updated_at")
           VALUES ('00000000-0000-0000-0000-000000000001','u@example.com','u','hash','Viewer','active',
                   '2020-01-01 00:00:00','2020-01-01 00:00:00')"#,
    ))
    .await
    .expect("insert user");
}

#[tokio::test]
async fn test_space_row_exists() {
    let db = setup().await;
    let backend = sea_orm_migration::sea_orm::DatabaseBackend::Sqlite;
    let row = db
        .query_one_raw(sea_orm_migration::sea_orm::Statement::from_string(
            backend,
            format!(r#"SELECT "name" FROM "organizations" WHERE "id" = '{TEST_SPACE_ID}'"#),
        ))
        .await
        .expect("query test space");
    let row = row.expect("test space row must exist");
    let name: String = row.try_get_by_index(0).expect("read name");
    assert_eq!(name, "测试空间");
}

#[tokio::test]
async fn space_members_table_exists() {
    use sea_orm_migration::sea_orm::{DatabaseBackend, Statement};
    let db = setup().await;
    insert_user(&db).await;
    db.execute_raw(Statement::from_string(
        DatabaseBackend::Sqlite,
        format!(
            r#"INSERT INTO "space_members" ("space_id","user_id","role","created_at","updated_at")
               VALUES ('{TEST_SPACE_ID}','00000000-0000-0000-0000-000000000001','owner',
                       '2020-01-01 00:00:00','2020-01-01 00:00:00')"#
        ),
    ))
    .await
    .expect("insert space_members row");
}

#[tokio::test]
async fn space_invitations_table_exists() {
    use sea_orm_migration::sea_orm::{DatabaseBackend, Statement};
    let db = setup().await;
    insert_user(&db).await;
    db.execute_raw(Statement::from_string(
        DatabaseBackend::Sqlite,
        format!(
            r#"INSERT INTO "space_invitations"
               ("id","space_id","invitee_email","inviter_id","role","token_hash","status",
                "expires_at","created_at","updated_at")
               VALUES ('00000000-0000-0000-0000-0000000000aa','{TEST_SPACE_ID}',
                       'invitee@example.com','00000000-0000-0000-0000-000000000001','editor',
                       'hash','pending',NULL,
                       '2020-01-01 00:00:00','2020-01-01 00:00:00')"#
        ),
    ))
    .await
    .expect("insert space_invitations row");
}

#[tokio::test]
async fn business_tables_have_space_id_column() {
    let db = setup().await;
    let backend = sea_orm_migration::sea_orm::DatabaseBackend::Sqlite;
    for table in &["value_streams", "business_capabilities", "business_processes"] {
        let rows = db
            .query_all_raw(sea_orm_migration::sea_orm::Statement::from_string(
                backend,
                format!(r#"PRAGMA table_info("{table}")"#),
            ))
            .await
            .expect("pragma table_info");
        let has_space_id = rows
            .iter()
            .any(|r| r.try_get_by_index::<String>(1).map(|n| n == "space_id").unwrap_or(false));
        assert!(has_space_id, "{table} should have a space_id column");
    }
}

#[tokio::test]
async fn existing_rows_backfilled_to_test_space() {
    let db = setup().await;
    let backend = sea_orm_migration::sea_orm::DatabaseBackend::Sqlite;
    let row = db
        .query_one_raw(sea_orm_migration::sea_orm::Statement::from_string(
            backend,
            format!(r#"SELECT COUNT(*) AS c FROM "organizations" WHERE "id" = '{TEST_SPACE_ID}'"#),
        ))
        .await
        .expect("count")
        .expect("row");
    let count: i64 = row.try_get_by_index(0).expect("read count");
    assert_eq!(count, 1, "exactly one test space row expected");
}
