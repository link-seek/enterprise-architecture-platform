use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(StageCapabilities::Table)
                    .if_not_exists()
                    .col(uuid(StageCapabilities::StageId))
                    .col(uuid(StageCapabilities::CapabilityId))
                    .primary_key(
                        Index::create()
                            .col(StageCapabilities::StageId)
                            .col(StageCapabilities::CapabilityId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_stage_cap_stage")
                            .from(StageCapabilities::Table, StageCapabilities::StageId)
                            .to(ValueStreamStages::Table, ValueStreamStages::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_stage_cap_cap")
                            .from(StageCapabilities::Table, StageCapabilities::CapabilityId)
                            .to(BusinessCapabilities::Table, BusinessCapabilities::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(StageCapabilities::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum StageCapabilities {
    Table,
    StageId,
    CapabilityId,
}

#[derive(DeriveIden)]
enum ValueStreamStages {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum BusinessCapabilities {
    Table,
    Id,
}
