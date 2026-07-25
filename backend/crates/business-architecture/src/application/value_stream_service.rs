use chrono::Utc;
use shared_common::enums::{StageType, ValueStreamImportance};
use shared_common::value_objects::{StringStringMap, StringVec};
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::value_stream::entity::{validate_stage_flow, ValueStream, ValueStreamStage};
use crate::domain::value_stream::repository::{
    ValueStreamRepository, ValueStreamStageRepository,
};

/// Application Service for ValueStream.
/// Thin orchestration layer: coordinates domain objects and transactions.
/// No business logic here — all rules live in the domain model.
pub struct ValueStreamService<R: ValueStreamRepository, S: ValueStreamStageRepository> {
    repo: R,
    stage_repo: S,
}

impl<R: ValueStreamRepository, S: ValueStreamStageRepository> ValueStreamService<R, S> {
    pub fn new(repo: R, stage_repo: S) -> Self {
        Self { repo, stage_repo }
    }

    /// Create a new value stream (first version).
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        space_id: Uuid,
        name: String,
        description: Option<String>,
        business_version: String,
        importance: ValueStreamImportance,
        triggering_event: Option<String>,
        end_deliverable: Option<String>,
        value_proposition: Option<String>,
    ) -> Result<ValueStream, DomainError> {
        let id = Uuid::now_v7();
        let now = Utc::now();
        let mut vs = ValueStream::create(id, space_id, name, description, business_version, importance, now);
        vs.triggering_event = triggering_event;
        vs.end_deliverable = end_deliverable;
        vs.value_proposition = value_proposition;
        self.repo.save(&vs).await
    }

    /// Archive a value stream by id.
    /// Delegates to domain model for state transition validation.
    pub async fn archive(&self, id: Uuid) -> Result<(), DomainError> {
        let mut vs = self.repo.find_by_id(id).await?.ok_or(DomainError::ValueStreamNotFound)?;
        let now = Utc::now();
        vs.archive(now)?; // Domain rule: only active → archived
        self.repo.archive(id).await
    }

    /// Create a new version of an existing value stream.
    /// The current active version is archived, and a new version is created
    /// with the same logical_id. The current stages are deep-copied
    /// (snapshot) onto the new version so it is not an empty shell.
    pub async fn create_version(
        &self,
        current_id: Uuid,
        new_version: String,
        new_name: Option<String>,
        new_description: Option<String>,
    ) -> Result<ValueStream, DomainError> {
        // Load current version
        let mut current = self.repo.find_by_id(current_id).await?
            .ok_or(DomainError::ValueStreamNotFound)?;

        let now = Utc::now();
        let new_id = Uuid::now_v7();
        let name = new_name.unwrap_or_else(|| current.name.clone());
        let description = new_description.or_else(|| current.description.clone());

        // Domain rule: archive current, create new version with same logical_id
        let new_vs = current.create_new_version(new_id, new_version, name, description, now)?;

        // Snapshot the current stages onto the new version.
        let current_stages = self.stage_repo.find_by_value_stream_ordered(current_id).await?;
        let snapshots: Vec<ValueStreamStage> = current_stages
            .iter()
            .map(|s| s.snapshot_to(Uuid::now_v7(), new_vs.id, now))
            .collect();

        // Persist: save archived current, save new version, then its stages.
        // NOTE: Ideally wrapped in a single DB transaction; the repository
        // trait does not currently expose a transaction handle, so we apply
        // the writes sequentially. A future refactor should add a
        // `save_with_stages` transactional method.
        self.repo.save(&current).await?;
        self.repo.save(&new_vs).await?;
        if !snapshots.is_empty() {
            self.stage_repo.save_batch(&snapshots).await?;
        }
        Ok(new_vs)
    }

    /// Update mutable fields of an active value stream.
    pub async fn update(
        &self,
        id: Uuid,
        name: Option<String>,
        description: Option<Option<String>>,
        importance: Option<ValueStreamImportance>,
    ) -> Result<ValueStream, DomainError> {
        self.update_full(id, name, description, importance, None, None, None).await
    }

    /// Update mutable fields including value-stream metadata.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_full(
        &self,
        id: Uuid,
        name: Option<String>,
        description: Option<Option<String>>,
        importance: Option<ValueStreamImportance>,
        triggering_event: Option<Option<String>>,
        end_deliverable: Option<Option<String>>,
        value_proposition: Option<Option<String>>,
    ) -> Result<ValueStream, DomainError> {
        let mut vs = self.repo.find_by_id(id).await?.ok_or(DomainError::ValueStreamNotFound)?;
        let now = Utc::now();
        vs.update_full(
            name,
            description,
            importance,
            triggering_event,
            end_deliverable,
            value_proposition,
            now,
        )?; // Domain rule: archived cannot be updated
        self.repo.save(&vs).await
    }

    /// Get all versions of a value stream by logical_id.
    pub async fn get_versions(
        &self,
        logical_id: Uuid,
    ) -> Result<Vec<ValueStream>, DomainError> {
        self.repo.find_all_versions(logical_id).await
    }

    /// Get the active version of a value stream by logical_id.
    pub async fn get_active_by_logical_id(
        &self,
        logical_id: Uuid,
    ) -> Result<Option<ValueStream>, DomainError> {
        self.repo.find_active_by_logical_id(logical_id).await
    }

    // ---------------------------------------------------------------------
    // Stage management (stages are entities within the ValueStream aggregate)
    // ---------------------------------------------------------------------

    /// List the stages of a value stream, ordered by `sequence_order`.
    pub async fn list_stages(&self, vs_id: Uuid) -> Result<Vec<ValueStreamStage>, DomainError> {
        self.stage_repo.find_by_value_stream_ordered(vs_id).await
    }

    /// Add a new stage to a value stream. The stage's `sequence_order` must
    /// not collide with an existing stage; the resulting flow must be
    /// contiguous starting at 1.
    #[allow(clippy::too_many_arguments)]
    pub async fn add_stage(
        &self,
        vs_id: Uuid,
        name: String,
        sequence_order: i32,
        stage_type: StageType,
        description: Option<String>,
        input: Option<String>,
        output: Option<String>,
        owner_id: Option<Uuid>,
        objectives: Option<StringVec>,
        metrics: Option<StringStringMap>,
    ) -> Result<ValueStreamStage, DomainError> {
        // The parent value stream must exist and be active.
        let vs = self.repo.find_by_id(vs_id).await?.ok_or(DomainError::ValueStreamNotFound)?;
        if !vs.is_active() {
            return Err(DomainError::CannotModifyArchived {
                entity: "ValueStream".to_string(),
            });
        }

        let id = Uuid::now_v7();
        let now = Utc::now();
        let mut stage = ValueStreamStage::create(id, vs_id, name, sequence_order, stage_type, now)?;
        stage.description = description;
        stage.input = input;
        stage.output = output;
        stage.owner_id = owner_id;
        if let Some(o) = objectives { stage.objectives = o; }
        if let Some(m) = metrics { stage.metrics = m; }

        // Validate the resulting flow (uniqueness + contiguity).
        let mut stages = self.stage_repo.find_by_value_stream_ordered(vs_id).await?;
        stages.push(stage.clone());
        validate_stage_flow(&stages, vs_id)?;

        self.stage_repo.save(&stage).await
    }

    /// Update a stage's mutable fields, including its `sequence_order`. When
    /// the order changes the resulting flow is re-validated for uniqueness and
    /// contiguity. The parent value stream must be active.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_stage(
        &self,
        stage_id: Uuid,
        name: Option<String>,
        description: Option<Option<String>>,
        stage_type: Option<StageType>,
        sequence_order: Option<i32>,
        input: Option<Option<String>>,
        output: Option<Option<String>>,
        owner_id: Option<Option<Uuid>>,
        objectives: Option<StringVec>,
        metrics: Option<StringStringMap>,
    ) -> Result<ValueStreamStage, DomainError> {
        let mut stage = self
            .stage_repo
            .find_by_id(stage_id)
            .await?
            .ok_or(DomainError::StageNotFound)?;

        // Parent-active guard: stages of an archived value stream are immutable.
        let vs = self
            .repo
            .find_by_id(stage.value_stream_id)
            .await?
            .ok_or(DomainError::ValueStreamNotFound)?;
        if !vs.is_active() {
            return Err(DomainError::CannotModifyArchived {
                entity: "ValueStream".to_string(),
            });
        }

        let now = Utc::now();
        stage.update(
            name, description, stage_type, input, output, owner_id, objectives, metrics, now,
        )?;

        // Apply sequence-order change and validate the resulting flow.
        if let Some(new_order) = sequence_order {
            if new_order < 1 {
                return Err(DomainError::InvalidStageOrder { order: new_order });
            }
            stage.sequence_order = new_order;
            stage.updated_at = now;
        }
        let mut stages = self
            .stage_repo
            .find_by_value_stream_ordered(stage.value_stream_id)
            .await?;
        // Replace the in-memory copy of this stage with the updated one.
        if let Some(slot) = stages.iter_mut().find(|s| s.id == stage.id) {
            *slot = stage.clone();
        } else {
            stages.push(stage.clone());
        }
        validate_stage_flow(&stages, stage.value_stream_id)?;

        self.stage_repo.save(&stage).await
    }

    /// Publish a draft stage (Draft → Active).
    pub async fn publish_stage(&self, stage_id: Uuid) -> Result<ValueStreamStage, DomainError> {
        let mut stage = self
            .stage_repo
            .find_by_id(stage_id)
            .await?
            .ok_or(DomainError::StageNotFound)?;
        let now = Utc::now();
        stage.publish(now)?;
        self.stage_repo.save(&stage).await
    }

    /// Archive a stage (Draft/Active → Archived).
    pub async fn archive_stage(&self, stage_id: Uuid) -> Result<ValueStreamStage, DomainError> {
        let mut stage = self
            .stage_repo
            .find_by_id(stage_id)
            .await?
            .ok_or(DomainError::StageNotFound)?;
        let now = Utc::now();
        stage.archive(now)?;
        self.stage_repo.save(&stage).await
    }

    /// Remove a stage (soft delete). The parent value stream must be active so
    /// the flow of an archived (historical) version is never mutated.
    pub async fn remove_stage(&self, stage_id: Uuid) -> Result<(), DomainError> {
        let stage = self
            .stage_repo
            .find_by_id(stage_id)
            .await?
            .ok_or(DomainError::StageNotFound)?;
        let vs = self
            .repo
            .find_by_id(stage.value_stream_id)
            .await?
            .ok_or(DomainError::ValueStreamNotFound)?;
        if !vs.is_active() {
            return Err(DomainError::CannotModifyArchived {
                entity: "ValueStream".to_string(),
            });
        }
        self.stage_repo.soft_delete(stage_id).await
    }

    /// Reorder stages within a value stream. `ordered_ids` is the desired
    /// order of stage ids (first → last). The stages are renumbered so their
    /// `sequence_order` becomes 1..=N.
    pub async fn reorder_stages(
        &self,
        vs_id: Uuid,
        ordered_ids: Vec<Uuid>,
    ) -> Result<Vec<ValueStreamStage>, DomainError> {
        let vs = self.repo.find_by_id(vs_id).await?.ok_or(DomainError::ValueStreamNotFound)?;
        if !vs.is_active() {
            return Err(DomainError::CannotModifyArchived {
                entity: "ValueStream".to_string(),
            });
        }

        let existing = self.stage_repo.find_by_value_stream_ordered(vs_id).await?;
        if ordered_ids.len() != existing.len() {
            return Err(DomainError::Validation(
                "reorder_stages: must provide every stage id".to_string(),
            ));
        }

        let now = Utc::now();
        let mut updated: Vec<ValueStreamStage> = Vec::with_capacity(existing.len());
        for (idx, id) in ordered_ids.iter().enumerate() {
            let mut stage = existing
                .iter()
                .find(|s| &s.id == id)
                .cloned()
                .ok_or(DomainError::StageNotFound)?;
            stage.reorder((idx + 1) as i32, now)?;
            updated.push(stage);
        }
        self.stage_repo.save_batch(&updated).await?;
        Ok(updated)
    }
}
