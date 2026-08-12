use sea_orm_migration::{prelude::*, schema::*};

/// Creates the `participations` join table: BusinessRole → BusinessProcess.
///
/// Maps to ArchiMate Assignment. Carries a `raci_role` discriminator
/// (responsible / accountable / consulted / informed) implementing the RACI
/// matrix.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Participations::Table)
                    .if_not_exists()
                    .col(uuid(Participations::BusinessRoleId))
                    .col(uuid(Participations::BusinessProcessId))
                    .col(string(Participations::RaciRole))
                    .primary_key(
                        Index::create()
                            .col(Participations::BusinessRoleId)
                            .col(Participations::BusinessProcessId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_participations_business_role")
                            .from(Participations::Table, Participations::BusinessRoleId)
                            .to(BusinessRoles::Table, BusinessRoles::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_participations_business_process")
                            .from(Participations::Table, Participations::BusinessProcessId)
                            .to(BusinessProcesses::Table, BusinessProcesses::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_participations_business_role")
                    .table(Participations::Table)
                    .col(Participations::BusinessRoleId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_participations_business_process")
                    .table(Participations::Table)
                    .col(Participations::BusinessProcessId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Participations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Participations {
    Table,
    BusinessRoleId,
    BusinessProcessId,
    RaciRole,
}

#[derive(DeriveIden)]
enum BusinessRoles {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum BusinessProcesses {
    Table,
    Id,
}