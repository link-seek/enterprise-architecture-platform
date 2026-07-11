use chrono::{DateTime, Utc};
use shared_common::enums::{
    BusinessValueRating, CapabilityLevel, CostRating, LifecycleStatus, MaturityLevel,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BusinessCapability {
    pub id: Uuid,
    pub logical_id: Uuid,
    pub business_version: String,
    pub status: LifecycleStatus,
    pub name: String,
    pub description: String,
    pub level: CapabilityLevel,
    pub maturity: MaturityLevel,
    pub business_value: BusinessValueRating,
    pub cost: CostRating,
    pub owner_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
