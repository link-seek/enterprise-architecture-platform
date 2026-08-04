use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApplicationProcessSteps::Table)
                    .if_not_exists()
                    .col(uuid(ApplicationProcessSteps::Id))
                    .col(string(ApplicationProcessSteps::Name))
                    .col(string(ApplicationProcessSteps::Action))
                    .col(string(ApplicationProcessSteps::Description))
                    .col(integer(ApplicationProcessSteps::SequenceOrder))
                    .col(json(ApplicationProcessSteps::Inputs))
                    .col(json(ApplicationProcessSteps::Outputs))
                    .col(json(ApplicationProcessSteps::Dependencies))
                    .col(uuid(ApplicationProcessSteps::ProcessId))
                    .col(timestamp_with_time_zone(ApplicationProcessSteps::CreatedAt))
                    .col(timestamp_with_time_zone(ApplicationProcessSteps::UpdatedAt))
                    .col(timestamp_with_time_zone_null(ApplicationProcessSteps::DeletedAt))
                    .primary_key(Index::create().col(ApplicationProcessSteps::Id))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_application_process_steps_process")
                            .from(ApplicationProcessSteps::Table, ApplicationProcessSteps::ProcessId)
                            .to(ApplicationProcesses::Table, ApplicationProcesses::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_application_process_steps_process_id")
                    .table(ApplicationProcessSteps::Table)
                    .col(ApplicationProcessSteps::ProcessId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApplicationProcessSteps::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ApplicationProcessSteps {
    Table,
    Id,
    Name,
    Action,
    Description,
    SequenceOrder,
    Inputs,
    Outputs,
    Dependencies,
    ProcessId,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
enum ApplicationProcesses {
    Table,
    Id,
}