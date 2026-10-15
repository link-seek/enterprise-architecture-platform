use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    PaginatorTrait, QueryFilter, Set, QueryOrder, TransactionTrait,
};
use shared_common::enums::LifecycleStatus;
use std::collections::HashMap;
use uuid::Uuid;

use crate::application::publish::{MovedLink, PublishOverrides, PublishStrategy};
use crate::domain::error::DomainError;
use crate::domain::value_stream::entity::{ValueStream, ValueStreamStage};
use crate::domain::value_stream::repository::{ValueStreamRepository, ValueStreamStageRepository};
use crate::infrastructure::persistence::entities::{business_capability, stage_capability, value_stream, value_stream_stage};

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

    /// Publish one new version through the unified engine
    /// ([`crate::application::publish::publish`]): the same six steps as
    /// every other publishable element. Returns the new row id. (The engine
    /// also yields moved links for the impact receipt; the
    /// `valueStreamCreateVersion` API surface is unchanged, so they are
    /// dropped here. The flow itself is the shared implementation.)
    async fn publish_version(
        &self,
        logical_id: Uuid,
        new_version: String,
        new_name: Option<String>,
        new_description: Option<String>,
    ) -> Result<Uuid, DomainError> {
        let (new_id, _moved) = crate::application::publish::publish(
            self,
            logical_id,
            &new_version,
            &PublishOverrides {
                name: new_name,
                description: new_description,
            },
        )
        .await?;
        Ok(new_id)
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

#[async_trait]
impl PublishStrategy for SeaOrmValueStreamRepo {
    fn entity_name(&self) -> &'static str {
        "ValueStream"
    }

    fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    async fn existing_versions(
        &self,
        txn: &DatabaseTransaction,
        logical_id: Uuid,
    ) -> Result<Vec<(Uuid, String)>, DomainError> {
        Ok(value_stream::Entity::find()
            .filter(value_stream::Column::LogicalId.eq(logical_id))
            .filter(value_stream::Column::DeletedAt.is_null())
            .all(txn)
            .await?
            .into_iter()
            .map(|r| (r.id, r.business_version))
            .collect())
    }

    async fn deprecate_active(
        &self,
        txn: &DatabaseTransaction,
        logical_id: Uuid,
    ) -> Result<(Uuid, String), DomainError> {
        let old = value_stream::Entity::find()
            .filter(value_stream::Column::LogicalId.eq(logical_id))
            .filter(value_stream::Column::Status.eq(LifecycleStatus::Active))
            .filter(value_stream::Column::DeletedAt.is_null())
            .one(txn)
            .await?
            .ok_or(DomainError::ValueStreamNotFound)?;

        // Conditional compare-and-swap: the flip only lands if the row is
        // still Active. Concurrent double-publishes serialize on the row
        // lock; the loser matches zero rows and aborts instead of producing
        // two Active rows.
        let flipped = value_stream::Entity::update_many()
            .col_expr(
                value_stream::Column::Status,
                sea_orm::sea_query::Expr::value(LifecycleStatus::Deprecated),
            )
            .col_expr(
                value_stream::Column::UpdatedAt,
                sea_orm::sea_query::Expr::value(chrono::Utc::now()),
            )
            .filter(value_stream::Column::Id.eq(old.id))
            .filter(value_stream::Column::Status.eq(LifecycleStatus::Active))
            .exec(txn)
            .await
            .map_err(|e| {
                DomainError::Database(format!(
                    "publish: deprecate current version {}: {e}",
                    old.id
                ))
            })?;
        if flipped.rows_affected != 1 {
            return Err(DomainError::InvalidTransition {
                from: "Active".to_string(),
                to: "Deprecated".to_string(),
                entity: "ValueStream".to_string(),
            });
        }
        Ok((old.id, old.business_version))
    }

    async fn insert_new(
        &self,
        txn: &DatabaseTransaction,
        old_id: Uuid,
        new_version: &str,
        overrides: &PublishOverrides,
    ) -> Result<Uuid, DomainError> {
        let old = value_stream::Entity::find_by_id(old_id)
            .one(txn)
            .await?
            .ok_or(DomainError::ValueStreamNotFound)?;

        let now = chrono::Utc::now();
        let new_id = Uuid::now_v7();
        let new_active = value_stream::ActiveModel {
            id: Set(new_id),
            logical_id: Set(old.logical_id),
            business_version: Set(new_version.to_owned()),
            status: Set(LifecycleStatus::Active),
            name: Set(overrides.name.clone().unwrap_or_else(|| old.name.clone())),
            description: Set(overrides.description.clone().or(old.description.clone())),
            triggering_event: Set(old.triggering_event.clone()),
            end_deliverable: Set(old.end_deliverable.clone()),
            owner_id: Set(old.owner_id),
            importance: Set(old.importance),
            stakeholders: Set(old.stakeholders.clone()),
            performance_metrics: Set(old.performance_metrics.clone()),
            created_by: Set(old.created_by),
            updated_by: Set(old.updated_by),
            created_at: Set(now),
            updated_at: Set(now),
            deleted_at: Set(None),
            space_id: Set(old.space_id),
        };
        new_active.insert(txn).await.map_err(|e| {
            DomainError::Database(format!("publish: insert new version: {e}"))
        })?;

        // Copy stages to the new version (new ids, fields preserved).
        let old_stages = value_stream_stage::Entity::find()
            .filter(value_stream_stage::Column::ValueStreamId.eq(old_id))
            .filter(value_stream_stage::Column::DeletedAt.is_null())
            .all(txn)
            .await
            .map_err(|e| {
                DomainError::Database(format!("publish: list old stages of {old_id}: {e}"))
            })?;
        for m in old_stages {
            let stage: ValueStreamStage =
                ValueStreamStage::from(m).for_version(Uuid::now_v7(), new_id);
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
            active.insert(txn).await.map_err(|e| {
                DomainError::Database(format!("publish: insert stage {}: {e}", stage.id))
            })?;
        }
        Ok(new_id)
    }

    async fn move_links(
        &self,
        txn: &DatabaseTransaction,
        old_id: Uuid,
        new_id: Uuid,
        old_version: &str,
        new_version: &str,
    ) -> Result<Vec<MovedLink>, DomainError> {
        // Names for the impact receipt (same snapshot).
        let old_stages = value_stream_stage::Entity::find()
            .filter(value_stream_stage::Column::ValueStreamId.eq(old_id))
            .filter(value_stream_stage::Column::DeletedAt.is_null())
            .all(txn)
            .await
            .map_err(|e| {
                DomainError::Database(format!("publish: list old stages of {old_id}: {e}"))
            })?;
        let mut cap_ids: Vec<Uuid> = Vec::new();
        for s in &old_stages {
            let links = stage_capability::Entity::find()
                .filter(stage_capability::Column::StageId.eq(s.id))
                .all(txn)
                .await
                .map_err(|e| {
                    DomainError::Database(format!(
                        "publish: list stage_capabilities of stage {}: {e}",
                        s.id
                    ))
                })?;
            cap_ids.extend(links.into_iter().map(|l| l.capability_id));
        }
        cap_ids.sort();
        cap_ids.dedup();
        let cap_name_by_id: std::collections::HashMap<Uuid, String> = if cap_ids.is_empty() {
            Default::default()
        } else {
            business_capability::Entity::find()
                .filter(business_capability::Column::Id.is_in(cap_ids))
                .all(txn)
                .await?
                .into_iter()
                .map(|c| (c.id, c.name))
                .collect()
        };
        let moved: Vec<MovedLink> = cap_name_by_id
            .iter()
            .map(|(id, name)| MovedLink {
                capability_id: *id,
                capability_name: name.clone(),
                old_version: old_version.to_owned(),
                new_version: new_version.to_owned(),
            })
            .collect();

        // Move `stage_capabilities` to the corresponding new stages (matched
        // by sequence_order). The live version owns the live links; old rows
        // keep their content snapshot, link-free.
        let new_stages = value_stream_stage::Entity::find()
            .filter(value_stream_stage::Column::ValueStreamId.eq(new_id))
            .filter(value_stream_stage::Column::DeletedAt.is_null())
            .all(txn)
            .await
            .map_err(|e| {
                DomainError::Database(format!("publish: list new stages of {new_id}: {e}"))
            })?;
        let new_stage_id_by_seq: HashMap<i32, Uuid> = new_stages
            .iter()
            .map(|s| (s.sequence_order, s.id))
            .collect();
        for s in &old_stages {
            let Some(new_stage_id) = new_stage_id_by_seq.get(&s.sequence_order) else {
                continue;
            };
            // Move (single UPDATE): every current link — including ones added
            // concurrently — lands on the new stage; nothing is orphaned.
            stage_capability::Entity::update_many()
                .col_expr(
                    stage_capability::Column::StageId,
                    sea_orm::sea_query::Expr::value(*new_stage_id),
                )
                .filter(stage_capability::Column::StageId.eq(s.id))
                .exec(txn)
                .await
                .map_err(|e| {
                    DomainError::Database(format!(
                        "publish: move stage_capabilities {} → {new_stage_id}: {e}",
                        s.id
                    ))
                })?;
        }
        Ok(moved)
    }
}