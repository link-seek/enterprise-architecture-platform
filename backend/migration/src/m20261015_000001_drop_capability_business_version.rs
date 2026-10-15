use sea_orm_migration::prelude::*;

/// Drops the dead `business_version` column from `business_capabilities`.
///
/// Rationale (discussions#752 D4, unified publish policy v1): capabilities
/// carry no version chain — they are the stable anchor that processes and
/// value streams version against. The column was write-once junk
/// (`capabilityCreate` hardcoded `"v1.0"`, never updated, never read by any
/// domain rule) and displaying it implied versioning that does not exist.
/// Lifecycle for capabilities is status-only (`Active → Archived`).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("business_capabilities"))
                    .drop_column(Alias::new("business_version"))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Alias::new("business_capabilities"))
                    .add_column(
                        ColumnDef::new(Alias::new("business_version"))
                            .string()
                            .not_null()
                            .default("v1.0"),
                    )
                    .to_owned(),
            )
            .await
    }
}
