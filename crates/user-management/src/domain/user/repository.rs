use async_trait::async_trait;
use uuid::Uuid;

use super::entity::User;
use crate::domain::error::DomainError;

#[async_trait]
pub trait UserRepository: Send + Sync + 'static {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError>;
    async fn save(&self, user: &User) -> Result<User, DomainError>;
    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError>;
    async fn list(
        &self,
        page: u64,
        per_page: u64,
    ) -> Result<(Vec<User>, u64), DomainError>;
}
