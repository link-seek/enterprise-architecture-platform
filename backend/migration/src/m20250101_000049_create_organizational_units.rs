use sea_orm_migration::{prelude::*, schema::*};

/// Creates the `organizational_units` table.
///
/// Represents an ArchiMate Business Actor / TOGAF Organization Unit. The
/// table name is `organizational_units` (not `organizations`, which is
/// already used by the Space entity). Supports a self-referencing `parent_id`
/// for hierarchy.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OrganizationalUnits::Table)
                    .if_not_exists()
                    .col(uuid(OrganizationalUnits::Id))
                    .col(string(OrganizationalUnits::Name))
                    .col(string(OrganizationalUnits::Type))
                    .col(uuid_null(OrganizationalUnits::ParentId))
                    .col(string_null(OrganizationalUnits::Description))
                    .col(string(OrganizationalUnits::Status))
                    .col(timestamp_with_time_zone(OrganizationalUnits::CreatedAt))
                    .col(timestamp_with_time_zone(OrganizationalUnits::UpdatedAt))
                    .col(timestamp_with_time_zone_null(OrganizationalUnits::DeletedAt))
                    .col(uuid(OrganizationalUnits::SpaceId))
                    .primary_key(Index::create().col(OrganizationalUnits::Id))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_organizational_units_space")
                            .from(OrganizationalUnits::Table, OrganizationalUnits::SpaceId)
                            .to(Spaces::Table, Spaces::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_organizational_units_space_id")
                    .table(OrganizationalUnits::Table)
                    .col(OrganizationalUnits::SpaceId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_organizational_units_parent_id")
                    .table(OrganizationalUnits::Table)
                    .col(OrganizationalUnits::ParentId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(OrganizationalUnits::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum OrganizationalUnits {
    Table,
    Id,
    Name,
    Type,
    ParentId,
    Description,
    Status,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SpaceId,
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