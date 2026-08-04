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
    pub description: Option<String>,
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
    pub space_id: Uuid,
}

impl ValueStream {
    /// Create a new ValueStream with the given attributes.
    /// The logical_id is set to the new id by default (first version).
    /// `owner_id` is the creating user; the four optional fields express the
    /// "triggering event → final deliverable" value-realisation chain.
    pub fn create(
        id: Uuid,
        space_id: Uuid,
        name: String,
        description: Option<String>,
        business_version: String,
        importance: ValueStreamImportance,
        owner_id: Option<Uuid>,
        triggering_event: Option<String>,
        end_deliverable: Option<String>,
        stakeholders: StringVec,
        performance_metrics: StringStringMap,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            logical_id: id, // First version: logical_id = id
            business_version,
            status: LifecycleStatus::Active,
            name,
            description,
            triggering_event,
            end_deliverable,
            owner_id,
            importance,
            stakeholders,
            performance_metrics,
            created_by: owner_id,
            updated_by: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            space_id,
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
    /// Ownership is inherited by the new version.
    pub fn create_new_version(
        &mut self,
        new_id: Uuid,
        new_version: String,
        new_name: String,
        new_description: Option<String>,
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

        // Create new version with same logical_id and inherited ownership
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
            created_by: self.owner_id,
            updated_by: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            space_id: self.space_id,
        };

        Ok(new_vs)
    }

    /// Update mutable fields. Archived streams cannot be updated.
    /// `owner_id` is used only for ownership transfer (not a general edit).
    pub fn update(
        &mut self,
        name: Option<String>,
        description: Option<Option<String>>,
        importance: Option<ValueStreamImportance>,
        owner_id: Option<Uuid>,
        triggering_event: Option<Option<String>>,
        end_deliverable: Option<Option<String>>,
        stakeholders: Option<StringVec>,
        performance_metrics: Option<StringStringMap>,
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
        if let Some(o) = owner_id { self.owner_id = Some(o); }
        if let Some(t) = triggering_event { self.triggering_event = t; }
        if let Some(e) = end_deliverable { self.end_deliverable = e; }
        if let Some(s) = stakeholders { self.stakeholders = s; }
        if let Some(p) = performance_metrics { self.performance_metrics = p; }
        self.updated_at = now;
        Ok(())
    }

    /// Transfer ownership to a new user. Only the current owner or admin
    /// may call this (the caller is responsible for enforcing that).
    pub fn transfer_ownership(&mut self, new_owner_id: Uuid, now: DateTime<Utc>) {
        self.owner_id = Some(new_owner_id);
        self.updated_at = now;
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
    /// Free-form stage description.
    pub description: Option<String>,
    /// Objective metrics: metric name → target value (e.g. "设计款式数" → "≥20").
    pub objective_metrics: StringStringMap,
    /// Entry gate criteria (free text).
    pub entry_criteria: Option<String>,
    /// Exit gate criteria (free text).
    pub exit_criteria: Option<String>,
    /// Stage owner (business semantics: who is responsible for this stage).
    /// NOT used for permission decisions — write permission follows the parent
    /// value stream's owner.
    pub owner_id: Option<Uuid>,
    /// Key metrics: metric name → current/actual value.
    pub key_metrics: StringStringMap,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl ValueStreamStage {
    /// Create a new stage. `sequence_order` uniqueness within the value stream
    /// is enforced by the service/repo layer (it needs to query siblings).
    pub fn create(
        id: Uuid,
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
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            sequence_order,
            input,
            output,
            value_stream_id,
            description,
            objective_metrics,
            entry_criteria,
            exit_criteria,
            owner_id,
            key_metrics,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    /// Update mutable fields.
    pub fn update(
        &mut self,
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
        now: DateTime<Utc>,
    ) {
        if let Some(n) = name { self.name = n; }
        if let Some(s) = sequence_order { self.sequence_order = s; }
        if let Some(i) = input { self.input = i; }
        if let Some(o) = output { self.output = o; }
        if let Some(d) = description { self.description = d; }
        if let Some(om) = objective_metrics { self.objective_metrics = om; }
        if let Some(ec) = entry_criteria { self.entry_criteria = ec; }
        if let Some(xc) = exit_criteria { self.exit_criteria = xc; }
        if let Some(oid) = owner_id { self.owner_id = oid; }
        if let Some(km) = key_metrics { self.key_metrics = km; }
        self.updated_at = now;
    }

    /// Create a copy of this stage attached to a different value stream
    /// (used when creating a new version). Gets a new id but preserves
    /// `sequence_order` and all field values.
    pub fn clone_for_new_version(&self, new_id: Uuid, new_value_stream_id: Uuid, now: DateTime<Utc>) -> Self {
        Self {
            id: new_id,
            name: self.name.clone(),
            sequence_order: self.sequence_order,
            input: self.input.clone(),
            output: self.output.clone(),
            value_stream_id: new_value_stream_id,
            description: self.description.clone(),
            objective_metrics: self.objective_metrics.clone(),
            entry_criteria: self.entry_criteria.clone(),
            exit_criteria: self.exit_criteria.clone(),
            owner_id: self.owner_id,
            key_metrics: self.key_metrics.clone(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }
}
