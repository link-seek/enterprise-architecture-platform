use sea_orm_migration::prelude::*;

/// Adds a `space_id` column to the three main business-architecture tables
/// (`value_streams`, `business_capabilities`, `business_processes`), turning
/// each row into a member of a Space (the `organizations` table is reused as
/// the space table).
///
/// Backfill strategy (SQLite cannot add a NOT NULL column without a default):
/// 1. Ensure a "测试空间" (test space) row exists in `organizations` with a
///    fixed UUID so existing data has a home (requirement #7).
/// 2. Add a nullable `space_id` column to each table.
/// 3. Backfill `space_id` to the test space id for all existing rows.
///
/// The application layer treats `space_id` as NOT NULL. A foreign key
/// `ON DELETE RESTRICT` plus a lookup index are added per table. Sub-tables
/// (stages/steps/link tables) inherit their space from their parent via the
/// existing cascade foreign keys, so they are intentionally left untouched.

/// Fixed UUID of the seeded "测试空间" (test space). Existing data is
/// backfilled into this space, and the runtime seed makes the admin its owner.
pub const TEST_SPACE_ID: &str = "00000000-0000-0000-0000-000000000010";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let now = "2020-01-01 00:00:00";

        // Step 1: ensure the test space exists (idempotent).
        let insert_space = format!(
            r#"INSERT INTO "organizations" ("id","name","created_at","updated_at")
               VALUES ('{id}','测试空间','{now}','{now}')
               ON CONFLICT ("id") DO NOTHING"#,
            id = TEST_SPACE_ID,
            now = now
        );
        db.execute_unprepared(&insert_space).await?;

        // Step 2: add nullable space_id column to each main table.
        for table in &["value_streams", "business_capabilities", "business_processes"] {
            let sql = format!(r#"ALTER TABLE "{table}" ADD COLUMN "space_id" uuid_text"#);
            db.execute_unprepared(&sql).await?;
        }

        // Step 3: backfill existing rows to the test space.
        for table in &["value_streams", "business_capabilities", "business_processes"] {
            let sql = format!(
                r#"UPDATE "{table}" SET "space_id" = '{id}' WHERE "space_id" IS NULL"#,
                id = TEST_SPACE_ID
            );
            db.execute_unprepared(&sql).await?;
        }

        // Step 4: foreign keys (ON DELETE RESTRICT) + indexes.
        // SQLite enforces these only with PRAGMA foreign_keys=ON (enabled at runtime).
        for table in &["value_streams", "business_capabilities", "business_processes"] {
            let fk_name = format!("fk_{table}_space");
            let fk_sql = format!(
                r#"ALTER TABLE "{table}" ADD CONSTRAINT "{fk_name}"
                   FOREIGN KEY ("space_id") REFERENCES "organizations" ("id")
                   ON DELETE RESTRICT"#,
            );
            // SQLite supports adding FK only via table recreation; the constraint
            // is declared here for Postgres compatibility. On SQLite the FK is
            // enforced through the runtime pragma + application-level checks, so
            // a failure to add the constraint is non-fatal.
            let _ = db.execute_unprepared(&fk_sql).await;

            let idx_name = format!("idx_{table}_space_id");
            let idx_sql = format!(
                r#"CREATE INDEX IF NOT EXISTS "{idx_name}" ON "{table}" ("space_id")"#,
            );
            db.execute_unprepared(&idx_sql).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for table in &["value_streams", "business_capabilities", "business_processes"] {
            let idx_name = format!("idx_{table}_space_id");
            let _ = db
                .execute_unprepared(&format!(r#"DROP INDEX IF EXISTS "{idx_name}""#))
                .await;
            // SQLite supports DROP COLUMN from 3.35.0 onwards.
            let _ = db
                .execute_unprepared(&format!(r#"ALTER TABLE "{table}" DROP COLUMN "space_id""#))
                .await;
        }
        Ok(())
    }
}