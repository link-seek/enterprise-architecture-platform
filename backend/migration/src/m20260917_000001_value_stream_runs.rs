use sea_orm_migration::{prelude::*, schema::*};

/// Creates the `value_stream_runs` table (pilot Task 5).
///
/// A run is an execution instance of a value stream definition:
/// `provisioning → live`, with `defined` as the initial placeholder and
/// `archived` as the terminal state. The `status` column is a plain string
/// holding one of `defined / provisioning / live / archived` — it
/// intentionally does NOT reuse `LifecycleStatus` (definition lifecycle vs.
/// run operational state are separate concerns).
///
/// `pinned_versions` stores a JSON object (`{ workflow: tag }`, default `'{}'`)
/// backed by `StringStringMap` / `FromJsonQueryResult`, same representation
/// as `value_streams.performance_metrics`. `snapshot_json` is an optional
/// free-form JSON snapshot (TEXT, NULL allowed).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ValueStreamRuns::Table)
                    .if_not_exists()
                    .col(uuid(ValueStreamRuns::Id))
                    .col(uuid(ValueStreamRuns::ValueStreamId))
                    .col(string(ValueStreamRuns::RepoUrl))
                    .col(string(ValueStreamRuns::ChainPath))
                    .col(string(ValueStreamRuns::Status))
                    .col(text(ValueStreamRuns::PinnedVersions))
                    .col(text_null(ValueStreamRuns::SnapshotJson))
                    .col(timestamp_with_time_zone(ValueStreamRuns::CreatedAt))
                    .col(timestamp_with_time_zone(ValueStreamRuns::UpdatedAt))
                    .primary_key(Index::create().col(ValueStreamRuns::Id))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_vs_runs_vs")
                            .from(ValueStreamRuns::Table, ValueStreamRuns::ValueStreamId)
                            .to(ValueStreams::Table, ValueStreams::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_vs_runs_value_stream_id")
                    .table(ValueStreamRuns::Table)
                    .col(ValueStreamRuns::ValueStreamId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_vs_runs_status")
                    .table(ValueStreamRuns::Table)
                    .col(ValueStreamRuns::Status)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ValueStreamRuns::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ValueStreamRuns {
    Table,
    Id,
    ValueStreamId,
    RepoUrl,
    ChainPath,
    Status,
    PinnedVersions,
    SnapshotJson,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum ValueStreams {
    Table,
    Id,
}
