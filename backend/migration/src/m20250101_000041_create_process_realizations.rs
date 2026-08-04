use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ProcessRealizations::Table)
                    .if_not_exists()
                    .col(uuid(ProcessRealizations::BusinessProcessId))
                    .col(uuid(ProcessRealizations::ApplicationProcessId))
                    .primary_key(
                        Index::create()
                            .col(ProcessRealizations::BusinessProcessId)
                            .col(ProcessRealizations::ApplicationProcessId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_process_realizations_business_process")
                            .from(ProcessRealizations::Table, ProcessRealizations::BusinessProcessId)
                            .to(BusinessProcesses::Table, BusinessProcesses::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_process_realizations_application_process")
                            .from(ProcessRealizations::Table, ProcessRealizations::ApplicationProcessId)
                            .to(ApplicationProcesses::Table, ApplicationProcesses::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_process_realizations_business_process")
                    .table(ProcessRealizations::Table)
                    .col(ProcessRealizations::BusinessProcessId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_process_realizations_application_process")
                    .table(ProcessRealizations::Table)
                    .col(ProcessRealizations::ApplicationProcessId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ProcessRealizations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ProcessRealizations {
    Table,
    BusinessProcessId,
    ApplicationProcessId,
}

#[derive(DeriveIden)]
enum BusinessProcesses {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ApplicationProcesses {
    Table,
    Id,
}