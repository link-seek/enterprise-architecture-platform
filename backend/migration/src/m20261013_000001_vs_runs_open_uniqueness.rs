use sea_orm_migration::prelude::*;

/// Guards the `valueStreamPublish` check-then-act against concurrent double
/// publish: at most one open (`provisioning`/`live`) run per value stream
/// version. The loser of a race fails fast on INSERT (before any repo
/// provisioning side effect) and the mutation falls back to returning the
/// winner's row.
///
/// Partial index (SQLite): terminal `archived` rows stay out of the index so
/// republish-after-failure keeps working.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // Idempotent dedupe first: legacy data may hold several open
        // (`provisioning`/`live`) rows per version, which would abort the
        // unique index below and break the whole migration chain. Keep the
        // newest live row per version (newest provisioning if no live),
        // archive the rest.
        db.execute_unprepared(
            r#"UPDATE "value_stream_runs" SET "status" = 'archived'
               WHERE "status" IN ('provisioning', 'live')
               AND "id" NOT IN (
                 SELECT "id" FROM (
                   SELECT "id", ROW_NUMBER() OVER (
                     PARTITION BY "value_stream_id"
                     ORDER BY CASE "status" WHEN 'live' THEN 0 ELSE 1 END,
                              "created_at" DESC, "id" DESC
                   ) AS "rn"
                   FROM "value_stream_runs"
                   WHERE "status" IN ('provisioning', 'live')
                 ) WHERE "rn" = 1
               )"#,
        )
        .await?;
        db.execute_unprepared(
            r#"CREATE UNIQUE INDEX IF NOT EXISTS "ux_vs_runs_open_per_version" ON "value_stream_runs" ("value_stream_id") WHERE "status" = 'provisioning' OR "status" = 'live'"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(r#"DROP INDEX IF EXISTS "ux_vs_runs_open_per_version""#)
            .await?;
        Ok(())
    }
}
