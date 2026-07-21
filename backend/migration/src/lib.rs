pub use sea_orm_migration::*;

mod m20250101_000001_create_users;
mod m20250101_000002_create_refresh_tokens;
mod m20250101_000003_create_oauth_codes;
mod m20250101_000004_create_business_capabilities;
mod m20250101_000005_create_business_processes;
mod m20250101_000006_create_process_steps;
mod m20250101_000007_create_value_streams;
mod m20250101_000008_create_value_stream_stages;
mod m20250101_000009_create_capability_processes;
mod m20250101_000010_create_stage_capabilities;
mod m20250101_000011_add_logical_id;
mod m20250101_000013_add_created_at_index_to_capabilities;
mod m20250101_000014_add_status_to_capabilities;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250101_000001_create_users::Migration),
            Box::new(m20250101_000002_create_refresh_tokens::Migration),
            Box::new(m20250101_000003_create_oauth_codes::Migration),
            Box::new(m20250101_000004_create_business_capabilities::Migration),
            Box::new(m20250101_000005_create_business_processes::Migration),
            Box::new(m20250101_000006_create_process_steps::Migration),
            Box::new(m20250101_000007_create_value_streams::Migration),
            Box::new(m20250101_000008_create_value_stream_stages::Migration),
            Box::new(m20250101_000009_create_capability_processes::Migration),
            Box::new(m20250101_000010_create_stage_capabilities::Migration),
            Box::new(m20250101_000011_add_logical_id::Migration),
            Box::new(m20250101_000013_add_created_at_index_to_capabilities::Migration),
            Box::new(m20250101_000014_add_status_to_capabilities::Migration),
        ]
    }
}
