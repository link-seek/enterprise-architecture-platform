use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(OauthCodes::Table)
                    .if_not_exists()
                    .col(uuid(OauthCodes::Id))
                    .col(string(OauthCodes::ClientId))
                    .col(uuid(OauthCodes::UserId))
                    .col(string(OauthCodes::CodeHash))
                    .col(string(OauthCodes::RedirectUri))
                    .col(string(OauthCodes::CodeChallenge))
                    .col(string(OauthCodes::CodeChallengeMethod))
                    .col(timestamp_with_time_zone(OauthCodes::ExpiresAt))
                    .col(
                        ColumnDef::new(OauthCodes::Used)
                            .boolean()
                            .not_null()
                            .default(false)
                            .to_owned(),
                    )
                    .col(timestamp_with_time_zone(OauthCodes::CreatedAt))
                    .primary_key(Index::create().col(OauthCodes::Id))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_oauth_codes_user")
                            .from(OauthCodes::Table, OauthCodes::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(OauthCodes::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum OauthCodes {
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
