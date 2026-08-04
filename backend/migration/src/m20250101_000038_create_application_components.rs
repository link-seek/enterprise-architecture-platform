use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApplicationComponents::Table)
                    .if_not_exists()
                    .col(uuid(ApplicationComponents::Id))
                    .col(string(ApplicationComponents::Name))
                    .col(string(ApplicationComponents::Type))
                    .col(string(ApplicationComponents::Repo))
                    .col(string(ApplicationComponents::Path))
                    .col(string_null(ApplicationComponents::Technology))
                    .col(string(ApplicationComponents::Status))
                    .col(string(ApplicationComponents::Version))
                    .col(uuid_null(ApplicationComponents::OwnerId))
                    .col(timestamp_with_time_zone(ApplicationComponents::CreatedAt))
                    .col(timestamp_with_time_zone(ApplicationComponents::UpdatedAt))
                    .col(timestamp_with_time_zone_null(ApplicationComponents::DeletedAt))
                    .col(uuid(ApplicationComponents::SpaceId))
                    .primary_key(Index::create().col(ApplicationComponents::Id))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_application_components_space")
                            .from(ApplicationComponents::Table, ApplicationComponents::SpaceId)
                            .to(Spaces::Table, Spaces::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_application_components_space_id")
                    .table(ApplicationComponents::Table)
                    .col(ApplicationComponents::SpaceId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApplicationComponents::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ApplicationComponents {
    Table,
    Id,
    Name,
    Type,
    Repo,
    Path,
    Technology,
    Status,
    Version,
    OwnerId,
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