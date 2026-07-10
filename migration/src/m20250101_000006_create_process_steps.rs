use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ProcessSteps::Table)
                    .if_not_exists()
                    .col(uuid(ProcessSteps::Id))
                    .col(string(ProcessSteps::Name))
                    .col(text(ProcessSteps::Description))
                    .col(integer(ProcessSteps::SequenceOrder))
                    .col(text(ProcessSteps::BusinessRules))
                    .col(text(ProcessSteps::RequiredInputs))
                    .col(text(ProcessSteps::ProducedOutputs))
                    .col(uuid_null(ProcessSteps::RoleId))
                    .col(uuid(ProcessSteps::ProcessId))
                    .col(timestamp_with_time_zone(ProcessSteps::CreatedAt))
                    .col(timestamp_with_time_zone(ProcessSteps::UpdatedAt))
                    .col(timestamp_with_time_zone_null(ProcessSteps::DeletedAt))
                    .primary_key(Index::create().col(ProcessSteps::Id))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_process_steps_process")
                            .from(ProcessSteps::Table, ProcessSteps::ProcessId)
                            .to(BusinessProcesses::Table, BusinessProcesses::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ProcessSteps::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ProcessSteps {
    Table,
    Id,
    Name,
    Description,
    SequenceOrder,
    BusinessRules,
    RequiredInputs,
    ProducedOutputs,
    RoleId,
    ProcessId,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
enum BusinessProcesses {
    Table,
    Id,
}
