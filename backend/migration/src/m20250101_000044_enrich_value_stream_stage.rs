use sea_orm_migration::prelude::*;

/// Enriches `value_stream_stages` with the value-stream-stage meta-model:
/// `description`, `objective_metrics` (target values), `entry_criteria`,
/// `exit_criteria`, `owner_id` (business-level stage owner), `key_metrics`
/// (current/actual values).
///
/// All columns are nullable and added via raw SQL so the migration works on
/// both SQLite and Postgres (see migration 000037 for the same rationale).
/// `objective_metrics` / `key_metrics` store a JSON map backed by
/// `StringStringMap` / `FromJsonQueryResult`, mirroring
/// `value_streams.performance_metrics`.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for sql in [
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "description" TEXT"#,
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "objective_metrics" TEXT"#,
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "entry_criteria" TEXT"#,
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "exit_criteria" TEXT"#,
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "owner_id" uuid_text"#,
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "key_metrics" TEXT"#,
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for col in [
            "description",
            "objective_metrics",
            "entry_criteria",
            "exit_criteria",
            "owner_id",
            "key_metrics",
        ] {
            let sql = format!(r#"ALTER TABLE "value_stream_stages" DROP COLUMN "{col}""#);
            // SQLite supports DROP COLUMN from 3.35.0 onwards; ignore failures
            // on older engines (e.g. a column that never existed).
            let _ = db.execute_unprepared(&sql).await;
        }
        Ok(())
    }
}
