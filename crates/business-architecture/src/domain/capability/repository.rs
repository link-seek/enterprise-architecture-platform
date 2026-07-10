use async_trait::async_trait;
use uuid::Uuid;

use super::entity::BusinessCapability;
use super::super::error::DomainError;

#[async_trait]
pub trait CapabilityRepository: Send + Sync + 'static {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<BusinessCapability>, DomainError>;
    async fn save(&self, cap: &BusinessCapability) -> Result<BusinessCapability, DomainError>;
    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError>;
    async fn list_active(
        &self,
        page: u64,
        per_page: u64,
    ) -> Result<(Vec<BusinessCapability>, u64), DomainError>;
}
