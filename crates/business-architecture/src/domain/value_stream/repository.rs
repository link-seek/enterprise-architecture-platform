use async_trait::async_trait;
use uuid::Uuid;

use super::entity::{ValueStream, ValueStreamStage};
use super::super::error::DomainError;

#[async_trait]
pub trait ValueStreamRepository: Send + Sync + 'static {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<ValueStream>, DomainError>;
    async fn find_active_by_logical_id(
        &self,
        logical_id: Uuid,
    ) -> Result<Option<ValueStream>, DomainError>;
    async fn find_all_versions(
        &self,
        logical_id: Uuid,
    ) -> Result<Vec<ValueStream>, DomainError>;
    async fn save(&self, vs: &ValueStream) -> Result<ValueStream, DomainError>;
    async fn archive(&self, id: Uuid) -> Result<(), DomainError>;
    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError>;
    async fn list_active(
        &self,
        page: u64,
        per_page: u64,
    ) -> Result<(Vec<ValueStream>, u64), DomainError>;
}

#[async_trait]
pub trait ValueStreamStageRepository: Send + Sync + 'static {
    async fn find_by_value_stream(
        &self,
        vs_id: Uuid,
    ) -> Result<Vec<ValueStreamStage>, DomainError>;
    async fn save(&self, stage: &ValueStreamStage) -> Result<ValueStreamStage, DomainError>;
    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError>;
}
