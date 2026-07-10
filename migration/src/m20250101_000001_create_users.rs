use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(uuid(Users::Id))
                    .col(string(Users::Email))
                    .col(string(Users::Name))
                    .col(string(Users::PasswordHash))
                    .col(string(Users::Role))
                    .col(string(Users::Status))
                    .col(timestamp_with_time_zone(Users::CreatedAt))
                    .col(timestamp_with_time_zone(Users::UpdatedAt))
                    .col(timestamp_with_time_zone_null(Users::DeletedAt))
                    .primary_key(Index::create().col(Users::Id))
                    .index(
                        Index::create()
                            .name("idx_users_email")
                            .col(Users::Email)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Email,
    Name,
    PasswordHash,
    Role,
    Status,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}
