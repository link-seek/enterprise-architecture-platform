use chrono::Utc;
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
    pub async fn create(
        &self,
        name: String,
        description: String,
        business_version: String,
        importance: shared_common::enums::ValueStreamImportance,
    ) -> Result<ValueStream, DomainError> {
        let id = Uuid::now_v7();
        let now = Utc::now();
        let vs = ValueStream::create(id, name, description, business_version, importance, now);
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
    /// with the same logical_id.
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
        let description = new_description.unwrap_or_else(|| current.description.clone());

        // Domain rule: archive current, create new version with same logical_id
        let new_vs = current.create_new_version(new_id, new_version, name, description, now)?;

        // Persist: save archived current, then save new version
        self.repo.save(&current).await?;
        self.repo.save(&new_vs).await
    }

    /// Update mutable fields of an active value stream.
    pub async fn update(
        &self,
        id: Uuid,
        name: Option<String>,
        description: Option<String>,
        importance: Option<shared_common::enums::ValueStreamImportance>,
    ) -> Result<ValueStream, DomainError> {
        let mut vs = self.repo.find_by_id(id).await?.ok_or(DomainError::ValueStreamNotFound)?;
        let now = Utc::now();
        vs.update(name, description, importance, now)?; // Domain rule: archived cannot be updated
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
