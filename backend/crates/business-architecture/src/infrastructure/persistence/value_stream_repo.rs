use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set, QueryOrder, TransactionTrait,
};
use shared_common::enums::LifecycleStatus;
use std::collections::HashMap;
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
            space_id: m.space_id,
        }
    }
}

impl From<&ValueStream> for value_stream::Model {
    fn from(vs: &ValueStream) -> Self {
        value_stream::Model {
            id: vs.id,
            logical_id: vs.logical_id,
            business_version: vs.business_version.clone(),
            status: vs.status,
            name: vs.name.clone(),
            description: vs.description.clone(),
            triggering_event: vs.triggering_event.clone(),
            end_deliverable: vs.end_deliverable.clone(),
            owner_id: vs.owner_id,
            importance: vs.importance,
            stakeholders: vs.stakeholders.clone(),
            performance_metrics: vs.performance_metrics.clone(),
            created_by: vs.created_by,
            updated_by: vs.updated_by,
            created_at: vs.created_at,
            updated_at: vs.updated_at,
            deleted_at: vs.deleted_at,
            space_id: vs.space_id,
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
            description: m.description,
            objective_metrics: m.objective_metrics.unwrap_or_default(),
            entry_criteria: m.entry_criteria,
            exit_criteria: m.exit_criteria,
            owner_id: m.owner_id,
            key_metrics: m.key_metrics.unwrap_or_default(),
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
            active.deleted_at = Set(vs.deleted_at);
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
                deleted_at: Set(vs.deleted_at),
                space_id: Set(vs.space_id),
            };
            active.insert(&self.db).await?
        };

