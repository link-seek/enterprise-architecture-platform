use sea_orm_migration::{prelude::*, schema::*};

/// Creates the `business_roles` table.
///
/// Represents an ArchiMate Business Role / TOGAF Actor Role. Each role belongs
/// to an organizational unit and carries a textual list of responsibilities.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BusinessRoles::Table)
                    .if_not_exists()
                    .col(uuid(BusinessRoles::Id))
                    .col(string(BusinessRoles::Name))
                    .col(string_null(BusinessRoles::Responsibilities))
                    .col(uuid(BusinessRoles::OrganizationId))
                    .col(timestamp_with_time_zone(BusinessRoles::CreatedAt))
                    .col(timestamp_with_time_zone(BusinessRoles::UpdatedAt))
                    .col(timestamp_with_time_zone_null(BusinessRoles::DeletedAt))
                    .col(uuid(BusinessRoles::SpaceId))
                    .primary_key(Index::create().col(BusinessRoles::Id))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_business_roles_organization")
                            .from(BusinessRoles::Table, BusinessRoles::OrganizationId)
                            .to(OrganizationalUnits::Table, OrganizationalUnits::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_business_roles_space")
                            .from(BusinessRoles::Table, BusinessRoles::SpaceId)
                            .to(Spaces::Table, Spaces::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_business_roles_space_id")
                    .table(BusinessRoles::Table)
                    .col(BusinessRoles::SpaceId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_business_roles_organization_id")
                    .table(BusinessRoles::Table)
                    .col(BusinessRoles::OrganizationId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(BusinessRoles::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum BusinessRoles {
    Table,
    Id,
    Name,
    Responsibilities,
    OrganizationId,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SpaceId,
}

#[derive(DeriveIden)]
enum OrganizationalUnits {
    Table,
    Id,
}

#[derive(Copy, Clone, Debug)]
enum Spaces {
    Table,
    Id,
}

impl sea_orm_migration::sea_orm::Iden for Spaces {
    fn unquoted(&self) -> &str {
        match self {
            Spaces::Table => "organizations",
            Spaces::Id => "id",
        }
    }
}