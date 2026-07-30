use chrono::{DateTime, Utc};
use uuid::Uuid;

/// An auditable space-level operation. The initial scope is visibility changes
/// (`action = "visibility_changed"`), recorded by `spaceSetVisibility`. The
/// shape is intentionally generic so future operations can reuse it without a
/// schema migration.
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceAuditLog {
    pub id: Uuid,
    pub space_id: Uuid,
    pub actor_id: Uuid,
    pub action: String,
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
            action: "visibility_changed".to_owned(),
            from_value: Some(from.to_owned()),
            to_value: Some(to.to_owned()),
            created_at: now,
        }
    }
}