use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BusinessProcesses::Table)
                    .if_not_exists()
                    .col(uuid(BusinessProcesses::Id))
                    .col(uuid(BusinessProcesses::LogicalId))
                    .col(string(BusinessProcesses::BusinessVersion))
                    .col(string(BusinessProcesses::Status))
                    .col(string(BusinessProcesses::Name))
                    .col(text(BusinessProcesses::Description))
                    .col(string_null(BusinessProcesses::Sla))
                    .col(double_null(BusinessProcesses::CostPerTransaction))
                    .col(big_integer_null(BusinessProcesses::CycleTime))
                    .col(uuid_null(BusinessProcesses::OwnerId))
                    .col(uuid_null(BusinessProcesses::CreatedBy))
                    .col(uuid_null(BusinessProcesses::UpdatedBy))
                    .col(timestamp_with_time_zone(BusinessProcesses::CreatedAt))
                    .col(timestamp_with_time_zone(BusinessProcesses::UpdatedAt))
                    .col(timestamp_with_time_zone_null(BusinessProcesses::DeletedAt))
                    .primary_key(Index::create().col(BusinessProcesses::Id))
                    .index(
                        Index::create()
                            .name("idx_processes_logical_id")
                            .col(BusinessProcesses::LogicalId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(BusinessProcesses::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum BusinessProcesses {
    Table,
    Id,
    LogicalId,
    BusinessVersion,
    Status,
    Name,
    Description,
    Sla,
    CostPerTransaction,
    CycleTime,
    OwnerId,
    CreatedBy,
    UpdatedBy,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}
