use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Organizations::Table)
                    .if_not_exists()
                    .col(uuid(Organizations::Id))
                    .col(string(Organizations::Name))
                    .col(timestamp_with_time_zone(Organizations::CreatedAt))
                    .col(timestamp_with_time_zone(Organizations::UpdatedAt))
                    .col(timestamp_with_time_zone_null(Organizations::DeletedAt))
                    .primary_key(Index::create().col(Organizations::Id))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Organizations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Organizations {
    Table,
    Id,
    Name,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}