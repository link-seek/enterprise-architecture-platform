use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set, QueryOrder, TransactionTrait,
};
use shared_common::enums::LifecycleStatus;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::value_stream::entity::{ValueStream, ValueStreamStage};
use crate::domain::value_stream::repository::{ValueStreamRepository, ValueStreamStageRepository};
use crate::infrastructure::persistence::entities::{stage_capability, value_stream, value_stream_stage};

/// Maps a SeaORM `DbErr` to `DomainError::DuplicateSequenceOrder` when the
/// error is a unique-constraint violation on the stage sequence-order index,
/// otherwise passes through as a generic `Database` error. This provides the
/// DB-level safety net for the TOCTOU race between the application-level
/// `sequence_order_exists` check and the `save` insert.
fn map_unique_violation(e: sea_orm::DbErr) -> DomainError {
    let msg = e.to_string();
    let is_unique_violation = msg.contains("UNIQUE constraint failed")
        || msg.contains("duplicate key value violates unique constraint")
        || msg.contains("Duplicate entry");
    if is_unique_violation {
        tracing::warn!(error = %msg, "unique constraint violation on value_stream_stage save");
        DomainError::DuplicateSequenceOrder
    } else {
        DomainError::Database(msg)
    }
}

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
        let objective_metrics = match &m.objective_metrics {
            Some(s) => match serde_json::from_str(s) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        stage_id = %m.id,
                        "Failed to parse objective_metrics JSON from DB; using empty default"
                    );
                    Default::default()
                }
            },
            None => Default::default(),
        };
        let key_metrics = match &m.key_metrics {
            Some(s) => match serde_json::from_str(s) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        stage_id = %m.id,
                        "Failed to parse key_metrics JSON from DB; using empty default"
                    );
                    Default::default()
                }
            },
            None => Default::default(),
        };
        ValueStreamStage {
            id: m.id,
            name: m.name,
            sequence_order: m.sequence_order,
            input: m.input,
            output: m.output,
            value_stream_id: m.value_stream_id,
            description: m.description,
            objective_metrics,
            entry_criteria: m.entry_criteria,
            exit_criteria: m.exit_criteria,
            owner_id: m.owner_id,
            key_metrics,
            created_at: m.created_at,
            updated_at: m.updated_at,
            deleted_at: m.deleted_at,
        }
    }
}

impl From<ValueStreamStage> for value_stream_stage::Model {
    fn from(s: ValueStreamStage) -> Self {
        // Always serialize metrics to Some(json) for consistency with the
        // `save` method, which also uses Some(json). This avoids NULL vs
        // "[]" storage inconsistency across different persistence paths.
        let objective_metrics = serde_json::to_string(&s.objective_metrics)
            .map(Some)
            .unwrap_or_else(|e| {
                tracing::error!(
                    error = %e,
                    stage_id = %s.id,
                    "Failed to serialize objective_metrics; storing as NULL"
                );
                None
            });
        let key_metrics = serde_json::to_string(&s.key_metrics)
            .map(Some)
            .unwrap_or_else(|e| {
                tracing::error!(
                    error = %e,
                    stage_id = %s.id,
                    "Failed to serialize key_metrics; storing as NULL"
                );
                None
            });
        value_stream_stage::Model {
            id: s.id,
            name: s.name,
            sequence_order: s.sequence_order,
            input: s.input,
            output: s.output,
            value_stream_id: s.value_stream_id,
            description: s.description,
            objective_metrics,
            entry_criteria: s.entry_criteria,
            exit_criteria: s.exit_criteria,
            owner_id: s.owner_id,
            key_metrics,
            created_at: s.created_at,
            updated_at: s.updated_at,
            deleted_at: s.deleted_at,
        }
    }
}

