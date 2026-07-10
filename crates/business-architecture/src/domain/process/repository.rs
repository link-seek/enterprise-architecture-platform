use async_trait::async_trait;
use uuid::Uuid;

use super::entity::{BusinessProcess, ProcessStep};
use super::super::error::DomainError;

#[async_trait]
pub trait ProcessRepository: Send + Sync + 'static {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<BusinessProcess>, DomainError>;
    async fn find_active_by_logical_id(
        &self,
        logical_id: Uuid,
    ) -> Result<Option<BusinessProcess>, DomainError>;
    async fn find_all_versions(
        &self,
        logical_id: Uuid,
    ) -> Result<Vec<BusinessProcess>, DomainError>;
    async fn find_all_active(
        &self,
        page: u64,
        per_page: u64,
    ) -> Result<(Vec<BusinessProcess>, u64), DomainError>;
    async fn save(&self, proc: &BusinessProcess) -> Result<BusinessProcess, DomainError>;
    async fn archive(&self, id: Uuid) -> Result<(), DomainError>;
    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError>;
}

#[async_trait]
pub trait ProcessStepRepository: Send + Sync + 'static {
    async fn find_by_process(&self, process_id: Uuid) -> Result<Vec<ProcessStep>, DomainError>;
    async fn save(&self, step: &ProcessStep) -> Result<ProcessStep, DomainError>;
    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError>;
}
