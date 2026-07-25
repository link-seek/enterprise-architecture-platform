use sea_orm_migration::prelude::*;

/// Enforces that a value stream cannot have two *active* (non-deleted) stages
/// with the same `sequence_order`. The partial index (`WHERE deleted_at IS NULL`)
/// keeps soft-deleted stages from blocking reuse of their order numbers while
/// still protecting the live flow that `validate_stage_flow` reasons about.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"CREATE UNIQUE INDEX IF NOT EXISTS "uq_vs_stages_vs_seq"
               ON "value_stream_stages" ("value_stream_id", "sequence_order")
               WHERE "deleted_at" IS NULL"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(r#"DROP INDEX IF EXISTS "uq_vs_stages_vs_seq""#)
            .await?;
        Ok(())
    }
}