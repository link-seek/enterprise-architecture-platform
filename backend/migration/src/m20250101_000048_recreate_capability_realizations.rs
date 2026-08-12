use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::*;

/// Recreates `capability_realizations` so that a capability is enabled by a
/// *process* (business or application) instead of an application component.
///
/// SQLite cannot ALTER a composite primary key in place, so this migration
/// drops and recreates the table. The old `capability → application_component`
/// data is semantically invalid under v2.1 (a capability is now enabled by a
/// process, not a component), so it is discarded rather than migrated.
///
/// New schema:
/// - `capability_id` (FK → business_capabilities)
/// - `process_id` (polymorphic: business_processes.id or application_processes.id)
/// - `process_type` (discriminator: business_process | application_process)
/// - composite PK `(capability_id, process_id, process_type)`
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the old table (FKs and indexes cascade).
        manager
            .drop_table(Table::drop().table(CapabilityRealizations::Table).to_owned())
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CapabilityRealizations::Table)
                    .if_not_exists()
                    .col(uuid(CapabilityRealizations::CapabilityId))
                    .col(uuid(CapabilityRealizations::ProcessId))
                    .col(string(CapabilityRealizations::ProcessType))
                    .primary_key(
                        Index::create()
                            .col(CapabilityRealizations::CapabilityId)
                            .col(CapabilityRealizations::ProcessId)
                            .col(CapabilityRealizations::ProcessType),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_capability_realizations_capability")
                            .from(
                                CapabilityRealizations::Table,
                                CapabilityRealizations::CapabilityId,
                            )
                            .to(BusinessCapabilities::Table, BusinessCapabilities::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_capability_realizations_capability")
                    .table(CapabilityRealizations::Table)
                    .col(CapabilityRealizations::CapabilityId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_capability_realizations_process")
                    .table(CapabilityRealizations::Table)
                    .col(CapabilityRealizations::ProcessId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Best-effort: drop the new table. Recreating the old schema is left
        // to the original creation migration; this keeps the registry reversible.
        manager
            .drop_table(Table::drop().table(CapabilityRealizations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CapabilityRealizations {
    Table,
    CapabilityId,
    ProcessId,
    ProcessType,
}

#[derive(DeriveIden)]
enum BusinessCapabilities {
    Table,
    Id,
}