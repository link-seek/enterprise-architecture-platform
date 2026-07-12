use chrono::{DateTime, Utc};
use shared_common::enums::{UserRole, UserStatus};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub role: UserRole,
    pub status: UserStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(email: String, name: String, password_hash: String, role: UserRole) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            email,
            name,
            password_hash,
            role,
            status: UserStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn can_modify(&self, actor_id: Uuid, actor_role: UserRole) -> bool {
        actor_role.is_admin() || self.id == actor_id
    }

    pub fn set_role(&mut self, new_role: UserRole) {
        self.role = new_role;
        self.updated_at = Utc::now();
    }
}
