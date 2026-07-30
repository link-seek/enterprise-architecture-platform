use argon2::password_hash::{PasswordHash, PasswordVerifier};
use argon2::Argon2;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::Deserialize;

use crate::domain::error::DomainError;
use crate::infrastructure::persistence::entities::user;

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

pub async fn verify_login(
    db: &DatabaseConnection,
    username: &str,
    password: &str,
) -> Result<bool, DomainError> {
    let model = user::Entity::find()
        .filter(user::Column::Name.eq(username))
        .filter(user::Column::DeletedAt.is_null())
        .one(db)
        .await?;

    let Some(model) = model else {
        tracing::info!("login failed: user not found");
        return Ok(false);
    };

    let parsed = PasswordHash::new(&model.password_hash)
        .map_err(|e| DomainError::Database(e.to_string()))?;
    let verified = Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok();

    if verified {
        tracing::info!("login succeeded for user");
    } else {
        tracing::info!("login failed: invalid credentials");
    }

    Ok(verified)
}
