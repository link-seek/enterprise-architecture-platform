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
    pub fn create(
        id: Uuid,
        space_id: Uuid,
        name: String,
        description: Option<String>,
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
            space_id,
        }
    }

    /// Set optional fields on a newly created ValueStream (builder-style).
    pub fn with_details(
        mut self,
        stakeholders: Option<StringVec>,
        triggering_event: Option<String>,
        end_deliverable: Option<String>,
        owner_id: Option<Uuid>,
        performance_metrics: Option<StringStringMap>,
    ) -> Self {
        if let Some(s) = stakeholders { self.stakeholders = s; }
        if let Some(t) = triggering_event { self.triggering_event = Some(t); }
        if let Some(e) = end_deliverable { self.end_deliverable = Some(e); }
        if let Some(o) = owner_id { self.owner_id = Some(o); }
        if let Some(p) = performance_metrics { self.performance_metrics = p; }
        self
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
            space_id: self.space_id,
        };

        Ok(new_vs)
    }

    /// Update mutable fields. Archived streams cannot be updated.
    pub fn update(
        &mut self,
        name: Option<String>,
        description: Option<Option<String>>,
        importance: Option<ValueStreamImportance>,
        stakeholders: Option<StringVec>,
        triggering_event: Option<Option<String>>,
        end_deliverable: Option<Option<String>>,
        owner_id: Option<Option<Uuid>>,
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
        if let Some(s) = stakeholders { self.stakeholders = s; }
        if let Some(t) = triggering_event { self.triggering_event = t; }
        if let Some(e) = end_deliverable { self.end_deliverable = e; }
        if let Some(o) = owner_id { self.owner_id = o; }
        if let Some(p) = performance_metrics { self.performance_metrics = p; }
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
    pub description: Option<String>,
    pub objective_metrics: StringStringMap,
    pub entry_criteria: Option<String>,
    pub exit_criteria: Option<String>,
    /// Business-level stage owner (who is accountable for this stage).
    /// This is a pure business attribute — it does NOT participate in
    /// write-permission checks (those follow the parent value stream's owner).
    pub owner_id: Option<Uuid>,
    pub key_metrics: StringStringMap,
    pub value_stream_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl ValueStreamStage {
    /// Create a new stage belonging to the given value stream.
    pub fn create(
        id: Uuid,
        value_stream_id: Uuid,
        name: String,
        sequence_order: i32,
        input: Option<String>,
        output: Option<String>,
        description: Option<String>,
        objective_metrics: Option<StringStringMap>,
        entry_criteria: Option<String>,
        exit_criteria: Option<String>,
        owner_id: Option<Uuid>,
        key_metrics: Option<StringStringMap>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            sequence_order,
            input,
            output,
            description,
            objective_metrics: objective_metrics.unwrap_or_default(),
            entry_criteria,
            exit_criteria,
            owner_id,
            key_metrics: key_metrics.unwrap_or_default(),
            value_stream_id,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    /// Update mutable fields of a stage.
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
        if let Some(m) = objective_metrics { self.objective_metrics = m; }
        if let Some(e) = entry_criteria { self.entry_criteria = e; }
        if let Some(x) = exit_criteria { self.exit_criteria = x; }
        if let Some(o) = owner_id { self.owner_id = o; }
        if let Some(m) = key_metrics { self.key_metrics = m; }
        self.updated_at = now;
    }

    /// Domain rule: within the same value stream, `sequence_order` must be
    /// unique. `siblings` is the list of other stages of the same value
    /// stream (excluding the stage being validated).
    pub fn ensure_sequence_order_unique(
        &self,
        siblings: &[ValueStreamStage],
    ) -> Result<(), DomainError> {
        if siblings
            .iter()
            .any(|s| s.sequence_order == self.sequence_order && s.id != self.id)
        {
            return Err(DomainError::Validation(format!(
                "sequence_order {} already exists in this value stream",
                self.sequence_order
            )));
        }
        Ok(())
    }

    /// Produce a deep copy of this stage for a new version of its value
    /// stream: new id, new `value_stream_id`, all business fields preserved.
    pub fn for_version(&self, new_id: Uuid, new_value_stream_id: Uuid) -> Self {
        Self {
            id: new_id,
            name: self.name.clone(),
            sequence_order: self.sequence_order,
            input: self.input.clone(),
            output: self.output.clone(),
            description: self.description.clone(),
            objective_metrics: self.objective_metrics.clone(),
            entry_criteria: self.entry_criteria.clone(),
            exit_criteria: self.exit_criteria.clone(),
            owner_id: self.owner_id,
            key_metrics: self.key_metrics.clone(),
            value_stream_id: new_value_stream_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: None,
        }
    }
}
