use chrono::{DateTime, Utc};
use shared_common::enums::LifecycleStatus;
use shared_common::value_objects::StringVec;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BusinessProcess {
    pub id: Uuid,
    pub logical_id: Uuid,
    pub business_version: String,
    pub status: LifecycleStatus,
    pub name: String,
    pub description: String,
    pub sla: Option<String>,
    pub cost_per_transaction: Option<f64>,
    pub cycle_time: Option<i64>,
    pub owner_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct ProcessStep {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub sequence_order: i32,
    pub business_rules: StringVec,
    pub required_inputs: StringVec,
    pub produced_outputs: StringVec,
    pub role_id: Option<Uuid>,
    pub process_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
