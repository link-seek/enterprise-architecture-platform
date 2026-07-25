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
    #[error("space not found")]
    SpaceNotFound,
    #[error("not a member of this space")]
    NotSpaceMember,
    #[error("only space editors can modify content")]
    NotSpaceEditor,
    #[error("only space owners can manage members")]
    NotSpaceOwner,
    #[error("space name cannot be empty")]
    SpaceNameEmpty,
    #[error("space quota exceeded: non-admin users may own at most one space")]
    SpaceQuotaExceeded,
    #[error("user is already a member of this space")]
    AlreadyMember,
    #[error("cannot remove the last owner of a space")]
    CannotRemoveLastOwner,
    #[error("invitation not found")]
    InvitationNotFound,
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
