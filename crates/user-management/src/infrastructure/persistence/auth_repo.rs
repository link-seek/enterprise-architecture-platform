use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::domain::auth::entity::{OAuthAuthorizationCode, RefreshToken};
use crate::domain::auth::repository::{AuthCodeRepository, RefreshTokenRepository};
use crate::domain::error::DomainError;
use crate::infrastructure::persistence::entities::{oauth_authorization_code, refresh_token};

impl From<refresh_token::Model> for RefreshToken {
    fn from(m: refresh_token::Model) -> Self {
        RefreshToken {
            id: m.id,
            user_id: m.user_id,
            token_hash: m.token_hash,
            expires_at: m.expires_at,
            revoked_at: m.revoked_at,
            created_at: m.created_at,
        }
    }
}

impl From<oauth_authorization_code::Model> for OAuthAuthorizationCode {
    fn from(m: oauth_authorization_code::Model) -> Self {
        OAuthAuthorizationCode {
            id: m.id,
            client_id: m.client_id,
            user_id: m.user_id,
            code_hash: m.code_hash,
            redirect_uri: m.redirect_uri,
            code_challenge: m.code_challenge,
            code_challenge_method: m.code_challenge_method,
            expires_at: m.expires_at,
            used: m.used,
            created_at: m.created_at,
        }
    }
}

pub struct SeaOrmRefreshTokenRepo {
    db: DatabaseConnection,
}

impl SeaOrmRefreshTokenRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl RefreshTokenRepository for SeaOrmRefreshTokenRepo {
    async fn save(&self, token: &RefreshToken) -> Result<(), DomainError> {
        let active = refresh_token::ActiveModel {
            id: Set(token.id),
            user_id: Set(token.user_id),
            token_hash: Set(token.token_hash.clone()),
            expires_at: Set(token.expires_at),
            revoked_at: Set(token.revoked_at),
            created_at: Set(token.created_at),
        };
        active.insert(&self.db).await?;
        Ok(())
    }

    async fn find_by_hash(&self, hash: &str) -> Result<Option<RefreshToken>, DomainError> {
        let model = refresh_token::Entity::find()
            .filter(refresh_token::Column::TokenHash.eq(hash))
            .one(&self.db)
            .await?;
        Ok(model.map(Into::into))
    }

    async fn revoke(&self, id: Uuid) -> Result<(), DomainError> {
        let model = refresh_token::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(DomainError::TokenRevoked)?;

        let mut active: refresh_token::ActiveModel = model.into();
        active.revoked_at = Set(Some(Utc::now()));
        active.update(&self.db).await?;

        Ok(())
    }

    async fn revoke_all_for_user(&self, user_id: Uuid) -> Result<(), DomainError> {
        let models = refresh_token::Entity::find()
            .filter(refresh_token::Column::UserId.eq(user_id))
            .filter(refresh_token::Column::RevokedAt.is_null())
            .all(&self.db)
            .await?;

        let now = Utc::now();
        for model in models {
            let mut active: refresh_token::ActiveModel = model.into();
            active.revoked_at = Set(Some(now));
            active.update(&self.db).await?;
        }

        Ok(())
    }
}

pub struct SeaOrmAuthCodeRepo {
    db: DatabaseConnection,
}

impl SeaOrmAuthCodeRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AuthCodeRepository for SeaOrmAuthCodeRepo {
    async fn save(&self, code: &OAuthAuthorizationCode) -> Result<(), DomainError> {
        let active = oauth_authorization_code::ActiveModel {
            id: Set(code.id),
            client_id: Set(code.client_id.clone()),
            user_id: Set(code.user_id),
            code_hash: Set(code.code_hash.clone()),
            redirect_uri: Set(code.redirect_uri.clone()),
            code_challenge: Set(code.code_challenge.clone()),
            code_challenge_method: Set(code.code_challenge_method.clone()),
            expires_at: Set(code.expires_at),
            used: Set(code.used),
            created_at: Set(code.created_at),
        };
        active.insert(&self.db).await?;
        Ok(())
    }

    async fn find_by_hash(&self, hash: &str) -> Result<Option<OAuthAuthorizationCode>, DomainError> {
        let model = oauth_authorization_code::Entity::find()
            .filter(oauth_authorization_code::Column::CodeHash.eq(hash))
            .one(&self.db)
            .await?;
        Ok(model.map(Into::into))
    }

    async fn mark_used(&self, id: Uuid) -> Result<(), DomainError> {
        let model = oauth_authorization_code::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(DomainError::InvalidAuthCode)?;

        let mut active: oauth_authorization_code::ActiveModel = model.into();
        active.used = Set(true);
        active.update(&self.db).await?;

        Ok(())
    }

    async fn cleanup_expired(&self, before: DateTime<Utc>) -> Result<(), DomainError> {
        let expired = oauth_authorization_code::Entity::find()
            .filter(oauth_authorization_code::Column::ExpiresAt.lte(before))
            .all(&self.db)
            .await?;

        for model in expired {
            let active: oauth_authorization_code::ActiveModel = model.into();
            active.delete(&self.db).await?;
        }

        Ok(())
    }
}
