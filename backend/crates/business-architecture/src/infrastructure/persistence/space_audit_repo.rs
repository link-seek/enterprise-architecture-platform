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
            id: Set(log.id()),
            space_id: Set(log.space_id()),
            actor_id: Set(log.actor_id()),
            action: Set(log.action().to_string()),
            from_value: Set(log.from_value().map(str::to_owned)),
            to_value: Set(log.to_value().map(str::to_owned)),
            created_at: Set(log.created_at()),
        };
        active.insert(&self.db).await?;
        Ok(())
    }

    async fn list_for_space(
        &self,
        space_id: Uuid,
        limit: Option<u64>,
        offset: u64,
    ) -> Result<Vec<SpaceAuditLog>, DomainError> {
        const DEFAULT_LIMIT: u64 = 200;
        const MAX_LIMIT: u64 = 1000;
        let max_limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        // Filter at the DB level by known action values so that pagination
        // (offset + limit) and filtering happen in the same layer. Without
        // this, rows with unmappable actions would consume pagination slots
        // but be dropped in memory, causing gaps in the result set.
        let models = space_audit_log::Entity::find()
            .filter(space_audit_log::Column::SpaceId.eq(space_id))
            .filter(space_audit_log::Column::Action.is_in(["visibility_changed"]))
            .order_by_desc(space_audit_log::Column::CreatedAt)
            .offset(offset)
            .limit(max_limit)
            .all(&self.db)
            .await?;
        Ok(models
            .into_iter()
            .map(SpaceAuditLog::try_from)
            .collect::<Result<Vec<_>, _>>()?)
    }
}

impl TryFrom<space_audit_log::Model> for SpaceAuditLog {
    type Error = DomainError;

    fn try_from(m: space_audit_log::Model) -> Result<Self, Self::Error> {
        let action = SpaceAuditAction::try_from(m.action.as_str())?;
        SpaceAuditLog::from_db_row(
            m.id,
            m.space_id,
            m.actor_id,
            action,
            m.from_value,
            m.to_value,
            m.created_at,
        )
    }
}
