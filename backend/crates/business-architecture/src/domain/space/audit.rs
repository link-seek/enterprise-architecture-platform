use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::error::DomainError;

/// A newtype wrapper for the `action` field of a space audit log. Constraining
/// the value to known variants prevents typos (e.g. `"visibility_chaged"`) from
/// being persisted while still serializing to a plain string in the database,
/// so no schema migration is required.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceAuditAction(String);

impl SpaceAuditAction {
    pub fn visibility_changed() -> Self {
        Self("visibility_changed".to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for SpaceAuditAction {
    type Error = DomainError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "visibility_changed" => Ok(Self::visibility_changed()),
            other => Err(DomainError::Validation(format!(
                "unknown space audit action: {other}"
            ))),
        }
    }
}

impl std::fmt::Display for SpaceAuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An auditable space-level operation. The initial scope is visibility changes
/// (`action = "visibility_changed"`), recorded by `spaceSetVisibility`. The
/// shape is intentionally generic so future operations can reuse it without a
/// schema migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceAuditLog {
    pub id: Uuid,
    pub space_id: Uuid,
    pub actor_id: Uuid,
    pub action: SpaceAuditAction,
    pub from_value: Option<String>,
    pub to_value: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl SpaceAuditLog {
    pub fn visibility_changed(
        id: Uuid,
        space_id: Uuid,
        actor_id: Uuid,
        from: &str,
        to: &str,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            space_id,
            actor_id,
            action: SpaceAuditAction::visibility_changed(),
            from_value: Some(from.to_owned()),
            to_value: Some(to.to_owned()),
            created_at: now,
        }
    }
}
