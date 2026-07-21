use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ValueStreams::Table)
                    .if_not_exists()
                    .col(uuid(ValueStreams::Id))
                    .col(string(ValueStreams::BusinessVersion))
                    .col(string(ValueStreams::Status))
                    .col(string(ValueStreams::Name))
                    .col(text_null(ValueStreams::Description))
                    .col(string_null(ValueStreams::TriggeringEvent))
                    .col(string_null(ValueStreams::EndDeliverable))
                    .col(uuid_null(ValueStreams::OwnerId))
                    .col(string(ValueStreams::Importance))
                    .col(text(ValueStreams::Stakeholders))
                    .col(text(ValueStreams::PerformanceMetrics))
                    .col(uuid_null(ValueStreams::CreatedBy))
                    .col(uuid_null(ValueStreams::UpdatedBy))
                    .col(timestamp_with_time_zone(ValueStreams::CreatedAt))
                    .col(timestamp_with_time_zone(ValueStreams::UpdatedAt))
                    .col(timestamp_with_time_zone_null(ValueStreams::DeletedAt))
                    .primary_key(Index::create().col(ValueStreams::Id))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ValueStreams::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ValueStreams {
    Table,
    Id,
    BusinessVersion,
    Status,
    Name,
    Description,
    TriggeringEvent,
    EndDeliverable,
    OwnerId,
    Importance,
    Stakeholders,
    PerformanceMetrics,
    CreatedBy,
    UpdatedBy,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}
