use sea_orm_migration::prelude::*;

/// Drops the `process_realizations` table.
///
/// Per EAP v2.1 (Discussion #336), `ProcessRealization` (BusinessProcess →
/// ApplicationProcess) is semantically incorrect: a business process and an
/// application process are different viewpoints, not an implementation
/// relationship. The replacement is `ProcessReference`
/// (ApplicationProcess → BusinessProcess), created in a later migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ProcessRealizations::Table).to_owned())
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Recreating the table with FKs is handled by the original creation
        // migration; a no-op here keeps the registry reversible.
        let _ = manager;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum ProcessRealizations {
    Table,
}