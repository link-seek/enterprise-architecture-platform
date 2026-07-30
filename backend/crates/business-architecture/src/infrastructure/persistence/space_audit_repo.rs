use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::space::audit::SpaceAuditLog;
use crate::domain::space::repository::AuditLogRepository;
use crate::infrastructure::persistence::entities::space_audit_log;

pub struct SeaOrmAuditLogRepo {
    db: DatabaseConnection,
}

impl SeaOrmAuditLogRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AuditLogRepository for SeaOrmAuditLogRepo {
    async fn record(&self, log: &SpaceAuditLog) -> Result<(), DomainError> {
        let active = space_audit_log::ActiveModel {
            id: Set(log.id),
            space_id: Set(log.space_id),
            actor_id: Set(log.actor_id),
            action: Set(log.action.clone()),
            from_value: Set(log.from_value.clone()),
            to_value: Set(log.to_value.clone()),
            created_at: Set(log.created_at),
        };
        active.insert(&self.db).await?;
        Ok(())
    }
}

// Re-exported for callers that need to query audit history directly (e.g. an
// admin audit view). Not currently exposed via GraphQL.
#[allow(dead_code)]
pub async fn list_for_space(
    db: &DatabaseConnection,
    space_id: Uuid,
) -> Result<Vec<SpaceAuditLog>, DomainError> {
    let models = space_audit_log::Entity::find()
        .filter(space_audit_log::Column::SpaceId.eq(space_id))
        .all(db)
        .await?;
    Ok(models.into_iter().map(Into::into).collect())
}

impl From<space_audit_log::Model> for SpaceAuditLog {
    fn from(m: space_audit_log::Model) -> Self {
        SpaceAuditLog {
            id: m.id,
            space_id: m.space_id,
            actor_id: m.actor_id,
            action: m.action,
            from_value: m.from_value,
            to_value: m.to_value,
            created_at: m.created_at,
        }
    }
}