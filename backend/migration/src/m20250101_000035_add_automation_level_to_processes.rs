use sea_orm_migration::prelude::*;

/// Adds an `automation_level` column to `business_processes`.
///
/// The column stores the automation classification of a process
/// (`manual` / `semi_automated` / `fully_automated`) and is nullable so
/// existing rows are not required to have a value.
///
/// Raw SQL is used so the migration works for both SQLite and Postgres
/// (see migration 000014 for the same rationale).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"ALTER TABLE "business_processes" ADD COLUMN "automation_level" TEXT"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"ALTER TABLE "business_processes" DROP COLUMN "automation_level""#,
        )
        .await?;
        Ok(())
    }
}