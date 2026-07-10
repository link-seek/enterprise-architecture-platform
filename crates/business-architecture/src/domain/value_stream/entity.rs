use chrono::{DateTime, Utc};
use shared_common::enums::{LifecycleStatus, ValueStreamImportance};
use shared_common::value_objects::{StringStringMap, StringVec};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ValueStream {
    pub id: Uuid,
    pub business_version: String,
    pub status: LifecycleStatus,
    pub name: String,
    pub description: String,
    pub triggering_event: Option<String>,
    pub end_deliverable: Option<String>,
    pub owner_id: Option<Uuid>,
    pub importance: ValueStreamImportance,
    pub stakeholders: StringVec,
    pub performance_metrics: StringStringMap,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct ValueStreamStage {
    pub id: Uuid,
    pub name: String,
    pub sequence_order: i32,
    pub input: Option<String>,
    pub output: Option<String>,
    pub value_stream_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
