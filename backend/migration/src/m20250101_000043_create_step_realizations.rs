use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(StepRealizations::Table)
                    .if_not_exists()
                    .col(uuid(StepRealizations::ProcessStepId))
                    .col(uuid(StepRealizations::ApplicationProcessStepId))
                    .primary_key(
                        Index::create()
                            .col(StepRealizations::ProcessStepId)
                            .col(StepRealizations::ApplicationProcessStepId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_step_realizations_process_step")
                            .from(StepRealizations::Table, StepRealizations::ProcessStepId)
                            .to(ProcessSteps::Table, ProcessSteps::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_step_realizations_application_process_step")
                            .from(StepRealizations::Table, StepRealizations::ApplicationProcessStepId)
                            .to(ApplicationProcessSteps::Table, ApplicationProcessSteps::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_step_realizations_process_step")
                    .table(StepRealizations::Table)
                    .col(StepRealizations::ProcessStepId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_step_realizations_application_process_step")
                    .table(StepRealizations::Table)
                    .col(StepRealizations::ApplicationProcessStepId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(StepRealizations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum StepRealizations {
    Table,
    ProcessStepId,
    ApplicationProcessStepId,
}

#[derive(DeriveIden)]
enum ProcessSteps {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum ApplicationProcessSteps {
    Table,
    Id,
}