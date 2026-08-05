use chrono::Utc;
use shared_common::value_objects::StringStringMap;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::value_stream::entity::ValueStream;
use crate::domain::value_stream::repository::ValueStreamRepository;

/// Application Service for ValueStream.
/// Thin orchestration layer: coordinates domain objects and transactions.
/// No business logic here — all rules live in the domain model.
pub struct ValueStreamService<R: ValueStreamRepository> {
    repo: R,
}

impl<R: ValueStreamRepository> ValueStreamService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Create a new value stream (first version).
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        space_id: Uuid,
        name: String,
        description: Option<String>,
        business_version: String,
        importance: shared_common::enums::ValueStreamImportance,
        stakeholders: Option<shared_common::value_objects::StringVec>,
        triggering_event: Option<String>,
        end_deliverable: Option<String>,
        owner_id: Option<Uuid>,
        performance_metrics: Option<StringStringMap>,
    ) -> Result<ValueStream, DomainError> {
        let id = Uuid::now_v7();
        let now = Utc::now();
        let vs = ValueStream::create(id, space_id, name, description, business_version, importance, now)
            .with_details(stakeholders, triggering_event, end_deliverable, owner_id, performance_metrics);
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
    /// with the same logical_id. The current version's stages are copied to
    /// the new version. The whole operation (archive + insert + stage copy)
    /// runs inside a single transaction.
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

        // Copy stages (new id, new value_stream_id, fields preserved).
        let stages = self.repo.find_stages_by_value_stream(current_id).await?;
        let new_stages: Vec<_> = stages
            .iter()
            .map(|s| s.for_version(Uuid::now_v7(), new_id))
            .collect();

        // Persist atomically: archive current + insert new version + copy stages.
        self.repo
            .save_version_atomic(&current, &new_vs, &new_stages)
            .await?;
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
        stakeholders: Option<shared_common::value_objects::StringVec>,
        triggering_event: Option<Option<String>>,
        end_deliverable: Option<Option<String>>,
        owner_id: Option<Option<Uuid>>,
        performance_metrics: Option<StringStringMap>,
    ) -> Result<ValueStream, DomainError> {
        let mut vs = self.repo.find_by_id(id).await?.ok_or(DomainError::ValueStreamNotFound)?;
        let now = Utc::now();
        vs.update(name, description, importance, stakeholders, triggering_event, end_deliverable, owner_id, performance_metrics, now)?; // Domain rule: archived cannot be updated
        self.repo.save(&vs).await
    }

    /// Transfer ownership of a value stream to another user.
    /// Caller is expected to have already verified that the actor is the
    /// current owner (or an admin) and that `new_owner_id` belongs to the
    /// same space.
    pub async fn transfer_ownership(
        &self,
        id: Uuid,
        new_owner_id: Uuid,
    ) -> Result<ValueStream, DomainError> {
        let mut vs = self.repo.find_by_id(id).await?.ok_or(DomainError::ValueStreamNotFound)?;
        if vs.status != shared_common::enums::LifecycleStatus::Active {
            return Err(DomainError::CannotModifyArchived {
                entity: "ValueStream".to_string(),
            });
        }
        vs.owner_id = Some(new_owner_id);
        vs.updated_at = Utc::now();
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
}
