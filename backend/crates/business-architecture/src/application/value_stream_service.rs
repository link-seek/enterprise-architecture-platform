use chrono::Utc;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::value_stream::entity::{ValueStream, ValueStreamStage};
use crate::domain::value_stream::repository::{
    ValueStreamRepository, ValueStreamStageRepository,
};
use shared_common::value_objects::{StringStringMap, StringVec};

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
    pub async fn create(
        &self,
        space_id: Uuid,
        name: String,
        description: Option<String>,
        business_version: String,
        importance: shared_common::enums::ValueStreamImportance,
        owner_id: Option<Uuid>,
        triggering_event: Option<String>,
        end_deliverable: Option<String>,
        stakeholders: StringVec,
        performance_metrics: StringStringMap,
    ) -> Result<ValueStream, DomainError> {
        let id = Uuid::now_v7();
        let now = Utc::now();
        let vs = ValueStream::create(
            id,
            space_id,
            name,
            description,
            business_version,
            importance,
            owner_id,
            triggering_event,
            end_deliverable,
            stakeholders,
            performance_metrics,
            now,
        );
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
    /// with the same logical_id. Stages are copied to the new version.
    /// The whole operation is wrapped in a transaction.
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

        // Load existing stages to copy into the new version.
        let stages = self.stage_repo.find_by_value_stream(current_id).await?;

        // Persist: save archived current and new version atomically
        self.repo.save_batch(&[current, new_vs.clone()]).await?;

        // Copy stages to the new version (each gets a fresh id).
        for stage in stages {
            let new_stage = stage.clone_for_new_version(Uuid::now_v7(), new_id, now);
            self.stage_repo.save(&new_stage).await?;
        }

        Ok(new_vs)
    }

    /// Update mutable fields of an active value stream.
    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        &self,
        id: Uuid,
        name: Option<String>,
        description: Option<Option<String>>,
        importance: Option<shared_common::enums::ValueStreamImportance>,
        owner_id: Option<Uuid>,
        triggering_event: Option<Option<String>>,
        end_deliverable: Option<Option<String>>,
        stakeholders: Option<StringVec>,
        performance_metrics: Option<StringStringMap>,
    ) -> Result<ValueStream, DomainError> {
        let mut vs = self.repo.find_by_id(id).await?.ok_or(DomainError::ValueStreamNotFound)?;
        let now = Utc::now();
        vs.update(
            name,
            description,
            importance,
            owner_id,
            triggering_event,
            end_deliverable,
            stakeholders,
            performance_metrics,
            now,
        )?; // Domain rule: archived cannot be updated
        self.repo.save(&vs).await
    }

    /// Transfer ownership of a value stream to a new user.
    /// The caller (GraphQL layer) is responsible for verifying that the actor
    /// is the current owner or an admin, and that `new_owner_id` is a member
    /// of the same space.
    pub async fn transfer_ownership(
        &self,
        id: Uuid,
        new_owner_id: Uuid,
    ) -> Result<ValueStream, DomainError> {
        let mut vs = self.repo.find_by_id(id).await?.ok_or(DomainError::ValueStreamNotFound)?;
        let now = Utc::now();
        vs.transfer_ownership(new_owner_id, now);
        self.repo.save(&vs).await
    }

    /// Create a new stage with sequence-order uniqueness validation.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_stage(
        &self,
        value_stream_id: Uuid,
        name: String,
        sequence_order: i32,
        input: Option<String>,
        output: Option<String>,
        description: Option<String>,
        objective_metrics: StringStringMap,
        entry_criteria: Option<String>,
        exit_criteria: Option<String>,
        owner_id: Option<Uuid>,
        key_metrics: StringStringMap,
    ) -> Result<ValueStreamStage, DomainError> {
        if self
            .stage_repo
            .sequence_order_exists(value_stream_id, sequence_order, None)
            .await?
        {
            return Err(DomainError::InvalidTransition {
                from: format!("sequence_order {sequence_order}"),
                to: "unique".to_string(),
                entity: "ValueStreamStage".to_string(),
            });
        }
        let id = Uuid::now_v7();
        let now = Utc::now();
        let stage = ValueStreamStage::create(
            id,
            value_stream_id,
            name,
            sequence_order,
            input,
            output,
            description,
            objective_metrics,
            entry_criteria,
            exit_criteria,
            owner_id,
            key_metrics,
            now,
        );
        self.stage_repo.save(&stage).await
    }

    /// Update a stage with sequence-order uniqueness validation.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_stage(
        &self,
        id: Uuid,
        name: Option<String>,
        sequence_order: Option<i32>,
        input: Option<Option<String>>,
        output: Option<Option<String>>,
        description: Option<Option<String>>,
        objective_metrics: Option<StringStringMap>,
        entry_criteria: Option<Option<String>>,
        exit_criteria: Option<Option<String>>,
        owner_id: Option<Option<Uuid>>,
        key_metrics: Option<StringStringMap>,
    ) -> Result<ValueStreamStage, DomainError> {
        let mut stage = self
            .stage_repo
            .find_by_id(id)
            .await?
            .ok_or(DomainError::ValueStreamNotFound)?;
        let now = Utc::now();

        if let Some(seq) = sequence_order {
            if seq != stage.sequence_order
                && self
                    .stage_repo
                    .sequence_order_exists(stage.value_stream_id, seq, Some(id))
                    .await?
            {
                return Err(DomainError::InvalidTransition {
                    from: format!("sequence_order {seq}"),
                    to: "unique".to_string(),
                    entity: "ValueStreamStage".to_string(),
                });
            }
        }

        stage.update(
            name,
            sequence_order,
            input,
            output,
            description,
            objective_metrics,
            entry_criteria,
            exit_criteria,
            owner_id,
            key_metrics,
            now,
        );
        self.stage_repo.save(&stage).await
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
}
