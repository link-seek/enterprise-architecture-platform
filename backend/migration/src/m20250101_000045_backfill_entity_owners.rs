use sea_orm::Statement;
use sea_orm_migration::prelude::*;

/// Backfills `owner_id` for the three main business-architecture tables.
///
/// Historically the application never wrote `owner_id`, so every row is
/// `NULL`. With entity-level ownership permissions in place, rows must carry
/// an owner so that write access can be enforced. For every row whose
/// `owner_id` is `NULL`, this migration assigns the first `role='owner'`
/// member of that row's space (via `space_members`). Spaces without an owner
/// member are skipped and a warning is logged, so the migration never fails
/// the deployment.
#[derive(DeriveMigrationName)]
pub struct Migration;

const TABLES: [&str; 3] = [
    "value_streams",
    "business_capabilities",
    "business_processes",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        for table in TABLES {
            let sql = format!(
                r#"UPDATE "{table}"
                   SET "owner_id" = (
                       SELECT sm."user_id" FROM "space_members" sm
                       WHERE sm."space_id" = "{table}"."space_id"
                         AND sm."role" = 'owner'
                       ORDER BY sm."user_id" LIMIT 1
                   )
                   WHERE "owner_id" IS NULL"#
            );
            db.execute_unprepared(&sql).await?;

            // Warn about rows that still have no owner (their space has no
            // owner member) — non-fatal, but operators should follow up.
            let count_sql = format!(
                r#"SELECT COUNT(*) AS cnt FROM "{table}" WHERE "owner_id" IS NULL"#
            );
            let count_stmt = Statement::from_string(db.get_database_backend(), count_sql);
            let remaining: i64 = match db.query_one_raw(count_stmt).await? {
                Some(row) => row.try_get("", "cnt").unwrap_or(0),
                None => 0,
            };
            if remaining > 0 {
                tracing::warn!(
                    "backfill_entity_owners: {remaining} row(s) in {table} still have owner_id = NULL (space has no owner member)"
                );
            }
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Backfill cannot be safely undone; leaving a no-op so the migration
        // remains reversible in the migration registry.
        let _ = manager;
        Ok(())
    }
}
