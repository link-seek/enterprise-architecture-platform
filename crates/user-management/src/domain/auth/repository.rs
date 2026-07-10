use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::entity::{OAuthAuthorizationCode, RefreshToken};
use super::super::error::DomainError;

#[async_trait]
pub trait RefreshTokenRepository: Send + Sync + 'static {
    async fn save(&self, token: &RefreshToken) -> Result<(), DomainError>;
    async fn find_by_hash(&self, hash: &str) -> Result<Option<RefreshToken>, DomainError>;
    async fn revoke(&self, id: Uuid) -> Result<(), DomainError>;
    async fn revoke_all_for_user(&self, user_id: Uuid) -> Result<(), DomainError>;
}

#[async_trait]
pub trait AuthCodeRepository: Send + Sync + 'static {
    async fn save(&self, code: &OAuthAuthorizationCode) -> Result<(), DomainError>;
    async fn find_by_hash(&self, hash: &str) -> Result<Option<OAuthAuthorizationCode>, DomainError>;
    async fn mark_used(&self, id: Uuid) -> Result<(), DomainError>;
    async fn cleanup_expired(&self, before: DateTime<Utc>) -> Result<(), DomainError>;
}
