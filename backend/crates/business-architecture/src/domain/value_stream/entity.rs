use chrono::{DateTime, Utc};
use shared_common::enums::{LifecycleStatus, StageStatus, StageType, ValueStreamImportance};
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
    pub value_proposition: Option<String>,
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
            value_proposition: None,
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
            value_proposition: self.value_proposition.clone(),
        };

        Ok(new_vs)
    }

    /// Update mutable fields. Archived streams cannot be updated.
    pub fn update(
        &mut self,
        name: Option<String>,
        description: Option<Option<String>>,
        importance: Option<ValueStreamImportance>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.update_full(
            name,
            description,
            importance,
            None,
            None,
            None,
            now,
        )
    }

    /// Update mutable fields including value-stream metadata. Archived streams
    /// cannot be updated. Fields wrapped in `Option<Option<T>>` follow the
    /// "outer Some ⇒ apply, outer None ⇒ leave unchanged" convention.
    #[allow(clippy::too_many_arguments)]
    pub fn update_full(
        &mut self,
        name: Option<String>,
        description: Option<Option<String>>,
        importance: Option<ValueStreamImportance>,
        triggering_event: Option<Option<String>>,
        end_deliverable: Option<Option<String>>,
        value_proposition: Option<Option<String>>,
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
        if let Some(t) = triggering_event { self.triggering_event = t; }
        if let Some(e) = end_deliverable { self.end_deliverable = e; }
        if let Some(v) = value_proposition { self.value_proposition = v; }
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
    pub description: Option<String>,
    pub stage_type: StageType,
    pub status: StageStatus,
    pub owner_id: Option<Uuid>,
    pub objectives: StringVec,
    pub metrics: StringStringMap,
}

impl ValueStreamStage {
    /// Create a new stage. New stages start in the `Draft` status so the
    /// author can flesh out objectives/metrics before publishing.
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        id: Uuid,
        value_stream_id: Uuid,
        name: String,
        sequence_order: i32,
        stage_type: StageType,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if sequence_order < 1 {
            return Err(DomainError::InvalidStageOrder {
                order: sequence_order,
            });
        }
        if name.trim().is_empty() {
            return Err(DomainError::Validation(
                "stage name cannot be empty".to_string(),
            ));
        }
        Ok(Self {
            id,
            name,
            sequence_order,
            input: None,
            output: None,
            value_stream_id,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            description: None,
            stage_type,
            status: StageStatus::Draft,
            owner_id: None,
            objectives: StringVec::default(),
            metrics: StringStringMap::default(),
        })
    }

    /// Publish a draft stage (Draft → Active). Active stages can later be
    /// archived.
    pub fn publish(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status != StageStatus::Draft {
            return Err(DomainError::InvalidTransition {
                from: format!("{:?}", self.status),
                to: "Active".to_string(),
                entity: "ValueStreamStage".to_string(),
            });
        }
        self.status = StageStatus::Active;
        self.updated_at = now;
        Ok(())
    }

    /// Archive a stage (Draft/Active → Archived). Archived stages are
    /// immutable.
    pub fn archive(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status == StageStatus::Archived {
            return Err(DomainError::InvalidTransition {
                from: format!("{:?}", self.status),
                to: "Archived".to_string(),
                entity: "ValueStreamStage".to_string(),
            });
        }
        self.status = StageStatus::Archived;
        self.updated_at = now;
        Ok(())
    }

    /// Update mutable fields. Archived stages cannot be updated. Fields
    /// wrapped in `Option<Option<T>>` follow the "outer Some ⇒ apply, outer
    /// None ⇒ leave unchanged" convention.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        name: Option<String>,
        description: Option<Option<String>>,
        stage_type: Option<StageType>,
        input: Option<Option<String>>,
        output: Option<Option<String>>,
        owner_id: Option<Option<Uuid>>,
        objectives: Option<StringVec>,
        metrics: Option<StringStringMap>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if self.status == StageStatus::Archived {
            return Err(DomainError::CannotModifyArchived {
                entity: "ValueStreamStage".to_string(),
            });
        }
        if let Some(n) = name {
            if n.trim().is_empty() {
                return Err(DomainError::Validation(
                    "stage name cannot be empty".to_string(),
                ));
            }
            self.name = n;
        }
        if let Some(d) = description { self.description = d; }
        if let Some(t) = stage_type { self.stage_type = t; }
        if let Some(i) = input { self.input = i; }
        if let Some(o) = output { self.output = o; }
        if let Some(ow) = owner_id { self.owner_id = ow; }
        if let Some(ob) = objectives { self.objectives = ob; }
        if let Some(m) = metrics { self.metrics = m; }
        self.updated_at = now;
        Ok(())
    }

    /// Reorder this stage within its value stream.
    pub fn reorder(&mut self, new_order: i32, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status == StageStatus::Archived {
            return Err(DomainError::CannotModifyArchived {
                entity: "ValueStreamStage".to_string(),
            });
        }
        if new_order < 1 {
            return Err(DomainError::InvalidStageOrder { order: new_order });
        }
        self.sequence_order = new_order;
        self.updated_at = now;
        Ok(())
    }

    /// Produce a deep copy of this stage attached to a (new) value stream id
    /// and with a fresh id. Used when snapshotting stages into a new value
    /// stream version.
    pub fn snapshot_to(
        &self,
        new_id: Uuid,
        new_value_stream_id: Uuid,
        now: DateTime<Utc>,
    ) -> ValueStreamStage {
        ValueStreamStage {
            id: new_id,
            name: self.name.clone(),
            sequence_order: self.sequence_order,
            input: self.input.clone(),
            output: self.output.clone(),
            value_stream_id: new_value_stream_id,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            description: self.description.clone(),
            stage_type: self.stage_type,
            status: self.status,
            owner_id: self.owner_id,
            objectives: self.objectives.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

/// Validate that a set of stages forms a well-ordered linear flow within one
/// value stream: sequence orders are unique and contiguous starting at 1, and
/// every stage belongs to the same value stream. This is a soft check used by
/// the aggregate root before persisting.
pub fn validate_stage_flow(stages: &[ValueStreamStage], value_stream_id: Uuid) -> Result<(), DomainError> {
    if stages.is_empty() {
        return Ok(());
    }
    let mut orders: Vec<i32> = stages.iter().map(|s| s.sequence_order).collect();
    orders.sort_unstable();
    for window in orders.windows(2) {
        if window[0] == window[1] {
            return Err(DomainError::DuplicateStageOrder { order: window[0] });
        }
    }
    for (idx, expected) in orders.iter().enumerate() {
        let wanted = (idx + 1) as i32;
        if *expected != wanted {
            return Err(DomainError::InvalidStageOrder { order: *expected });
        }
    }
    for s in stages {
        if s.value_stream_id != value_stream_id {
            return Err(DomainError::Validation(format!(
                "stage {} does not belong to value stream {}",
                s.id, value_stream_id
            )));
        }
    }
    Ok(())
}
