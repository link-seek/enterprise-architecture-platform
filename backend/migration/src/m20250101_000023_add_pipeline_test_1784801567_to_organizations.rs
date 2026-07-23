use sea_orm_migration::prelude::*;

/// Adds a nullable `pipeline_test_1784801567` column to the `organizations` table.
///
/// The column is a harmless test column. It is nullable with a default of
/// `NULL` so existing rows are backfilled without requiring a value.
///
/// Raw SQL is used so the migration works for both SQLite and Postgres:
/// SQLite cannot add a `NOT NULL` column without a default in a single
/// `ALTER TABLE`, and `sea_query`'s high-level helpers do not consistently
/// express a `DEFAULT` on `ADD COLUMN` across backends. A nullable column
/// with no default is accepted by both backends.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"ALTER TABLE "organizations" ADD COLUMN "pipeline_test_1784801567" TEXT"#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // SQLite supports DROP COLUMN from 3.35.0 onwards; Postgres supports it natively.
        db.execute_unprepared(
            r#"ALTER TABLE "organizations" DROP COLUMN "pipeline_test_1784801567""#,
        )
        .await?;
        Ok(())
    }
}