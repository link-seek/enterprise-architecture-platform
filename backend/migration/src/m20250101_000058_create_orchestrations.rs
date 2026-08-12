use sea_orm_migration::{prelude::*, schema::*};

/// Creates the `orchestrations` join table:
/// ApplicationProcess → FunctionalModule.
///
/// Maps to ArchiMate Triggering: an application process orchestrates
/// collaboration between functional modules.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Orchestrations::Table)
                    .if_not_exists()
                    .col(uuid(Orchestrations::ApplicationProcessId))
                    .col(uuid(Orchestrations::FunctionalModuleId))
                    .primary_key(
                        Index::create()
                            .col(Orchestrations::ApplicationProcessId)
                            .col(Orchestrations::FunctionalModuleId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_orchestrations_application_process")
                            .from(Orchestrations::Table, Orchestrations::ApplicationProcessId)
                            .to(ApplicationProcesses::Table, ApplicationProcesses::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_orchestrations_functional_module")
                            .from(Orchestrations::Table, Orchestrations::FunctionalModuleId)
                            .to(FunctionalModules::Table, FunctionalModules::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_orchestrations_application_process")
                    .table(Orchestrations::Table)
                    .col(Orchestrations::ApplicationProcessId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_orchestrations_functional_module")
                    .table(Orchestrations::Table)
                    .col(Orchestrations::FunctionalModuleId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Orchestrations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Orchestrations {
    Table,
    ApplicationProcessId,
    FunctionalModuleId,
}

#[derive(DeriveIden)]
enum ApplicationProcesses {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum FunctionalModules {
    Table,
    Id,
}