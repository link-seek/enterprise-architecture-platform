use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "assignments")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub organization_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub business_role_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}