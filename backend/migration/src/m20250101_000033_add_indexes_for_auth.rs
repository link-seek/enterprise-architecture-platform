use sea_orm_migration::prelude::*;

/// Adds missing indexes on `refresh_tokens(token_hash)`,
/// `refresh_tokens(user_id)`, and `oauth_codes(code_hash)`
/// to speed up token lookups and user-token listings.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Remove duplicate token_hash rows before adding a UNIQUE index. If
        // pre-existing data contains duplicates (e.g. from before the uniqueness
        // constraint existed), CREATE UNIQUE INDEX would fail and block the
        // migration. We keep the row with the smallest id (oldest) per
        // token_hash and delete the rest. SQLite supports this MIN-based
        // subquery; Postgres would use the same pattern.
        let result = db
            .execute_unprepared(
                r#"DELETE FROM "refresh_tokens" WHERE "id" NOT IN (
                       SELECT MIN("id") FROM "refresh_tokens" GROUP BY "token_hash"
                   )"#,
            )
            .await?;
        if result.rows_affected() > 0 {
            tracing::warn!(
                rows_deleted = result.rows_affected(),
                "migration 000033: deleted duplicate refresh_tokens rows to enforce \
                 unique token_hash; this is an irreversible data change."
            );
        }

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

        // Same dedup for oauth_codes.code_hash before the UNIQUE index.
        let result = db
            .execute_unprepared(
                r#"DELETE FROM "oauth_codes" WHERE "id" NOT IN (
                       SELECT MIN("id") FROM "oauth_codes" GROUP BY "code_hash"
                   )"#,
            )
            .await?;
        if result.rows_affected() > 0 {
            tracing::warn!(
                rows_deleted = result.rows_affected(),
                "migration 000033: deleted duplicate oauth_codes rows to enforce \
                 unique code_hash; this is an irreversible data change."
            );
        }

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
