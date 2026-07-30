use sea_orm_migration::{prelude::*, schema::*};

/// Creates the `space_audit_logs` table recording auditable space-level
/// operations. The initial scope is visibility changes (`action =
/// 'visibility_changed'` with `from_value`/`to_value`), recorded by
/// `spaceSetVisibility`. The schema is intentionally generic (`action`,
/// `from_value`, `to_value`) so future auditable operations can reuse it
/// without a migration.
///
/// Foreign key to `organizations` cascades on delete: removing a space clears
/// its audit history. `actor_id` is intentionally not a foreign key so audit
/// records survive user deletion (an actor that no longer exists is still
/// attributable by id).
///
/// Note: the application only ever soft-deletes spaces (`archive_space`), so
/// this cascade is not exercised in normal operation. A *hard* delete of an
/// `organizations` row (e.g. by a DBA or a data-migration script) will,
/// however, silently cascade-delete its audit logs. Anyone performing such a
/// hard delete must explicitly back up or export the audit history first.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SpaceAuditLogs::Table)
                    .if_not_exists()
                    .col(uuid(SpaceAuditLogs::Id))
                    .col(uuid(SpaceAuditLogs::SpaceId))
                    .col(uuid(SpaceAuditLogs::ActorId))
                    .col(string(SpaceAuditLogs::Action))
                    .col(string_null(SpaceAuditLogs::FromValue))
                    .col(string_null(SpaceAuditLogs::ToValue))
                    .col(timestamp_with_time_zone(SpaceAuditLogs::CreatedAt))
                    .primary_key(
                        Index::create().col(SpaceAuditLogs::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_space_audit_logs_space")
                            .from(SpaceAuditLogs::Table, SpaceAuditLogs::SpaceId)
                            .to(Spaces::Table, Spaces::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_space_audit_logs_space_created_at")
                    .table(SpaceAuditLogs::Table)
                    .col(SpaceAuditLogs::SpaceId)
                    .col(SpaceAuditLogs::CreatedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SpaceAuditLogs::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum SpaceAuditLogs {
    Table,
    Id,
    SpaceId,
    ActorId,
    Action,
    FromValue,
    ToValue,
    CreatedAt,
}

// `Spaces` reuses the existing `organizations` table; map the Iden manually so
// the foreign key targets the real table name rather than "spaces".
#[derive(Copy, Clone, Debug)]
enum Spaces {
    Table,
    Id,
}

impl sea_orm_migration::sea_orm::Iden for Spaces {
    fn unquoted(&self) -> &str {
        match self {
            Spaces::Table => "organizations",
            Spaces::Id => "id",
        }
    }
}
