use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OauthAuthorizationCodes::Table)
                    .if_not_exists()
                    .col(uuid(OauthAuthorizationCodes::Id))
                    .col(string(OauthAuthorizationCodes::ClientId))
                    .col(uuid(OauthAuthorizationCodes::UserId))
                    .col(string(OauthAuthorizationCodes::CodeHash))
                    .col(string(OauthAuthorizationCodes::RedirectUri))
                    .col(string(OauthAuthorizationCodes::CodeChallenge))
                    .col(string(OauthAuthorizationCodes::CodeChallengeMethod))
                    .col(timestamp_with_time_zone(OauthAuthorizationCodes::ExpiresAt))
                    .col(
                        ColumnDef::new(OauthAuthorizationCodes::Used)
                            .boolean()
                            .not_null()
                            .default(false)
                            .to_owned(),
                    )
                    .col(timestamp_with_time_zone(OauthAuthorizationCodes::CreatedAt))
                    .primary_key(Index::create().col(OauthAuthorizationCodes::Id))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_oauth_codes_user")
                            .from(OauthAuthorizationCodes::Table, OauthAuthorizationCodes::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(OauthAuthorizationCodes::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum OauthAuthorizationCodes {
    Table,
    Id,
    ClientId,
    UserId,
    CodeHash,
    RedirectUri,
    CodeChallenge,
    CodeChallengeMethod,
    ExpiresAt,
    Used,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
