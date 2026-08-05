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
mod m20250101_000012_create_organizations;
mod m20250101_000013_add_created_at_index_to_capabilities;
mod m20250101_000014_add_status_to_capabilities;
mod m20250101_000015_add_description_to_organizations;
mod m20250101_000016_add_pipeline_test_1784711733_to_organizations;
mod m20250101_000017_add_pipeline_test_1784712959_to_organizations;
mod m20250101_000018_add_pipeline_test_1784714989_to_organizations;
mod m20250101_000019_add_pipeline_test_1784735899_to_organizations;
mod m20250101_000020_add_pipeline_test_1784772794_to_organizations;
mod m20250101_000021_add_pipeline_test_1784793339_to_organizations;
mod m20250101_000022_add_pipeline_test_1784796635_to_organizations;
mod m20250101_000023_add_pipeline_test_1784801567_to_organizations;
mod m20250101_000024_add_pipeline_test_1784802862_to_organizations;
mod m20250101_000025_add_pipeline_test_1784864177_to_organizations;
mod m20250101_000026_add_pipeline_test_1784874460_to_organizations;
mod m20250101_000027_create_space_members;
mod m20250101_000028_create_space_invitations;
pub mod m20250101_000029_add_space_id_to_business_entities;
mod m20250101_000030_fix_organization_uuid_format;
mod m20250101_000031_add_visibility_to_organizations;
mod m20250101_000032_create_space_audit_logs;
mod m20250101_000033_add_indexes_for_auth;
mod m20250101_000034_rename_oauth_codes_table;
mod m20250101_000035_add_automation_level_to_processes;
mod m20250101_000036_add_maturity_to_processes;
mod m20250101_000037_add_metrics_to_capabilities;
mod m20250101_000038_create_application_components;
mod m20250101_000039_create_application_processes;
mod m20250101_000040_create_application_process_steps;
mod m20250101_000041_create_process_realizations;
mod m20250101_000042_create_capability_realizations;
mod m20250101_000043_create_step_realizations;
mod m20250101_000044_enrich_value_stream_stage;
mod m20250101_000045_backfill_entity_owners;

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
            Box::new(m20250101_000012_create_organizations::Migration),
            Box::new(m20250101_000013_add_created_at_index_to_capabilities::Migration),
            Box::new(m20250101_000014_add_status_to_capabilities::Migration),
            Box::new(m20250101_000015_add_description_to_organizations::Migration),
            Box::new(m20250101_000016_add_pipeline_test_1784711733_to_organizations::Migration),
            Box::new(m20250101_000017_add_pipeline_test_1784712959_to_organizations::Migration),
            Box::new(m20250101_000018_add_pipeline_test_1784714989_to_organizations::Migration),
            Box::new(m20250101_000019_add_pipeline_test_1784735899_to_organizations::Migration),
            Box::new(m20250101_000020_add_pipeline_test_1784772794_to_organizations::Migration),
            Box::new(m20250101_000021_add_pipeline_test_1784793339_to_organizations::Migration),
            Box::new(m20250101_000022_add_pipeline_test_1784796635_to_organizations::Migration),
            Box::new(m20250101_000023_add_pipeline_test_1784801567_to_organizations::Migration),
            Box::new(m20250101_000024_add_pipeline_test_1784802862_to_organizations::Migration),
            Box::new(m20250101_000025_add_pipeline_test_1784864177_to_organizations::Migration),
            Box::new(m20250101_000026_add_pipeline_test_1784874460_to_organizations::Migration),
            Box::new(m20250101_000027_create_space_members::Migration),
            Box::new(m20250101_000028_create_space_invitations::Migration),
            Box::new(m20250101_000029_add_space_id_to_business_entities::Migration),
            Box::new(m20250101_000030_fix_organization_uuid_format::Migration),
            Box::new(m20250101_000031_add_visibility_to_organizations::Migration),
            Box::new(m20250101_000032_create_space_audit_logs::Migration),
            Box::new(m20250101_000033_add_indexes_for_auth::Migration),
            Box::new(m20250101_000034_rename_oauth_codes_table::Migration),
            Box::new(m20250101_000035_add_automation_level_to_processes::Migration),
            Box::new(m20250101_000036_add_maturity_to_processes::Migration),
            Box::new(m20250101_000037_add_metrics_to_capabilities::Migration),
            Box::new(m20250101_000038_create_application_components::Migration),
            Box::new(m20250101_000039_create_application_processes::Migration),
            Box::new(m20250101_000040_create_application_process_steps::Migration),
            Box::new(m20250101_000041_create_process_realizations::Migration),
            Box::new(m20250101_000042_create_capability_realizations::Migration),
            Box::new(m20250101_000043_create_step_realizations::Migration),
            Box::new(m20250101_000044_enrich_value_stream_stage::Migration),
            Box::new(m20250101_000045_backfill_entity_owners::Migration),
        ]
    }
}
