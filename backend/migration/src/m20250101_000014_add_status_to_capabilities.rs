use sea_orm_migration::prelude::*;

/// Adds a `capability_status` column to `business_capabilities`.
///
/// The column stores the operational state of a capability
/// (`active` / `inactive` / `draft`) and defaults to `active`.
///
/// Note: the table already has a `status` column backed by `LifecycleStatus`
/// (`active` / `archived`), so this new column is named `capability_status`
/// to avoid a conflict and to keep existing functionality unchanged.
///
/// Raw SQL is used so the migration works for both SQLite and Postgres:
/// SQLite cannot add a `NOT NULL` column without a default in a single
/// `ALTER TABLE`, and `sea_query`'s high-level helpers do not consistently
/// express a `DEFAULT` on `ADD COLUMN` across backends.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Add the column with a default so existing rows are backfilled and
        // the NOT NULL constraint can be satisfied immediately.
        db.execute_unprepared(
            r#"ALTER TABLE "business_capabilities" ADD COLUMN "capability_status" TEXT NOT NULL DEFAULT 'active'"#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // SQLite supports DROP COLUMN from 3.35.0 onwards; Postgres supports it natively.
        db.execute_unprepared(
            r#"ALTER TABLE "business_capabilities" DROP COLUMN "capability_status""#,
        )
        .await?;
        Ok(())
    }
}