use sea_orm_migration::prelude::*;

/// Adds missing indexes on `refresh_tokens(token_hash)`,
/// `refresh_tokens(user_id)`, and `oauth_codes(code_hash)`
/// to speed up token lookups and user-token listings.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                Index::create()
                    .name("idx_refresh_tokens_token_hash")
                    .table(RefreshTokens::Table)
                    .col(RefreshTokens::TokenHash)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_refresh_tokens_user_id")
                    .table(RefreshTokens::Table)
                    .col(RefreshTokens::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_oauth_codes_code_hash")
                    .table(OauthCodes::Table)
                    .col(OauthCodes::CodeHash)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_oauth_codes_code_hash")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_refresh_tokens_user_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_refresh_tokens_token_hash")
                    .to_owned(),
            )
            .await
    }
}

#[derive(Copy, Clone, Debug)]
enum RefreshTokens {
    Table,
    TokenHash,
    UserId,
}

impl sea_orm_migration::sea_orm::Iden for RefreshTokens {
    fn unquoted(&self) -> &str {
        match self {
            RefreshTokens::Table => "refresh_tokens",
            RefreshTokens::TokenHash => "token_hash",
            RefreshTokens::UserId => "user_id",
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum OauthCodes {
    Table,
    CodeHash,
}

impl sea_orm_migration::sea_orm::Iden for OauthCodes {
    fn unquoted(&self) -> &str {
        match self {
            OauthCodes::Table => "oauth_codes",
            OauthCodes::CodeHash => "code_hash",
        }
    }
}
