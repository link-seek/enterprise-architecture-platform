use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("invalid email")]
    InvalidEmail,
    #[error("user not found")]
    UserNotFound,
    #[error("email already exists")]
    EmailExists,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("user is not active")]
    UserInactive,
    #[error("token expired")]
    TokenExpired,
    #[error("token revoked")]
    TokenRevoked,
    #[error("invalid authorization code")]
    InvalidAuthCode,
    #[error("pkce verification failed")]
    PkceFailed,
    #[error("database error: {0}")]
    Database(String),
}

impl From<sea_orm::DbErr> for DomainError {
    fn from(e: sea_orm::DbErr) -> Self {
        DomainError::Database(e.to_string())
    }
}
