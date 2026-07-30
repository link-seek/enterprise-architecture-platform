use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use uuid::Uuid;

/// Audit log row for space-level operations. See migration
/// `m20250101_000032_create_space_audit_logs`. `actor_id` is intentionally not
/// a foreign key so records survive user deletion.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "space_audit_logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub space_id: Uuid,
    pub actor_id: Uuid,
    pub action: String,
    pub from_value: Option<String>,
    pub to_value: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
