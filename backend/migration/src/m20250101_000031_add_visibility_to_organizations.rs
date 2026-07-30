use sea_orm_migration::prelude::*;

/// Adds a `visibility` column to the `organizations` table (reused as Spaces).
///
/// Values are `public` (default) or `private`. The column is `NOT NULL DEFAULT
/// 'public'` so existing rows are backfilled to public with zero behavior
/// change — anonymous browsing of previously-open spaces is preserved.
///
/// Raw SQL is used so the migration works for both SQLite and Postgres:
/// SQLite cannot add a `NOT NULL` column without a default in a single
/// `ALTER TABLE`, and a `DEFAULT` clause on `ADD COLUMN` is the portable form.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"ALTER TABLE "organizations" ADD COLUMN "visibility" TEXT NOT NULL DEFAULT 'public'"#,
        )
        .await?;

        // Defensive backfill in case a backend accepted a NULL despite the
        // DEFAULT (older SQLite versions treat NOT NULL without default on a
        // populated table differently). No-op when already populated.
        db.execute_unprepared(
            r#"UPDATE "organizations" SET "visibility"='public' WHERE "visibility" IS NULL"#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // SQLite supports DROP COLUMN from 3.35.0 onwards; Postgres supports it natively.
        db.execute_unprepared(r#"ALTER TABLE "organizations" DROP COLUMN "visibility""#)
            .await?;
        Ok(())
    }
}