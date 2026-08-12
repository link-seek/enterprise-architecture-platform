use sea_orm_migration::{prelude::*, schema::*};

/// Creates the `process_references` join table:
/// ApplicationProcess → BusinessProcess.
///
/// Maps to ArchiMate Serving: a technical orchestration references the
/// business responsibility划分 it serves. This replaces the deleted
/// `process_realizations` (BusinessProcess → ApplicationProcess), which had
/// the direction and semantics inverted.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ProcessReferences::Table)
                    .if_not_exists()
                    .col(uuid(ProcessReferences::ApplicationProcessId))
                    .col(uuid(ProcessReferences::BusinessProcessId))
                    .primary_key(
                        Index::create()
                            .col(ProcessReferences::ApplicationProcessId)
                            .col(ProcessReferences::BusinessProcessId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_process_references_application_process")
                            .from(
                                ProcessReferences::Table,
                                ProcessReferences::ApplicationProcessId,
                            )
                            .to(ApplicationProcesses::Table, ApplicationProcesses::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_process_references_business_process")
                            .from(
                                ProcessReferences::Table,
                                ProcessReferences::BusinessProcessId,
                            )
                            .to(BusinessProcesses::Table, BusinessProcesses::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_process_references_application_process")
                    .table(ProcessReferences::Table)
                    .col(ProcessReferences::ApplicationProcessId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_process_references_business_process")
                    .table(ProcessReferences::Table)
                    .col(ProcessReferences::BusinessProcessId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ProcessReferences::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ProcessReferences {
    Table,
    ApplicationProcessId,
    BusinessProcessId,
}

#[derive(DeriveIden)]
enum ApplicationProcesses {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum BusinessProcesses {
    Table,
    Id,
}