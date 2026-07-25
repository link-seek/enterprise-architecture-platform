use sea_orm_migration::{prelude::*, schema::*};

/// Creates the `space_invitations` table, supporting the "users can only join
/// a space by being invited" requirement.
///
/// An invitation is created by a space owner (or admin) and addressed to an
/// invitee by email. The `token_hash` stores a hash of the one-time acceptance
/// token; `status` tracks `pending`/`accepted`/`expired`; `expires_at` caps
/// the invitation lifetime.
///
/// P0 uses the simpler "owner adds member directly" path; this table is
/// created upfront so the invitation flow can be layered on without a later
/// schema migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SpaceInvitations::Table)
                    .if_not_exists()
                    .col(uuid(SpaceInvitations::Id))
                    .col(uuid(SpaceInvitations::SpaceId))
                    .col(string(SpaceInvitations::InviteeEmail))
                    .col(uuid(SpaceInvitations::InviterId))
                    .col(string(SpaceInvitations::Role))
                    .col(string(SpaceInvitations::TokenHash))
                    .col(string(SpaceInvitations::Status))
                    .col(timestamp_with_time_zone_null(SpaceInvitations::ExpiresAt))
                    .col(timestamp_with_time_zone(SpaceInvitations::CreatedAt))
                    .col(timestamp_with_time_zone(SpaceInvitations::UpdatedAt))
                    .primary_key(Index::create().col(SpaceInvitations::Id))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_space_invitations_space")
                            .from(SpaceInvitations::Table, SpaceInvitations::SpaceId)
                            .to(Spaces::Table, Spaces::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_space_invitations_space")
                    .table(SpaceInvitations::Table)
                    .col(SpaceInvitations::SpaceId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SpaceInvitations::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum SpaceInvitations {
    Table,
    Id,
    SpaceId,
    InviteeEmail,
    InviterId,
    Role,
    TokenHash,
    Status,
    ExpiresAt,
    CreatedAt,
    UpdatedAt,
}

// `Spaces` reuses the existing `organizations` table; map the Iden manually so
// the foreign key targets the real table name rather than "spaces".
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