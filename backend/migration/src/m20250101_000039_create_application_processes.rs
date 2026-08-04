use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApplicationProcesses::Table)
                    .if_not_exists()
                    .col(uuid(ApplicationProcesses::Id))
                    .col(string(ApplicationProcesses::Name))
                    .col(string(ApplicationProcesses::Description))
                    .col(string(ApplicationProcesses::Trigger))
                    .col(json(ApplicationProcesses::Inputs))
                    .col(json(ApplicationProcesses::Outputs))
                    .col(integer_null(ApplicationProcesses::Timeout))
                    .col(integer_null(ApplicationProcesses::Retry))
                    .col(string(ApplicationProcesses::Status))
                    .col(uuid(ApplicationProcesses::LogicalId))
                    .col(string(ApplicationProcesses::BusinessVersion))
                    .col(timestamp_with_time_zone(ApplicationProcesses::CreatedAt))
                    .col(timestamp_with_time_zone(ApplicationProcesses::UpdatedAt))
                    .col(timestamp_with_time_zone_null(ApplicationProcesses::DeletedAt))
                    .col(uuid(ApplicationProcesses::SpaceId))
                    .primary_key(Index::create().col(ApplicationProcesses::Id))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_application_processes_space")
                            .from(ApplicationProcesses::Table, ApplicationProcesses::SpaceId)
                            .to(Spaces::Table, Spaces::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_application_processes_space_id")
                    .table(ApplicationProcesses::Table)
                    .col(ApplicationProcesses::SpaceId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_application_processes_logical_id")
                    .table(ApplicationProcesses::Table)
                    .col(ApplicationProcesses::LogicalId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApplicationProcesses::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ApplicationProcesses {
    Table,
    Id,
    Name,
    Description,
    Trigger,
    Inputs,
    Outputs,
    Timeout,
    Retry,
    Status,
    LogicalId,
    BusinessVersion,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SpaceId,
}

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