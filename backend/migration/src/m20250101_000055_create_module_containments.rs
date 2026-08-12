use sea_orm_migration::{prelude::*, schema::*};

/// Creates the `module_containments` join table:
/// FunctionalModule → ApplicationComponent.
///
/// Maps to ArchiMate Composition (a module contains components).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ModuleContainments::Table)
                    .if_not_exists()
                    .col(uuid(ModuleContainments::FunctionalModuleId))
                    .col(uuid(ModuleContainments::ApplicationComponentId))
                    .primary_key(
                        Index::create()
                            .col(ModuleContainments::FunctionalModuleId)
                            .col(ModuleContainments::ApplicationComponentId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_module_containments_module")
                            .from(
                                ModuleContainments::Table,
                                ModuleContainments::FunctionalModuleId,
                            )
                            .to(FunctionalModules::Table, FunctionalModules::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_module_containments_component")
                            .from(
                                ModuleContainments::Table,
                                ModuleContainments::ApplicationComponentId,
                            )
                            .to(ApplicationComponents::Table, ApplicationComponents::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_module_containments_module")
                    .table(ModuleContainments::Table)
                    .col(ModuleContainments::FunctionalModuleId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_module_containments_component")
                    .table(ModuleContainments::Table)
                    .col(ModuleContainments::ApplicationComponentId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ModuleContainments::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ModuleContainments {
    Table,
    FunctionalModuleId,
    ApplicationComponentId,
}

#[derive(DeriveIden)]
enum FunctionalModules {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ApplicationComponents {
    Table,
    Id,
}