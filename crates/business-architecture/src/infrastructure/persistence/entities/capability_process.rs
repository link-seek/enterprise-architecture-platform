use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "business_capability_processes")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub capability_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub process_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
