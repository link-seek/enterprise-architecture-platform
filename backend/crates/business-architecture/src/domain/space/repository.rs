use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::error::DomainError;
use crate::domain::space::audit::SpaceAuditLog;
use crate::domain::space::entity::{Space, SpaceMember};

#[async_trait]
pub trait SpaceRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Space>, DomainError>;
    /// All non-deleted public spaces (anonymous case-showcase listing).
    async fn find_all_public(&self) -> Result<Vec<Space>, DomainError>;
    /// All non-deleted spaces regardless of visibility (admin listing).
    async fn find_all_non_deleted(&self) -> Result<Vec<Space>, DomainError>;
    /// Spaces visible to a user: all non-deleted public spaces plus non-deleted
    /// private spaces the user is a member of. Used for the authenticated
    /// space listing.
    async fn find_visible_for_user(&self, user_id: Uuid) -> Result<Vec<Space>, DomainError>;
    async fn save(&self, space: &Space) -> Result<Space, DomainError>;
    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError>;
    /// Number of spaces owned by a user (for quota checks). Counts all owned
    /// spaces including archived ones — archiving does not release quota.
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

/// Records auditable space-level operations. Recording is best-effort at the
/// service layer: a failure here should not block the audited operation.
#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    async fn record(&self, log: &SpaceAuditLog) -> Result<(), DomainError>;
    /// List audit logs for a space, ordered by `created_at` descending (most
    /// recent first), capped at `limit` (default 200) with `offset` for
    /// pagination.
    async fn list_for_space(
        &self,
        space_id: Uuid,
        limit: Option<u64>,
        offset: u64,
    ) -> Result<Vec<SpaceAuditLog>, DomainError>;
}
