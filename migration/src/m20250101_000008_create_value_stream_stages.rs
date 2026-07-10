use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ValueStreamStages::Table)
                    .if_not_exists()
                    .col(uuid(ValueStreamStages::Id))
                    .col(string(ValueStreamStages::Name))
                    .col(integer(ValueStreamStages::SequenceOrder))
                    .col(string_null(ValueStreamStages::Input))
                    .col(string_null(ValueStreamStages::Output))
                    .col(uuid(ValueStreamStages::ValueStreamId))
                    .col(timestamp_with_time_zone(ValueStreamStages::CreatedAt))
                    .col(timestamp_with_time_zone(ValueStreamStages::UpdatedAt))
                    .col(timestamp_with_time_zone_null(ValueStreamStages::DeletedAt))
                    .primary_key(Index::create().col(ValueStreamStages::Id))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_vs_stages_vs")
                            .from(ValueStreamStages::Table, ValueStreamStages::ValueStreamId)
                            .to(ValueStreams::Table, ValueStreams::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ValueStreamStages::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ValueStreamStages {
    Table,
    Id,
    Name,
    SequenceOrder,
    Input,
    Output,
    ValueStreamId,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}

#[derive(DeriveIden)]
enum ValueStreams {
    Table,
    Id,
}