        Ok(result.into())
    }

    async fn save_batch(&self, vss: &[ValueStream]) -> Result<(), DomainError> {
        let txn = self.db.begin().await?;
        for (idx, vs) in vss.iter().enumerate() {
            let existing = value_stream::Entity::find_by_id(vs.id)
                .one(&txn)
                .await?;

            if let Some(model) = existing {
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
                active.deleted_at = Set(vs.deleted_at);
                active.update(&txn).await.map_err(|e| {
                    DomainError::Database(format!(
                        "save_batch: failed to update record at index {idx} (id={}): {e}",
                        vs.id
                    ))
                })?;
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
                    deleted_at: Set(vs.deleted_at),
                    space_id: Set(vs.space_id),
                };
                active.insert(&txn).await.map_err(|e| {
                    DomainError::Database(format!(
                        "save_batch: failed to insert record at index {idx} (id={}): {e}",
                        vs.id
                    ))
                })?;
            }
        }
        txn.commit().await?;
        Ok(())
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

    async fn find_stages_by_value_stream(
        &self,
        vs_id: Uuid,
    ) -> Result<Vec<ValueStreamStage>, DomainError> {
        let models = value_stream_stage::Entity::find()
            .filter(value_stream_stage::Column::ValueStreamId.eq(vs_id))
            .filter(value_stream_stage::Column::DeletedAt.is_null())
            .order_by_asc(value_stream_stage::Column::SequenceOrder)
            .all(&self.db)
            .await?;
        Ok(models.into_iter().map(Into::into).collect())
    }

    async fn save_version_atomic(
        &self,
        current: &ValueStream,
        new_version: &ValueStream,
        new_stages: &[ValueStreamStage],
    ) -> Result<(), DomainError> {
        let txn = self.db.begin().await?;

        // 1. Archive the current version.
        let current_model = value_stream::Entity::find_by_id(current.id)
            .one(&txn)
            .await?
            .ok_or(DomainError::ValueStreamNotFound)?;
        let mut current_active: value_stream::ActiveModel = current_model.into();
        current_active.status = Set(LifecycleStatus::Archived);
        current_active.updated_at = Set(current.updated_at);
        current_active.update(&txn).await.map_err(|e| {
            DomainError::Database(format!("save_version_atomic: archive current version: {e}"))
        })?;

        // 2. Insert the new version.
        let new_active = value_stream::ActiveModel {
            id: Set(new_version.id),
            logical_id: Set(new_version.logical_id),
            business_version: Set(new_version.business_version.clone()),
            status: Set(new_version.status),
            name: Set(new_version.name.clone()),
            description: Set(new_version.description.clone()),
            triggering_event: Set(new_version.triggering_event.clone()),
            end_deliverable: Set(new_version.end_deliverable.clone()),
            owner_id: Set(new_version.owner_id),
            importance: Set(new_version.importance),
            stakeholders: Set(new_version.stakeholders.clone()),
            performance_metrics: Set(new_version.performance_metrics.clone()),
            created_by: Set(new_version.created_by),
            updated_by: Set(new_version.updated_by),
            created_at: Set(new_version.created_at),
            updated_at: Set(new_version.updated_at),
            deleted_at: Set(new_version.deleted_at),
            space_id: Set(new_version.space_id),
        };
        new_active.insert(&txn).await.map_err(|e| {
            DomainError::Database(format!("save_version_atomic: insert new version: {e}"))
        })?;

        // 3. Copy stages to the new version.
        for stage in new_stages {
            let active = value_stream_stage::ActiveModel {
                id: Set(stage.id),
                name: Set(stage.name.clone()),
                sequence_order: Set(stage.sequence_order),
                input: Set(stage.input.clone()),
                output: Set(stage.output.clone()),
                description: Set(stage.description.clone()),
                objective_metrics: Set(Some(stage.objective_metrics.clone())),
                entry_criteria: Set(stage.entry_criteria.clone()),
                exit_criteria: Set(stage.exit_criteria.clone()),
                owner_id: Set(stage.owner_id),
                key_metrics: Set(Some(stage.key_metrics.clone())),
                value_stream_id: Set(stage.value_stream_id),
                created_at: Set(stage.created_at),
                updated_at: Set(stage.updated_at),
                deleted_at: Set(stage.deleted_at),
            };
            active.insert(&txn).await.map_err(|e| {
                DomainError::Database(format!(
                    "save_version_atomic: insert stage {}: {e}",
                    stage.id
                ))
            })?;
        }

        // 4. Copy `stage_capabilities` associations from the old stages to the
        //    corresponding new stages (matched by sequence_order). Without this
        //    the new version's stages would have no capability links while the
        //    old links remain attached to the just-archived stage rows.
        let old_stages = value_stream_stage::Entity::find()
            .filter(value_stream_stage::Column::ValueStreamId.eq(current.id))
            .all(&txn)
            .await
            .map_err(|e| {
                DomainError::Database(format!(
                    "save_version_atomic: list old stages of {}: {e}",
                    current.id
                ))
            })?;
        let old_stage_id_by_seq: HashMap<i32, Uuid> = old_stages
            .iter()
            .map(|s| (s.sequence_order, s.id))
            .collect();
        for stage in new_stages {
            let Some(old_stage_id) = old_stage_id_by_seq.get(&stage.sequence_order) else {
                continue;
            };
            let links = stage_capability::Entity::find()
                .filter(stage_capability::Column::StageId.eq(*old_stage_id))
                .all(&txn)
                .await
                .map_err(|e| {
                    DomainError::Database(format!(
                        "save_version_atomic: list stage_capabilities of stage {old_stage_id}: {e}"
                    ))
                })?;
            for link in links {
                let active = stage_capability::ActiveModel {
                    stage_id: Set(stage.id),
                    capability_id: Set(link.capability_id),
                };
                stage_capability::Entity::insert(active)
                    .on_conflict(sea_orm::sea_query::OnConflict::new().do_nothing().to_owned())
                    .exec(&txn)
                    .await
                    .map_err(|e| {
                        DomainError::Database(format!(
                            "save_version_atomic: copy stage_capability ({}, {}) → {}: {e}",
                            old_stage_id,
                            link.capability_id,
                            stage.id
                        ))
                    })?;
            }
        }

        txn.commit().await?;
        Ok(())
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
            active.description = Set(stage.description.clone());
            active.objective_metrics = Set(Some(stage.objective_metrics.clone()));
            active.entry_criteria = Set(stage.entry_criteria.clone());
            active.exit_criteria = Set(stage.exit_criteria.clone());
            active.owner_id = Set(stage.owner_id);
            active.key_metrics = Set(Some(stage.key_metrics.clone()));
            active.updated_at = Set(stage.updated_at);
            active.deleted_at = Set(stage.deleted_at);
            active.update(&self.db).await?
        } else {
            let active = value_stream_stage::ActiveModel {
                id: Set(stage.id),
                name: Set(stage.name.clone()),
                sequence_order: Set(stage.sequence_order),
                input: Set(stage.input.clone()),
                output: Set(stage.output.clone()),
                description: Set(stage.description.clone()),
                objective_metrics: Set(Some(stage.objective_metrics.clone())),
                entry_criteria: Set(stage.entry_criteria.clone()),
                exit_criteria: Set(stage.exit_criteria.clone()),
                owner_id: Set(stage.owner_id),
                key_metrics: Set(Some(stage.key_metrics.clone())),
                value_stream_id: Set(stage.value_stream_id),
                created_at: Set(stage.created_at),
                updated_at: Set(stage.updated_at),
                deleted_at: Set(stage.deleted_at),
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
