use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::error::DomainError;

/// A newtype wrapper for the `action` field of a space audit log. Constraining
/// the value to known variants prevents typos (e.g. `"visibility_chaged"`) from
/// being persisted while still serializing to a plain string in the database,
/// so no schema migration is required.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

impl AsRef<str> for SpaceAuditAction {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// An auditable space-level operation. The initial scope is visibility changes
/// (`action = "visibility_changed"`), recorded by `spaceSetVisibility`. The
/// shape is intentionally generic so future operations can reuse it without a
/// schema migration.
///
/// Fields are `pub(crate)` so that the persistence layer (same crate) can
/// reconstruct a log from a database row, while external crates must use the
/// `visibility_changed` factory — which enforces the invariant that a
/// visibility change always carries both `from_value` and `to_value`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceAuditLog {
    pub(crate) id: Uuid,
    pub(crate) space_id: Uuid,
    pub(crate) actor_id: Uuid,
    pub(crate) action: SpaceAuditAction,
    pub(crate) from_value: Option<String>,
    pub(crate) to_value: Option<String>,
    pub(crate) created_at: DateTime<Utc>,
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

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn space_id(&self) -> Uuid {
        self.space_id
    }

    pub fn actor_id(&self) -> Uuid {
        self.actor_id
    }

    pub fn action(&self) -> &SpaceAuditAction {
        &self.action
    }

    pub fn from_value(&self) -> Option<&str> {
        self.from_value.as_deref()
    }

    pub fn to_value(&self) -> Option<&str> {
        self.to_value.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
