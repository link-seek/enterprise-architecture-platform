use chrono::{DateTime, Utc};
use shared_common::enums::SpaceVisibility;
use uuid::Uuid;

use crate::domain::error::DomainError;

/// A Space is the multi-tenant container for one enterprise's architecture.
/// Reuses the `organizations` table; soft-deleted via `deleted_at`.
/// `visibility` controls whether content is readable by anonymous/any user
/// (`Public`) or only by members/Admins (`Private`).
#[derive(Debug, Clone, PartialEq)]
pub struct Space {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub visibility: SpaceVisibility,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Space {
    pub fn create(
        id: Uuid,
        name: String,
        description: Option<String>,
        visibility: SpaceVisibility,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::SpaceNameEmpty);
        }
        Ok(Self {
            id,
            name: name.trim().to_owned(),
            description,
            visibility,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        })
    }

    pub fn rename(&mut self, name: String, now: DateTime<Utc>) -> Result<(), DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::SpaceNameEmpty);
        }
        self.name = name.trim().to_owned();
        self.updated_at = now;
        Ok(())
    }

    pub fn set_description(&mut self, description: Option<String>, now: DateTime<Utc>) {
        self.description = description;
        self.updated_at = now;
    }

    pub fn set_visibility(&mut self, visibility: SpaceVisibility, now: DateTime<Utc>) {
        self.visibility = visibility;
        self.updated_at = now;
    }

    pub fn archive(&mut self, now: DateTime<Utc>) {
        self.deleted_at = Some(now);
        self.updated_at = now;
    }
}

/// Membership of a user in a space.
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceMember {
    pub space_id: Uuid,
    pub user_id: Uuid,
    pub role: shared_common::enums::SpaceRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}