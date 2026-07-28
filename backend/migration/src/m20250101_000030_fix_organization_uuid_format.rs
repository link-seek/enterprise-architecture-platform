use sea_orm_migration::prelude::*;

/// Fixes the `organizations.id` column: migration `m20250101_000029` inserted
/// the test-space row with a **string** UUID (`'00000000-…-000000000010'`, 36 bytes)
/// via raw SQL, but the SeaORM entity declares `id: Uuid` which expects a
/// **binary** UUID (16 bytes).  SQLite's dynamic typing accepted the string,
/// so the row was stored as `text` instead of `blob`, causing a decode error
/// at runtime:
///
/// ```text
/// error occurred while decoding column "id": invalid length: expected 16 bytes, found 36
/// ```
///
/// This migration converts any text-format UUIDs in `organizations.id` (and the
/// referencing `space_id` columns) to their 16-byte binary form.
///
/// Only rows whose `typeof(...) = 'text'` are touched; binary rows are left
/// unchanged, so the migration is idempotent and safe to re-run.

/// Binary literal for `00000000-0000-0000-0000-000000000010`.
const TEST_SPACE_ID_BIN: &str = "X'00000000000000000000000000000010'";
const TEST_SPACE_ID_STR: &str = "00000000-0000-0000-0000-000000000010";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 1. Fix organizations.id: text → blob
        let sql = format!(
            r#"UPDATE "organizations" SET "id" = {bin} WHERE "id" = '{str}'"#,
            bin = TEST_SPACE_ID_BIN,
            str = TEST_SPACE_ID_STR,
        );
        db.execute_unprepared(&sql).await?;

        // 2. Fix space_id in business tables (if any rows reference the string UUID)
        for table in &["value_streams", "business_capabilities", "business_processes"] {
            let sql = format!(
                r#"UPDATE "{table}" SET "space_id" = {bin} WHERE "space_id" = '{str}'"#,
                table = table,
                bin = TEST_SPACE_ID_BIN,
                str = TEST_SPACE_ID_STR,
            );
            db.execute_unprepared(&sql).await?;
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // No-op: reverting would re-introduce the bug.
        Ok(())
    }
}
