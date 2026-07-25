use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::space::entity::{Space, SpaceMember};

#[async_trait]
pub trait SpaceRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Space>, DomainError>;
    /// All non-deleted spaces (public case-showcase listing).
    async fn find_all_public(&self) -> Result<Vec<Space>, DomainError>;
    async fn save(&self, space: &Space) -> Result<Space, DomainError>;
    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError>;
    /// Number of spaces owned by a user (for quota checks).
    async fn count_owned_by(&self, user_id: Uuid) -> Result<u64, DomainError>;
}

#[async_trait]
pub trait MembershipRepository: Send + Sync {
    async fn find_membership(&self, space_id: Uuid, user_id: Uuid) -> Result<Option<SpaceMember>, DomainError>;
    async fn list_members(&self, space_id: Uuid) -> Result<Vec<SpaceMember>, DomainError>;
    async fn add(&self, member: &SpaceMember) -> Result<SpaceMember, DomainError>;
    async fn remove(&self, space_id: Uuid, user_id: Uuid) -> Result<(), DomainError>;
    async fn count_owners(&self, space_id: Uuid) -> Result<u64, DomainError>;
}