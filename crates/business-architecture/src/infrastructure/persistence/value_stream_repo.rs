use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set, QueryOrder,
};
use sea_orm::Order;
use shared_common::enums::LifecycleStatus;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::value_stream::entity::{ValueStream, ValueStreamStage};
use crate::domain::value_stream::repository::{ValueStreamRepository, ValueStreamStageRepository};
use crate::infrastructure::persistence::entities::{stage_capability, value_stream, value_stream_stage};

impl From<value_stream::Model> for ValueStream {
    fn from(m: value_stream::Model) -> Self {
        ValueStream {
            id: m.id,
            logical_id: m.logical_id,
            business_version: m.business_version,
            status: m.status,
            name: m.name,
            description: m.description,
            triggering_event: m.triggering_event,
            end_deliverable: m.end_deliverable,
            owner_id: m.owner_id,
            importance: m.importance,
            stakeholders: m.stakeholders,
            performance_metrics: m.performance_metrics,
            created_by: m.created_by,
            updated_by: m.updated_by,
            created_at: m.created_at,
            updated_at: m.updated_at,
            deleted_at: m.deleted_at,
        }
    }
}

impl From<value_stream_stage::Model> for ValueStreamStage {
    fn from(m: value_stream_stage::Model) -> Self {
        ValueStreamStage {
            id: m.id,
            name: m.name,
            sequence_order: m.sequence_order,
            input: m.input,
            output: m.output,
            value_stream_id: m.value_stream_id,
            created_at: m.created_at,
            updated_at: m.updated_at,
            deleted_at: m.deleted_at,
        }
    }
}

pub struct SeaOrmValueStreamRepo {
    db: DatabaseConnection,
}

impl SeaOrmValueStreamRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ValueStreamRepository for SeaOrmValueStreamRepo {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<ValueStream>, DomainError> {
        let model = value_stream::Entity::find()
            .filter(value_stream::Column::Id.eq(id))
            .filter(value_stream::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;
        Ok(model.map(Into::into))
    }

