use std::sync::OnceLock;

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
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

// A valid Argon2 hash computed once and reused so that the "user not found"
// path performs the same expensive Argon2 verification as the "user found"
// path. This mitigates timing-based username enumeration.
static DUMMY_PASSWORD_HASH: OnceLock<String> = OnceLock::new();

fn dummy_password_hash() -> &'static str {
    DUMMY_PASSWORD_HASH.get_or_init(|| {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(b"constant-dummy-password", &salt)
            .map(|h| h.to_string())
            .unwrap_or_default()
    })
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
        // Mitigate timing-based username enumeration: perform a dummy Argon2
        // verification so the not-found path takes comparable time to the
        // found path, then return a uniform failure.
        if let Ok(parsed) = PasswordHash::new(dummy_password_hash()) {
            let _ = Argon2::default().verify_password(password.as_bytes(), &parsed);
        }
        tracing::info!("login failed: user not found");
        return Ok(false);
    };

    let parsed = PasswordHash::new(&model.password_hash)
        .map_err(|e| DomainError::InvalidPasswordHash(e.to_string()))?;
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
