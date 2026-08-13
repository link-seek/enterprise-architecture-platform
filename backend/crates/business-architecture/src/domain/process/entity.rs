use chrono::{DateTime, Utc};
use shared_common::enums::{AutomationLevel, LifecycleStatus, MaturityLevel};
use shared_common::value_objects::StringVec;
use uuid::Uuid;

use super::super::error::DomainError;

#[derive(Debug, Clone)]
pub struct BusinessProcess {
    pub id: Uuid,
    pub logical_id: Uuid,
    pub business_version: String,
    pub status: LifecycleStatus,
    pub name: String,
    pub description: String,
    /// Process-level overall inputs (JSON array). Step-level inputs live on
    /// `ProcessStep::required_inputs`.
    pub inputs: StringVec,
    /// Process-level overall outputs (JSON array). Step-level outputs live on
    /// `ProcessStep::produced_outputs`.
    pub outputs: StringVec,
    pub sla: Option<String>,
    pub cost_per_transaction: Option<f64>,
    pub cycle_time: Option<i64>,
    pub automation_level: Option<AutomationLevel>,
    pub maturity: Option<MaturityLevel>,
    pub owner_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub space_id: Uuid,
}

impl BusinessProcess {
    /// Lifecycle state machine: `Active → Deprecated` (compatibility window).
    /// A deprecated process remains valid for existing capability links while
    /// a newer version is live.
    pub fn deprecate(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status != LifecycleStatus::Active {
            return Err(DomainError::InvalidTransition {
                from: format!("{:?}", self.status),
                to: "Deprecated".to_string(),
                entity: "BusinessProcess".to_string(),
            });
        }
        self.status = LifecycleStatus::Deprecated;
        self.updated_at = now;
        Ok(())
    }

    /// Lifecycle state machine: `Active | Deprecated → Archived` (terminal).
    /// The `processArchive` mutation additionally restricts archival to the
    /// `Deprecated → Archived` edge so the compatibility window cannot be
    /// skipped.
    pub fn archive(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if !matches!(self.status, LifecycleStatus::Active | LifecycleStatus::Deprecated) {
            return Err(DomainError::InvalidTransition {
                from: format!("{:?}", self.status),
                to: "Archived".to_string(),
                entity: "BusinessProcess".to_string(),
            });
        }
        self.status = LifecycleStatus::Archived;
        self.updated_at = now;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ProcessStep {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub sequence_order: i32,
    pub business_rules: StringVec,
    pub required_inputs: StringVec,
    pub produced_outputs: StringVec,
    pub role_id: Option<Uuid>,
    pub process_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// A capability that references an old version of a process and must be
/// reviewed / re-anchored after a new version is published.
#[derive(Debug, Clone)]
pub struct AffectedProcessLink {
    pub capability_id: Uuid,
    pub capability_name: String,
    pub old_version: String,
    pub new_version: String,
}

/// Result of `publish_new_version`: the newly created active version plus the
/// list of capability links that now point at the deprecated old version.
#[derive(Debug, Clone)]
pub struct PublishVersionResult {
    pub new_process: BusinessProcess,
    pub affected_links: Vec<AffectedProcessLink>,
}