    async fn find_active_by_logical_id(
        &self,
        logical_id: Uuid,
    ) -> Result<Option<ValueStream>, DomainError> {
        let model = value_stream::Entity::find()
            .filter(value_stream::Column::LogicalId.eq(logical_id))
            .filter(value_stream::Column::Status.eq(LifecycleStatus::Active))
            .filter(value_stream::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;
        Ok(model.map(Into::into))
    }

    async fn find_all_versions(
        &self,
        logical_id: Uuid,
    ) -> Result<Vec<ValueStream>, DomainError> {
        let models = value_stream::Entity::find()
            .filter(value_stream::Column::LogicalId.eq(logical_id))
            .filter(value_stream::Column::DeletedAt.is_null())
            .order_by_desc(value_stream::Column::CreatedAt)
            .all(&self.db)
            .await?;
        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn archive(&self, id: Uuid) -> Result<(), DomainError> {
        let model = value_stream::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(DomainError::ValueStreamNotFound)?;

        let mut active: value_stream::ActiveModel = model.into();
        active.status = Set(LifecycleStatus::Archived);
        active.updated_at = Set(chrono::Utc::now());
        active.update(&self.db).await?;
        Ok(())
    }

    async fn save(&self, vs: &ValueStream) -> Result<ValueStream, DomainError> {
        let existing = value_stream::Entity::find_by_id(vs.id)
            .one(&self.db)
            .await?;

        let result = if let Some(model) = existing {
            let mut active: value_stream::ActiveModel = model.into();
            active.business_version = Set(vs.business_version.clone());
            active.status = Set(vs.status);
            active.name = Set(vs.name.clone());
            active.description = Set(vs.description.clone());
            active.triggering_event = Set(vs.triggering_event.clone());
            active.end_deliverable = Set(vs.end_deliverable.clone());
            active.owner_id = Set(vs.owner_id);
            active.importance = Set(vs.importance);
            active.stakeholders = Set(vs.stakeholders.clone());
            active.performance_metrics = Set(vs.performance_metrics.clone());
            active.updated_by = Set(vs.updated_by);
            active.updated_at = Set(vs.updated_at);
            active.update(&self.db).await?
        } else {
            let active = value_stream::ActiveModel {
                id: Set(vs.id),
                logical_id: Set(vs.logical_id),
                business_version: Set(vs.business_version.clone()),
                status: Set(vs.status),
                name: Set(vs.name.clone()),
                description: Set(vs.description.clone()),
                triggering_event: Set(vs.triggering_event.clone()),
                end_deliverable: Set(vs.end_deliverable.clone()),
                owner_id: Set(vs.owner_id),
                importance: Set(vs.importance),
                stakeholders: Set(vs.stakeholders.clone()),
                performance_metrics: Set(vs.performance_metrics.clone()),
                created_by: Set(vs.created_by),
                updated_by: Set(vs.updated_by),
                created_at: Set(vs.created_at),
                updated_at: Set(vs.updated_at),
                deleted_at: Set(None),
            };
            active.insert(&self.db).await?
        };

        Ok(result.into())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError> {
        let model = value_stream::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(DomainError::ValueStreamNotFound)?;

        let mut active: value_stream::ActiveModel = model.into();
        active.deleted_at = Set(Some(chrono::Utc::now()));
        active.update(&self.db).await?;

        Ok(())
    }

    async fn list_active(
        &self,
        page: u64,
        per_page: u64,
    ) -> Result<(Vec<ValueStream>, u64), DomainError> {
        let paginator = value_stream::Entity::find()
            .filter(value_stream::Column::DeletedAt.is_null())
            .filter(value_stream::Column::Status.eq(LifecycleStatus::Active))
            .paginate(&self.db, per_page);

        let total = paginator.num_items().await?;
        let models = paginator.fetch_page(page.saturating_sub(1)).await?;

        let vss = models.into_iter().map(Into::into).collect();
        Ok((vss, total))
    }
}

#[async_trait]
impl ValueStreamStageRepository for SeaOrmValueStreamRepo {
    async fn find_by_value_stream(
        &self,
        vs_id: Uuid,
    ) -> Result<Vec<ValueStreamStage>, DomainError> {
        let models = value_stream_stage::Entity::find()
            .filter(value_stream_stage::Column::ValueStreamId.eq(vs_id))
            .filter(value_stream_stage::Column::DeletedAt.is_null())
            .all(&self.db)
            .await?;
        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn save(&self, stage: &ValueStreamStage) -> Result<ValueStreamStage, DomainError> {
        let existing = value_stream_stage::Entity::find_by_id(stage.id)
            .one(&self.db)
            .await?;

        let result = if let Some(model) = existing {
            let mut active: value_stream_stage::ActiveModel = model.into();
            active.name = Set(stage.name.clone());
            active.sequence_order = Set(stage.sequence_order);
            active.input = Set(stage.input.clone());
            active.output = Set(stage.output.clone());
            active.updated_at = Set(stage.updated_at);
            active.update(&self.db).await?
        } else {
            let active = value_stream_stage::ActiveModel {
                id: Set(stage.id),
                name: Set(stage.name.clone()),
                sequence_order: Set(stage.sequence_order),
                input: Set(stage.input.clone()),
                output: Set(stage.output.clone()),
                value_stream_id: Set(stage.value_stream_id),
                created_at: Set(stage.created_at),
                updated_at: Set(stage.updated_at),
                deleted_at: Set(None),
            };
            active.insert(&self.db).await?
        };

        Ok(result.into())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError> {
        let model = value_stream_stage::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(DomainError::ValueStreamNotFound)?;

        let mut active: value_stream_stage::ActiveModel = model.into();
        active.deleted_at = Set(Some(chrono::Utc::now()));
        active.update(&self.db).await?;

        Ok(())
    }
}

impl SeaOrmValueStreamRepo {
    pub async fn link_stage_capability(
        &self,
        stage_id: Uuid,
        capability_id: Uuid,
    ) -> Result<(), DomainError> {
        let active = stage_capability::ActiveModel {
            stage_id: Set(stage_id),
            capability_id: Set(capability_id),
        };
        stage_capability::Entity::insert(active)
            .on_conflict(sea_orm::sea_query::OnConflict::new().do_nothing().to_owned())
            .exec(&self.db)
            .await?;
        Ok(())
    }

    pub async fn unlink_stage_capability(
        &self,
        stage_id: Uuid,
        capability_id: Uuid,
    ) -> Result<(), DomainError> {
        stage_capability::Entity::delete_many()
            .filter(stage_capability::Column::StageId.eq(stage_id))
            .filter(stage_capability::Column::CapabilityId.eq(capability_id))
            .exec(&self.db)
            .await?;
        Ok(())
    }
}
