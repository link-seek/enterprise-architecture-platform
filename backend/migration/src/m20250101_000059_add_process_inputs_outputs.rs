use sea_orm_migration::prelude::*;

/// Adds process-level `inputs` / `outputs` to `business_processes`.
///
/// Both columns store a JSON array backed by `StringVec` /
/// `FromJsonQueryResult` (same representation as `process_steps`
/// `required_inputs` / `produced_outputs`). `NOT NULL DEFAULT '[]'` backfills
/// existing rows with an empty array (SQLite supports ADD COLUMN with a
/// constant default) and keeps the Rust `StringVec` non-optional.
///
/// Raw SQL is used so the migration works for both SQLite and Postgres.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for sql in [
            r#"ALTER TABLE "business_processes" ADD COLUMN "inputs" TEXT NOT NULL DEFAULT '[]'"#,
            r#"ALTER TABLE "business_processes" ADD COLUMN "outputs" TEXT NOT NULL DEFAULT '[]'"#,
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for col in ["inputs", "outputs"] {
            let sql = format!(r#"ALTER TABLE "business_processes" DROP COLUMN "{col}""#);
            // SQLite supports DROP COLUMN from 3.35.0 onwards; ignore failures
            // on older engines (e.g. a column that never existed).
            let _ = db.execute_unprepared(&sql).await;
        }
        Ok(())
    }
}
