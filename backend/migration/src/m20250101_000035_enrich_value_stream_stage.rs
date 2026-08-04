use sea_orm_migration::prelude::*;

/// Enriches `value_stream_stages` with six nullable columns to express the
/// full stage metamodel:
/// - `description`        — free-form stage description
/// - `objective_metrics`  — JSON text storing a `StringStringMap` (metric → target)
/// - `entry_criteria`     — entry gate criteria (free text)
/// - `exit_criteria`      — exit gate criteria (free text)
/// - `owner_id`           — stage owner (business semantics, NOT a permission gate)
/// - `key_metrics`        — JSON text storing a `StringStringMap` (metric → current)
///
/// All columns are nullable so the migration is SQLite-compatible
/// (`ALTER TABLE ADD COLUMN` without NOT NULL) and existing rows remain valid.
/// `objective_metrics` / `key_metrics` use JSON text columns mirroring the
/// existing `value_streams.performance_metrics` storage approach.

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        for stmt in [
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "description" text"#,
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "objective_metrics" text"#,
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "entry_criteria" text"#,
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "exit_criteria" text"#,
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "owner_id" uuid_text"#,
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "key_metrics" text"#,
        ] {
            // Adding a column that may already exist (re-run) is a no-op error
            // on SQLite; ignore it to keep the migration idempotent.
            let _ = db.execute_unprepared(stmt).await;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for col in &[
            "key_metrics",
            "owner_id",
            "exit_criteria",
            "entry_criteria",
            "objective_metrics",
            "description",
        ] {
            let _ = db
                .execute_unprepared(&format!(
                    r#"ALTER TABLE "value_stream_stages" DROP COLUMN "{col}""#
                ))
                .await;
        }
        Ok(())
    }
}