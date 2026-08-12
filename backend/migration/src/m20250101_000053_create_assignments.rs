use sea_orm_migration::{prelude::*, schema::*};

/// Creates the `assignments` join table: OrganizationalUnit → BusinessRole.
///
/// Maps to ArchiMate Assignment (a person/team is assigned a role).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Assignments::Table)
                    .if_not_exists()
                    .col(uuid(Assignments::OrganizationId))
                    .col(uuid(Assignments::BusinessRoleId))
                    .primary_key(
                        Index::create()
                            .col(Assignments::OrganizationId)
                            .col(Assignments::BusinessRoleId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_assignments_organization")
                            .from(Assignments::Table, Assignments::OrganizationId)
                            .to(OrganizationalUnits::Table, OrganizationalUnits::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_assignments_business_role")
                            .from(Assignments::Table, Assignments::BusinessRoleId)
                            .to(BusinessRoles::Table, BusinessRoles::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_assignments_organization")
                    .table(Assignments::Table)
                    .col(Assignments::OrganizationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_assignments_business_role")
                    .table(Assignments::Table)
                    .col(Assignments::BusinessRoleId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Assignments::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Assignments {
    Table,
    OrganizationId,
    BusinessRoleId,
}

#[derive(DeriveIden)]
enum OrganizationalUnits {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum BusinessRoles {
    Table,
    Id,
}