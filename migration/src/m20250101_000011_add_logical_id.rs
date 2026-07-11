use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // SQLite: cannot add NOT NULL column without default. Use raw SQL.
        // Step 1: Add nullable column
        db.execute_unprepared(r#"ALTER TABLE "value_streams" ADD COLUMN "logical_id" uuid_text"#)
            .await?;
        db.execute_unprepared(r#"ALTER TABLE "business_capabilities" ADD COLUMN "logical_id" uuid_text"#)
            .await?;

        // Step 2: Backfill logical_id = id
        db.execute_unprepared(
            r#"UPDATE "value_streams" SET "logical_id" = "id" WHERE "logical_id" IS NULL"#,
        )
        .await?;
        db.execute_unprepared(
            r#"UPDATE "business_capabilities" SET "logical_id" = "id" WHERE "logical_id" IS NULL"#,
        )
        .await?;

        // Step 3: Recreate tables with NOT NULL constraint (SQLite way: can't alter column)
        // For SQLite, nullable is acceptable — the application enforces NOT NULL.
        // If using Postgres, we would:
        // db.execute_unprepared(r#"ALTER TABLE "value_streams" ALTER COLUMN "logical_id" SET NOT NULL"#).await?;
        // db.execute_unprepared(r#"ALTER TABLE "business_capabilities" ALTER COLUMN "logical_id" SET NOT NULL"#).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        // SQLite doesn't support DROP COLUMN before 3.35, use raw SQL
        db.execute_unprepared(r#"ALTER TABLE "value_streams" DROP COLUMN "logical_id""#)
            .await?;
        db.execute_unprepared(r#"ALTER TABLE "business_capabilities" DROP COLUMN "logical_id""#)
            .await?;
        Ok(())
    }
}
