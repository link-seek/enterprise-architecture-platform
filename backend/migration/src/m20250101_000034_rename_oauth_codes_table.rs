use sea_orm_migration::prelude::*;

/// Renames the `oauth_codes` table (created by migration 000003) to
/// `oauth_authorization_codes` to match the SeaORM entity table name.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .exec_stmt(
                Table::rename()
                    .table(OauthCodes::Table, OauthAuthorizationCodes::Table)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .exec_stmt(
                Table::rename()
                    .table(OauthAuthorizationCodes::Table, OauthCodes::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum OauthCodes {
    Table,
}

#[derive(DeriveIden)]
enum OauthAuthorizationCodes {
    Table,
}