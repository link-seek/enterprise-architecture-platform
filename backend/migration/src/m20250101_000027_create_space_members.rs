use sea_orm_migration::{prelude::*, schema::*};

/// Creates the `space_members` table: the many-to-many membership relation
/// between users and spaces (the `organizations` table is reused as spaces).
///
/// Composite primary key `(space_id, user_id)` enforces that a user belongs
/// to a given space at most once. `role` is `owner` or `editor`.
///
/// Foreign keys cascade on delete: removing a space or a user clears the
/// membership rows. (SQLite requires `PRAGMA foreign_keys=ON` to enforce
/// these; `AppState::new` enables it at runtime.)
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SpaceMembers::Table)
                    .if_not_exists()
                    .col(uuid(SpaceMembers::SpaceId))
                    .col(uuid(SpaceMembers::UserId))
                    .col(string(SpaceMembers::Role))
                    .col(timestamp_with_time_zone(SpaceMembers::CreatedAt))
                    .col(timestamp_with_time_zone(SpaceMembers::UpdatedAt))
                    .primary_key(
                        Index::create()
                            .col(SpaceMembers::SpaceId)
                            .col(SpaceMembers::UserId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_space_members_space")
                            .from(SpaceMembers::Table, SpaceMembers::SpaceId)
                            .to(Spaces::Table, Spaces::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_space_members_user")
                            .from(SpaceMembers::Table, SpaceMembers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_space_members_user")
                    .table(SpaceMembers::Table)
                    .col(SpaceMembers::UserId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SpaceMembers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum SpaceMembers {
    Table,
    SpaceId,
    UserId,
    Role,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
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