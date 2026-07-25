use sea_orm_migration::prelude::*;

/// Enriches the value-stream metamodel so a stage can express *what slice of
/// value creation it owns* (design / production / sales / delivery / custom),
/// who owns it, what it aims to achieve, and how it is measured — and lets a
/// value stream state its value proposition.
///
/// New columns on `value_stream_stages`:
/// - `description`   — what the stage focuses on (e.g. "design suitable sweaters")
/// - `stage_type`    — `StageType` enum (default `custom`)
/// - `status`        — `StageStatus` enum (default `active`)
/// - `owner_id`      — stage owner (nullable uuid)
/// - `objectives`    — JSON array of objective strings
/// - `metrics`       — JSON map of KPI name → value
///
/// New column on `value_streams`:
/// - `value_proposition` — the value being monetised (e.g. "sell sweaters")
///
/// `space_id` already binds a value stream to an organisation (see migration
/// `m20250101_000029`), so no separate `organization_id` is added here.
///
/// All new columns are nullable / have defaults so existing rows stay valid
/// (SQLite cannot add a NOT NULL column without a default).

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // --- value_stream_stages enrichment ---
        db.execute_unprepared(
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "description" text"#,
        )
        .await?;
        db.execute_unprepared(
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "stage_type" varchar(20) DEFAULT 'custom'"#,
        )
        .await?;
        db.execute_unprepared(
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "status" varchar(20) DEFAULT 'active'"#,
        )
        .await?;
        db.execute_unprepared(
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "owner_id" uuid_text"#,
        )
        .await?;
        db.execute_unprepared(
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "objectives" text"#,
        )
        .await?;
        db.execute_unprepared(
            r#"ALTER TABLE "value_stream_stages" ADD COLUMN "metrics" text"#,
        )
        .await?;

        // Backfill JSON columns to empty containers so reads never see NULL.
        db.execute_unprepared(
            r#"UPDATE "value_stream_stages" SET "objectives" = '[]' WHERE "objectives" IS NULL"#,
        )
        .await?;
        db.execute_unprepared(
            r#"UPDATE "value_stream_stages" SET "metrics" = '{}' WHERE "metrics" IS NULL"#,
        )
        .await?;

        // --- value_streams enrichment ---
        db.execute_unprepared(
            r#"ALTER TABLE "value_streams" ADD COLUMN "value_proposition" text"#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // SQLite supports DROP COLUMN from 3.35.0 onwards.
        for col in &[
            "objectives",
            "metrics",
            "owner_id",
            "status",
            "stage_type",
            "description",
        ] {
            let _ = db
                .execute_unprepared(&format!(
                    r#"ALTER TABLE "value_stream_stages" DROP COLUMN "{col}""#
                ))
                .await;
        }
        let _ = db
            .execute_unprepared(r#"ALTER TABLE "value_streams" DROP COLUMN "value_proposition""#)
            .await;
        Ok(())
    }
}