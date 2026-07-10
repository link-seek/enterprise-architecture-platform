use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CapabilityProcesses::Table)
                    .if_not_exists()
                    .col(uuid(CapabilityProcesses::CapabilityId))
                    .col(uuid(CapabilityProcesses::ProcessId))
                    .primary_key(
                        Index::create()
                            .col(CapabilityProcesses::CapabilityId)
                            .col(CapabilityProcesses::ProcessId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cap_proc_cap")
                            .from(CapabilityProcesses::Table, CapabilityProcesses::CapabilityId)
                            .to(BusinessCapabilities::Table, BusinessCapabilities::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_cap_proc_proc")
                            .from(CapabilityProcesses::Table, CapabilityProcesses::ProcessId)
                            .to(BusinessProcesses::Table, BusinessProcesses::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CapabilityProcesses::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CapabilityProcesses {
    Table,
    CapabilityId,
    ProcessId,
}

#[derive(DeriveIden)]
enum BusinessCapabilities {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum BusinessProcesses {
    Table,
    Id,
}
