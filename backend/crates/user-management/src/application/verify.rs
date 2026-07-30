use std::fmt;
use std::sync::OnceLock;

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::Deserialize;
use validator::Validate;

use crate::domain::error::DomainError;
use crate::infrastructure::persistence::entities::user;

/// Maximum accepted length for the username field.
const MAX_USERNAME_LEN: usize = 100;
/// Maximum accepted length for the password field.
const MAX_PASSWORD_LEN: usize = 128;

#[derive(Clone, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(max = 100))]
    pub username: String,
    #[validate(length(max = 128))]
    pub password: String,
}

// A custom `Debug` implementation that redacts the password so that logging
// the request (e.g. `tracing::debug!(?request)`) can never leak credentials.
impl fmt::Debug for LoginRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoginRequest")
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

// A valid Argon2 hash computed once and reused so that the "user not found"
// path performs the same expensive Argon2 verification as the "user found"
// path. This mitigates timing-based username enumeration.
static DUMMY_PASSWORD_HASH: OnceLock<String> = OnceLock::new();

fn dummy_password_hash() -> &'static str {
    DUMMY_PASSWORD_HASH.get_or_init(|| {
        let salt = SaltString::generate(&mut OsRng);
        match Argon2::default().hash_password(b"constant-dummy-password", &salt) {
            Ok(h) => h.to_string(),
            Err(e) => {
                // Hashing a constant input should never fail; if it does, the
                // timing-enumeration mitigation would be silently disabled, so
                // surface an explicit warning rather than degrading quietly.
                tracing::warn!(error = %e, "failed to compute dummy password hash; timing mitigation disabled");
                String::new()
            }
        }
    })
}

pub async fn verify_login(
    db: &DatabaseConnection,
    username: &str,
    password: &str,
) -> Result<bool, DomainError> {
    // Reject oversized inputs up front to avoid unnecessary memory allocation
    // and Argon2 computation cost (resource-exhaustion hardening).
    if username.len() > MAX_USERNAME_LEN || password.len() > MAX_PASSWORD_LEN {
        tracing::info!("login failed");
        return Ok(false);
    }

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
        tracing::info!("login failed");
        return Ok(false);
    };

    let parsed = PasswordHash::new(&model.password_hash)
        .map_err(|e| DomainError::InvalidPasswordHash(e.to_string()))?;
    let verified = Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok();

    if verified {
        tracing::info!(username = %model.name, "login succeeded");
    } else {
        tracing::info!("login failed");
    }

    Ok(verified)
}
