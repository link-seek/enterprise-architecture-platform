use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use shared_common::enums::{StageStatus, StageType};
use shared_common::value_objects::{StringStringMap, StringVec};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "value_stream_stages")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub sequence_order: i32,
    pub input: Option<String>,
    pub output: Option<String>,
    pub value_stream_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub description: Option<String>,
    pub stage_type: StageType,
    pub status: StageStatus,
    pub owner_id: Option<Uuid>,
    pub objectives: StringVec,
    pub metrics: StringStringMap,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
