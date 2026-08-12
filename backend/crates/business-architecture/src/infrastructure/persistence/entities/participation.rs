use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use shared_common::enums::RaciRole;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "participations")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub business_role_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub business_process_id: Uuid,
    pub raci_role: RaciRole,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}