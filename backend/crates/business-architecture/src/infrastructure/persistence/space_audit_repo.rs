use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::space::audit::{SpaceAuditAction, SpaceAuditLog};
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
            action: Set(log.action.to_string()),
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
//
// Results are ordered by `created_at` descending (most recent first) and
// capped at `limit` (default 200) to bound memory usage for spaces with large
// audit histories. Callers may pass an offset for pagination.
#[allow(dead_code)]
pub async fn list_for_space(
    db: &DatabaseConnection,
    space_id: Uuid,
    limit: Option<u64>,
    offset: u64,
) -> Result<Vec<SpaceAuditLog>, DomainError> {
    let cap = limit.unwrap_or(200).min(1000);
    let models = space_audit_log::Entity::find()
        .filter(space_audit_log::Column::SpaceId.eq(space_id))
        .order_by_desc(space_audit_log::Column::CreatedAt)
        .offset(offset)
        .limit(cap)
        .all(db)
        .await?;
    Ok(models
        .into_iter()
        .filter_map(|m| match SpaceAuditLog::try_from(m) {
            Ok(log) => Some(log),
            Err(e) => {
                tracing::warn!(error = %e, "skipping audit log with unmappable action");
                None
            }
        })
        .collect())
}

impl TryFrom<space_audit_log::Model> for SpaceAuditLog {
    type Error = DomainError;

    fn try_from(m: space_audit_log::Model) -> Result<Self, Self::Error> {
        Ok(SpaceAuditLog {
            id: m.id,
            space_id: m.space_id,
            actor_id: m.actor_id,
            action: SpaceAuditAction::try_from(m.action.as_str())?,
            from_value: m.from_value,
            to_value: m.to_value,
            created_at: m.created_at,
        })
    }
}
