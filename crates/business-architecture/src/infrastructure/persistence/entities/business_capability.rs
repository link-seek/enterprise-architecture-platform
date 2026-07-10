use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use shared_common::enums::{
    BusinessValueRating, CapabilityLevel, CostRating, LifecycleStatus, MaturityLevel,
};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "business_capabilities")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
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

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