#[derive(Clone)]
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

    async fn save_version_with_stages(
        &self,
        current_id: Uuid,
        new_vs_id: Uuid,
        vss: &[ValueStream],
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<ValueStreamStage>, DomainError> {
        let txn = self.db.begin().await?;

        // Save value stream versions (archived current + new version).
        for vs in vss {
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
                        "save_version_with_stages: failed to update value stream (id={}): {e}",
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
                        "save_version_with_stages: failed to insert value stream (id={}): {e}",
                        vs.id
                    ))
                })?;
            }
        }

        // Read current stages inside the transaction to avoid TOCTOU races.
        let current_stage_models = value_stream_stage::Entity::find()
            .filter(value_stream_stage::Column::ValueStreamId.eq(current_id))
            .filter(value_stream_stage::Column::DeletedAt.is_null())
            .all(&txn)
            .await?;

        // Clone stages for the new version (each gets a fresh id).
        let mut new_stages: Vec<ValueStreamStage> = Vec::new();
        for model in current_stage_models {
            let stage: ValueStreamStage = model.into();
            new_stages.push(stage.clone_for_new_version(Uuid::now_v7(), new_vs_id, now));
        }

        // Copy stages to the new version.
        for stage in &new_stages {
            let objective_metrics_json = serde_json::to_string(&stage.objective_metrics)
                .map_err(|e| DomainError::Database(format!("serialize objective_metrics: {e}")))?;
            let key_metrics_json = serde_json::to_string(&stage.key_metrics)
                .map_err(|e| DomainError::Database(format!("serialize key_metrics: {e}")))?;

            let active = value_stream_stage::ActiveModel {
                id: Set(stage.id),
                name: Set(stage.name.clone()),
                sequence_order: Set(stage.sequence_order),
                input: Set(stage.input.clone()),
                output: Set(stage.output.clone()),
                value_stream_id: Set(stage.value_stream_id),
                description: Set(stage.description.clone()),
                objective_metrics: Set(Some(objective_metrics_json)),
                entry_criteria: Set(stage.entry_criteria.clone()),
                exit_criteria: Set(stage.exit_criteria.clone()),
                owner_id: Set(stage.owner_id),
                key_metrics: Set(Some(key_metrics_json)),
                created_at: Set(stage.created_at),
                updated_at: Set(stage.updated_at),
                deleted_at: Set(stage.deleted_at),
            };
            active.insert(&txn).await.map_err(|e| {
                DomainError::Database(format!(
                    "save_version_with_stages: failed to insert stage (id={}): {e}",
                    stage.id
                ))
            })?;
        }

        txn.commit().await?;
        Ok(new_stages)
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

    async fn find_by_id(&self, id: Uuid) -> Result<Option<ValueStreamStage>, DomainError> {
        let model = value_stream_stage::Entity::find()
            .filter(value_stream_stage::Column::Id.eq(id))
            .filter(value_stream_stage::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;
        Ok(model.map(Into::into))
    }

    async fn save(&self, stage: &ValueStreamStage) -> Result<ValueStreamStage, DomainError> {
        let objective_metrics_json = serde_json::to_string(&stage.objective_metrics)
            .map_err(|e| DomainError::Database(format!("serialize objective_metrics: {e}")))?;
        let key_metrics_json = serde_json::to_string(&stage.key_metrics)
            .map_err(|e| DomainError::Database(format!("serialize key_metrics: {e}")))?;

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
            active.objective_metrics = Set(Some(objective_metrics_json));
            active.entry_criteria = Set(stage.entry_criteria.clone());
            active.exit_criteria = Set(stage.exit_criteria.clone());
            active.owner_id = Set(stage.owner_id);
            active.key_metrics = Set(Some(key_metrics_json));
            active.updated_at = Set(stage.updated_at);
            active.deleted_at = Set(stage.deleted_at);
            active.update(&self.db).await.map_err(map_unique_violation)?
        } else {
            let active = value_stream_stage::ActiveModel {
                id: Set(stage.id),
                name: Set(stage.name.clone()),
                sequence_order: Set(stage.sequence_order),
                input: Set(stage.input.clone()),
                output: Set(stage.output.clone()),
                value_stream_id: Set(stage.value_stream_id),
                description: Set(stage.description.clone()),
                objective_metrics: Set(Some(objective_metrics_json)),
                entry_criteria: Set(stage.entry_criteria.clone()),
                exit_criteria: Set(stage.exit_criteria.clone()),
                owner_id: Set(stage.owner_id),
                key_metrics: Set(Some(key_metrics_json)),
                created_at: Set(stage.created_at),
                updated_at: Set(stage.updated_at),
                deleted_at: Set(stage.deleted_at),
            };
            active.insert(&self.db).await.map_err(map_unique_violation)?
        };

        Ok(result.into())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError> {
        let model = value_stream_stage::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(DomainError::ValueStreamStageNotFound)?;

        let mut active: value_stream_stage::ActiveModel = model.into();
        active.deleted_at = Set(Some(chrono::Utc::now()));
        active.update(&self.db).await?;

        Ok(())
    }

    async fn sequence_order_exists(
        &self,
        vs_id: Uuid,
        sequence_order: i32,
        exclude_id: Option<Uuid>,
    ) -> Result<bool, DomainError> {
        let mut query = value_stream_stage::Entity::find()
            .filter(value_stream_stage::Column::ValueStreamId.eq(vs_id))
            .filter(value_stream_stage::Column::SequenceOrder.eq(sequence_order))
            .filter(value_stream_stage::Column::DeletedAt.is_null());
        if let Some(excl) = exclude_id {
            query = query.filter(value_stream_stage::Column::Id.ne(excl));
        }
        let count = query.count(&self.db).await?;
        Ok(count > 0)
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
