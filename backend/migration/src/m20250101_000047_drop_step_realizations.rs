use sea_orm_migration::prelude::*;

/// Drops the `step_realizations` table.
///
/// Per EAP v2.1 (Discussion #336), `StepRealization` (ProcessStep →
/// ApplicationProcessStep) should not exist: business steps and application
/// steps have no mapping relationship.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(StepRealizations::Table).to_owned())
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let _ = manager;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum StepRealizations {
    Table,
}