use sea_orm_migration::{prelude::*, schema::*};

/// Creates the `functional_modules` table.
///
/// Represents an ArchiMate Application Collaboration / TOGAF Functional
/// Decomposition. Groups application components into coarser functional
/// boundaries. Supports a self-referencing `parent_id` for nesting.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FunctionalModules::Table)
                    .if_not_exists()
                    .col(uuid(FunctionalModules::Id))
                    .col(string(FunctionalModules::Name))
                    .col(string_null(FunctionalModules::Description))
                    .col(string_null(FunctionalModules::Boundary))
                    .col(string(FunctionalModules::Status))
                    .col(uuid_null(FunctionalModules::ParentId))
                    .col(timestamp_with_time_zone(FunctionalModules::CreatedAt))
                    .col(timestamp_with_time_zone(FunctionalModules::UpdatedAt))
                    .col(timestamp_with_time_zone_null(FunctionalModules::DeletedAt))
                    .col(uuid(FunctionalModules::SpaceId))
                    .primary_key(Index::create().col(FunctionalModules::Id))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_functional_modules_space")
                            .from(FunctionalModules::Table, FunctionalModules::SpaceId)
                            .to(Spaces::Table, Spaces::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_functional_modules_space_id")
                    .table(FunctionalModules::Table)
                    .col(FunctionalModules::SpaceId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_functional_modules_parent_id")
                    .table(FunctionalModules::Table)
                    .col(FunctionalModules::ParentId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FunctionalModules::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum FunctionalModules {
    Table,
    Id,
    Name,
    Description,
    Boundary,
    Status,
    ParentId,
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