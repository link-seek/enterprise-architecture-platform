use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(RefreshTokens::Table)
                    .if_not_exists()
                    .col(uuid(RefreshTokens::Id))
                    .col(uuid(RefreshTokens::UserId))
                    .col(string(RefreshTokens::TokenHash))
                    .col(timestamp_with_time_zone(RefreshTokens::ExpiresAt))
                    .col(timestamp_with_time_zone_null(RefreshTokens::RevokedAt))
                    .col(timestamp_with_time_zone(RefreshTokens::CreatedAt))
                    .primary_key(Index::create().col(RefreshTokens::Id))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_refresh_tokens_user")
                            .from(RefreshTokens::Table, RefreshTokens::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(RefreshTokens::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum RefreshTokens {
    Table,
    Id,
    UserId,
    TokenHash,
    ExpiresAt,
    RevokedAt,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
