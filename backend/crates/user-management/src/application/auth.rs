use argon2::password_hash::{PasswordHash, PasswordVerifier};
use argon2::Argon2;

use crate::domain::error::DomainError;
use crate::domain::user::repository::UserRepository;

/// Verifies credentials using a parameterized query and constant-time
/// password hash verification.
///
/// Returns `Ok(true)` when the credentials match, `Ok(false)` otherwise.
/// Errors are propagated via `DomainError` instead of panicking.
pub async fn authenticate(
    repo: &dyn UserRepository,
    email: &str,
    password: &str,
) -> Result<bool, DomainError> {
    let user = match repo.find_by_email(email).await? {
        Some(user) => user,
        None => return Ok(false),
    };

    let parsed = match PasswordHash::new(&user.password_hash) {
        Ok(hash) => hash,
        Err(_) => return Ok(false),
    };

    let verified = Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok();

    Ok(verified)
}
