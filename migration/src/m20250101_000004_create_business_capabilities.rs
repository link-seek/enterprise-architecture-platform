use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BusinessCapabilities::Table)
                    .if_not_exists()
                    .col(uuid(BusinessCapabilities::Id))
                    .col(string(BusinessCapabilities::BusinessVersion))
                    .col(string(BusinessCapabilities::Status))
                    .col(string(BusinessCapabilities::Name))
                    .col(text(BusinessCapabilities::Description))
                    .col(string(BusinessCapabilities::Level))
                    .col(string(BusinessCapabilities::Maturity))
                    .col(string(BusinessCapabilities::BusinessValue))
                    .col(string(BusinessCapabilities::Cost))
                    .col(uuid_null(BusinessCapabilities::OwnerId))
                    .col(uuid_null(BusinessCapabilities::CreatedBy))
                    .col(uuid_null(BusinessCapabilities::UpdatedBy))
                    .col(timestamp_with_time_zone(BusinessCapabilities::CreatedAt))
                    .col(timestamp_with_time_zone(BusinessCapabilities::UpdatedAt))
                    .col(timestamp_with_time_zone_null(BusinessCapabilities::DeletedAt))
                    .primary_key(Index::create().col(BusinessCapabilities::Id))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(BusinessCapabilities::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum BusinessCapabilities {
    Table,
    Id,
    BusinessVersion,
    Status,
    Name,
    Description,
    Level,
    Maturity,
    BusinessValue,
    Cost,
    OwnerId,
    CreatedBy,
    UpdatedBy,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}
