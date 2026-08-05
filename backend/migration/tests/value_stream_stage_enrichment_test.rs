//! Verifies migration `m20250101_000044_enrich_value_stream_stage`:
//! the six new columns (`description`, `objective_metrics`,
//! `entry_criteria`, `exit_criteria`, `owner_id`, `key_metrics`) exist on
//! `value_stream_stages` and can be written and read back on SQLite.

use sea_orm_migration::sea_orm::{ConnectOptions, Database, DatabaseConnection, Statement};
use sea_orm_migration::sea_orm::ConnectionTrait;
use sea_orm_migration::MigratorTrait;

/// Binary literal matching how migration `m20250101_000029` stores the test
/// space id in SQLite (`Uuid` column type → 16-byte blob).
const TEST_SPACE_ID_BIN: &str = "X'00000000000000000000000000000010'";

async fn setup() -> DatabaseConnection {
    let opt = ConnectOptions::new("sqlite::memory:").to_owned();
    let db: DatabaseConnection = Database::connect(opt).await.expect("connect sqlite");
    migration::Migrator::up(&db, None).await.expect("migrator up");
    db
}

#[tokio::test]
async fn stage_new_columns_exist() {
    let db = setup().await;
    let backend = sea_orm_migration::sea_orm::DatabaseBackend::Sqlite;
    let rows = db
        .query_all_raw(Statement::from_string(
            backend,
            r#"PRAGMA table_info("value_stream_stages")"#.to_owned(),
        ))
        .await
        .expect("pragma table_info");
    let names: Vec<String> = rows
        .iter()
        .filter_map(|r| r.try_get_by_index::<String>(1).ok())
        .collect();
    for col in [
        "description",
        "objective_metrics",
        "entry_criteria",
        "exit_criteria",
        "owner_id",
        "key_metrics",
    ] {
        assert!(names.iter().any(|n| n == col), "missing column {col}");
    }
}

#[tokio::test]
async fn stage_new_columns_roundtrip() {
    let db = setup().await;
    let backend = sea_orm_migration::sea_orm::DatabaseBackend::Sqlite;

    // Parent value stream (stage FK points at it).
    db.execute_raw(Statement::from_string(
        backend,
        format!(
            r#"INSERT INTO "value_streams" ("id","logical_id","business_version","status","name","description","triggering_event","end_deliverable","owner_id","importance","stakeholders","performance_metrics","created_at","updated_at","space_id")
               VALUES ('00000000-0000-0000-0000-0000000000b1','00000000-0000-0000-0000-0000000000b1','1.0.0','active','StageEnrich VS',NULL,NULL,NULL,NULL,'High','[]','{{}}','2020-01-01 00:00:00','2020-01-01 00:00:00',{TEST_SPACE_ID_BIN})"#
        ),
    ))
    .await
    .expect("insert value stream");

    // Stage with all six new fields populated.
    db.execute_raw(Statement::from_string(
        backend,
        format!(
            r#"INSERT INTO "value_stream_stages" ("id","name","sequence_order","input","output","value_stream_id","created_at","updated_at","description","objective_metrics","entry_criteria","exit_criteria","owner_id","key_metrics")
               VALUES ('00000000-0000-0000-0000-0000000000b2','设计阶段',1,'in','out','00000000-0000-0000-0000-0000000000b1','2020-01-01 00:00:00','2020-01-01 00:00:00','阶段描述','{{"设计款式数":"≥20"}}','进入条件','退出条件','00000000-0000-0000-0000-0000000000b3','{{"实际款式数":"18"}}')"#
        ),
    ))
    .await
    .expect("insert stage");

    let row = db
        .query_one_raw(Statement::from_string(
            backend,
            r#"SELECT "description","objective_metrics","entry_criteria","exit_criteria","owner_id","key_metrics"
               FROM "value_stream_stages" WHERE "id" = '00000000-0000-0000-0000-0000000000b2'"#
                .to_owned(),
        ))
        .await
        .expect("query stage")
        .expect("row");

    let description: String = row.try_get_by_index(0).expect("description");
    let objective_metrics: String = row.try_get_by_index(1).expect("objective_metrics");
    let entry_criteria: String = row.try_get_by_index(2).expect("entry_criteria");
    let exit_criteria: String = row.try_get_by_index(3).expect("exit_criteria");
    let owner_id: String = row.try_get_by_index(4).expect("owner_id");
    let key_metrics: String = row.try_get_by_index(5).expect("key_metrics");

    assert_eq!(description, "阶段描述");
    assert!(objective_metrics.contains("设计款式数"), "objective_metrics = {objective_metrics}");
    assert!(objective_metrics.contains("≥20"), "objective_metrics = {objective_metrics}");
    assert_eq!(entry_criteria, "进入条件");
    assert_eq!(exit_criteria, "退出条件");
    assert_eq!(owner_id, "00000000-0000-0000-0000-0000000000b3");
    assert!(key_metrics.contains("实际款式数"), "key_metrics = {key_metrics}");
}
