use sea_orm_migration::{prelude::*, schema::*};

/// Creates the `application_interfaces` table.
///
/// Represents an ArchiMate Application Interface / TOGAF Interface Catalog.
/// An interface is exposed by a provider module and optionally consumed by
/// another module. `input_schema` / `output_schema` store JSON contracts.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApplicationInterfaces::Table)
                    .if_not_exists()
                    .col(uuid(ApplicationInterfaces::Id))
                    .col(string(ApplicationInterfaces::Name))
                    .col(string(ApplicationInterfaces::Protocol))
                    .col(string_null(ApplicationInterfaces::Contract))
                    .col(json_null(ApplicationInterfaces::InputSchema))
                    .col(json_null(ApplicationInterfaces::OutputSchema))
                    .col(uuid(ApplicationInterfaces::ProviderModuleId))
                    .col(uuid_null(ApplicationInterfaces::ConsumerModuleId))
                    .col(timestamp_with_time_zone(ApplicationInterfaces::CreatedAt))
                    .col(timestamp_with_time_zone(ApplicationInterfaces::UpdatedAt))
                    .col(timestamp_with_time_zone_null(ApplicationInterfaces::DeletedAt))
                    .col(uuid(ApplicationInterfaces::SpaceId))
                    .primary_key(Index::create().col(ApplicationInterfaces::Id))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_application_interfaces_provider")
                            .from(
                                ApplicationInterfaces::Table,
                                ApplicationInterfaces::ProviderModuleId,
                            )
                            .to(FunctionalModules::Table, FunctionalModules::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_application_interfaces_consumer")
                            .from(
                                ApplicationInterfaces::Table,
                                ApplicationInterfaces::ConsumerModuleId,
                            )
                            .to(FunctionalModules::Table, FunctionalModules::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_application_interfaces_space")
                            .from(ApplicationInterfaces::Table, ApplicationInterfaces::SpaceId)
                            .to(Spaces::Table, Spaces::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_application_interfaces_space_id")
                    .table(ApplicationInterfaces::Table)
                    .col(ApplicationInterfaces::SpaceId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_application_interfaces_provider")
                    .table(ApplicationInterfaces::Table)
                    .col(ApplicationInterfaces::ProviderModuleId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApplicationInterfaces::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ApplicationInterfaces {
    Table,
    Id,
    Name,
    Protocol,
    Contract,
    InputSchema,
    OutputSchema,
    ProviderModuleId,
    ConsumerModuleId,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SpaceId,
}

#[derive(DeriveIden)]
enum FunctionalModules {
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