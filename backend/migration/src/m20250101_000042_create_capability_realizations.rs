use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CapabilityRealizations::Table)
                    .if_not_exists()
                    .col(uuid(CapabilityRealizations::CapabilityId))
                    .col(uuid(CapabilityRealizations::ApplicationComponentId))
                    .primary_key(
                        Index::create()
                            .col(CapabilityRealizations::CapabilityId)
                            .col(CapabilityRealizations::ApplicationComponentId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_capability_realizations_capability")
                            .from(CapabilityRealizations::Table, CapabilityRealizations::CapabilityId)
                            .to(BusinessCapabilities::Table, BusinessCapabilities::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_capability_realizations_application_component")
                            .from(CapabilityRealizations::Table, CapabilityRealizations::ApplicationComponentId)
                            .to(ApplicationComponents::Table, ApplicationComponents::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_capability_realizations_capability")
                    .table(CapabilityRealizations::Table)
                    .col(CapabilityRealizations::CapabilityId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_capability_realizations_application_component")
                    .table(CapabilityRealizations::Table)
                    .col(CapabilityRealizations::ApplicationComponentId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CapabilityRealizations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum CapabilityRealizations {
    Table,
    CapabilityId,
    ApplicationComponentId,
}

#[derive(DeriveIden)]
enum BusinessCapabilities {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ApplicationComponents {
    Table,
    Id,
}