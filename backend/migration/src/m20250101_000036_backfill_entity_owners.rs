use sea_orm_migration::prelude::*;

/// Backfills `owner_id` for the three main business-architecture tables
/// (`value_streams`, `business_capabilities`, `business_processes`). Any row
/// whose `owner_id IS NULL` is assigned the user id of the first `owner`-role
/// member of that row's space (looked up from `space_members`).
///
/// Rows whose space has no owner member are skipped (left with `owner_id NULL`)
/// and a warning is logged, so the migration never fails on spaces without an
/// owner — avoiding a deployment blocker.
///
/// This is a one-time data migration; it is idempotent (only touches NULL
/// rows) and safe to re-run.

#[derive(DeriveMigrationName)]
pub struct Migration;

const TABLES: &[&str] = &["value_streams", "business_capabilities", "business_processes"];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        for table in TABLES {
            // Backfill: for each row with owner_id IS NULL, set owner_id to
            // the first owner-role member of the row's space. Uses a
            // correlated subquery so every row resolves its own space owner.
            let sql = format!(
                r#"UPDATE "{table}"
                   SET "owner_id" = (
                       SELECT "sm"."user_id"
                       FROM "space_members" AS "sm"
                       WHERE "sm"."space_id" = "{table}"."space_id"
                         AND "sm"."role" = 'owner'
                       LIMIT 1
                   )
                   WHERE "owner_id" IS NULL"#,
            );
            db.execute_unprepared(&sql).await?;

            // Warn on rows that still have no owner (space has no owner member).
            let count_sql = format!(
                r#"SELECT COUNT(*) AS "c" FROM "{table}" WHERE "owner_id" IS NULL"#,
            );
            if let Some(row) = db.query_one_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Sqlite,
                &count_sql,
                [],
            ))
            .await?
            {
                let remaining: i64 = row.try_get_by_index(0).unwrap_or(0);
                if remaining > 0 {
                    tracing::warn!(
                        table = table,
                        remaining = remaining,
                        "backfill_entity_owners: {} rows still have owner_id NULL \
                         (space has no owner member); skipped",
                        table,
                    );
                }
            }
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // No-op: reversing the backfill would orphan content (set owners back
        // to NULL), which is unsafe. The migration is intentionally irreversible.
        Ok(())
    }
}