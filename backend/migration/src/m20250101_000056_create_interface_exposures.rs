use sea_orm_migration::{prelude::*, schema::*};

/// Creates the `interface_exposures` join table:
/// FunctionalModule → ApplicationInterface.
///
/// Maps to ArchiMate Assignment (a module exposes an interface).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(InterfaceExposures::Table)
                    .if_not_exists()
                    .col(uuid(InterfaceExposures::FunctionalModuleId))
                    .col(uuid(InterfaceExposures::ApplicationInterfaceId))
                    .primary_key(
                        Index::create()
                            .col(InterfaceExposures::FunctionalModuleId)
                            .col(InterfaceExposures::ApplicationInterfaceId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_interface_exposures_module")
                            .from(InterfaceExposures::Table, InterfaceExposures::FunctionalModuleId)
                            .to(FunctionalModules::Table, FunctionalModules::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_interface_exposures_interface")
                            .from(
                                InterfaceExposures::Table,
                                InterfaceExposures::ApplicationInterfaceId,
                            )
                            .to(ApplicationInterfaces::Table, ApplicationInterfaces::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_interface_exposures_module")
                    .table(InterfaceExposures::Table)
                    .col(InterfaceExposures::FunctionalModuleId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_interface_exposures_interface")
                    .table(InterfaceExposures::Table)
                    .col(InterfaceExposures::ApplicationInterfaceId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(InterfaceExposures::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum InterfaceExposures {
    Table,
    FunctionalModuleId,
    ApplicationInterfaceId,
}

#[derive(DeriveIden)]
enum FunctionalModules {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ApplicationInterfaces {
    Table,
    Id,
}