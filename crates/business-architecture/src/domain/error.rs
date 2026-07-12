use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("capability not found")]
    CapabilityNotFound,
    #[error("process not found")]
    ProcessNotFound,
    #[error("value stream not found")]
    ValueStreamNotFound,
    #[error("process version not found")]
    ProcessVersionNotFound,
    #[error("cannot reference archived process")]
    CannotReferenceArchived,
    #[error("only owner or admin can modify")]
    NotOwner,
    #[error("invalid lifecycle transition: {from} → {to} on {entity}")]
    InvalidTransition {
        from: String,
        to: String,
        entity: String,
    },
    #[error("cannot modify archived {entity}")]
    CannotModifyArchived {
        entity: String,
    },
    #[error("validation error: {0}")]
    Validation(String),
    #[error("semver error: {0}")]
    Semver(String),
    #[error("database error: {0}")]
    Database(String),
}

impl From<sea_orm::DbErr> for DomainError {
    fn from(e: sea_orm::DbErr) -> Self {
        DomainError::Database(e.to_string())
    }
}

impl From<semver::Error> for DomainError {
    fn from(e: semver::Error) -> Self {
        DomainError::Semver(e.to_string())
    }
}
