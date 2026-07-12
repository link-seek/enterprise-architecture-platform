use chrono::{DateTime, Utc};
use shared_common::enums::{LifecycleStatus, ValueStreamImportance};
use shared_common::value_objects::{StringStringMap, StringVec};
use uuid::Uuid;

use super::super::error::DomainError;

// ============================================================================
// ValueStream Aggregate Root
// ============================================================================

#[derive(Debug, Clone)]
pub struct ValueStream {
    pub id: Uuid,
    pub logical_id: Uuid,
    pub business_version: String,
    pub status: LifecycleStatus,
    pub name: String,
    pub description: String,
    pub triggering_event: Option<String>,
    pub end_deliverable: Option<String>,
    pub owner_id: Option<Uuid>,
    pub importance: ValueStreamImportance,
    pub stakeholders: StringVec,
    pub performance_metrics: StringStringMap,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl ValueStream {
    /// Create a new ValueStream with the given attributes.
    /// The logical_id is set to the new id by default (first version).
    pub fn create(
        id: Uuid,
        name: String,
        description: String,
        business_version: String,
        importance: ValueStreamImportance,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            logical_id: id, // First version: logical_id = id
            business_version,
            status: LifecycleStatus::Active,
            name,
            description,
            triggering_event: None,
            end_deliverable: None,
            owner_id: None,
            importance,
            stakeholders: StringVec::default(),
            performance_metrics: StringStringMap::default(),
            created_by: None,
            updated_by: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    /// Archive this value stream. Only active streams can be archived.
    /// This is a lifecycle state transition: Active → Archived (one-way).
    pub fn archive(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status != LifecycleStatus::Active {
            return Err(DomainError::InvalidTransition {
                from: format!("{:?}", self.status),
                to: "Archived".to_string(),
                entity: "ValueStream".to_string(),
            });
        }
        self.status = LifecycleStatus::Archived;
        self.updated_at = now;
        Ok(())
    }

    /// Create a new version of this value stream.
    /// The current version is archived, and a new version is returned
    /// with the same logical_id but a new id and business_version.
    pub fn create_new_version(
        &mut self,
        new_id: Uuid,
        new_version: String,
        new_name: String,
        new_description: String,
        now: DateTime<Utc>,
    ) -> Result<ValueStream, DomainError> {
        // Must be active to create a new version
        if self.status != LifecycleStatus::Active {
            return Err(DomainError::InvalidTransition {
                from: format!("{:?}", self.status),
                to: "Archived (for versioning)".to_string(),
                entity: "ValueStream".to_string(),
            });
        }

        // Archive current version
        self.archive(now)?;

        // Create new version with same logical_id
        let new_vs = ValueStream {
            id: new_id,
            logical_id: self.logical_id, // Same logical entity
            business_version: new_version,
            status: LifecycleStatus::Active,
            name: new_name,
            description: new_description,
            triggering_event: self.triggering_event.clone(),
            end_deliverable: self.end_deliverable.clone(),
            owner_id: self.owner_id,
            importance: self.importance,
            stakeholders: self.stakeholders.clone(),
            performance_metrics: self.performance_metrics.clone(),
            created_by: None,
            updated_by: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };

        Ok(new_vs)
    }

    /// Update mutable fields. Archived streams cannot be updated.
    pub fn update(
        &mut self,
        name: Option<String>,
        description: Option<String>,
        importance: Option<ValueStreamImportance>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if self.status != LifecycleStatus::Active {
            return Err(DomainError::CannotModifyArchived {
                entity: "ValueStream".to_string(),
            });
        }
        if let Some(n) = name { self.name = n; }
        if let Some(d) = description { self.description = d; }
        if let Some(i) = importance { self.importance = i; }
        self.updated_at = now;
        Ok(())
    }

    /// Check if this is the active version among its versions.
    pub fn is_active(&self) -> bool {
        self.status == LifecycleStatus::Active
    }
}

#[derive(Debug, Clone)]
pub struct ValueStreamStage {
    pub id: Uuid,
    pub name: String,
    pub sequence_order: i32,
    pub input: Option<String>,
    pub output: Option<String>,
    pub value_stream_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
