use sea_orm_migration::prelude::*;

/// Adds a unique index on `(value_stream_id, sequence_order)` for
/// `value_stream_stages` (excluding soft-deleted rows). This enforces
/// sequence-order uniqueness at the database level, preventing TOCTOU
/// race conditions where two concurrent requests both pass the
/// application-level check and then insert duplicate rows.
///
/// The index is a partial unique index: `WHERE deleted_at IS NULL`.
/// On SQLite this is expressed via a filtered index; on PostgreSQL
/// via a partial index. We use raw SQL for cross-backend compatibility.

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let backend = db.get_database_backend();

        let sql = match backend {
            sea_orm::DatabaseBackend::Sqlite => {
                r#"CREATE UNIQUE INDEX IF NOT EXISTS "uq_vs_stage_seq_order"
                   ON "value_stream_stages" ("value_stream_id", "sequence_order")
                   WHERE "deleted_at" IS NULL"#
            }
            sea_orm::DatabaseBackend::Postgres => {
                r#"CREATE UNIQUE INDEX IF NOT EXISTS "uq_vs_stage_seq_order"
                   ON "value_stream_stages" ("value_stream_id", "sequence_order")
                   WHERE "deleted_at" IS NULL"#
            }
            sea_orm::DatabaseBackend::MySql => {
                // MySQL does not support filtered indexes. Use a regular
                // unique index — soft-deleted rows with duplicate sequence
                // orders are an accepted trade-off on MySQL.
                r#"CREATE UNIQUE INDEX IF NOT EXISTS "uq_vs_stage_seq_order"
                   ON "value_stream_stages" ("value_stream_id", "sequence_order")"#
            }
            _ => {
                r#"CREATE UNIQUE INDEX IF NOT EXISTS "uq_vs_stage_seq_order"
                   ON "value_stream_stages" ("value_stream_id", "sequence_order")
                   WHERE "deleted_at" IS NULL"#
            }
        };

        db.execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"DROP INDEX IF EXISTS "uq_vs_stage_seq_order""#,
        )
        .await?;
        Ok(())
    }
}