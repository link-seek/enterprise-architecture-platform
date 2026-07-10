use crate::domain::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    pub fn new(s: &str) -> Result<Self, DomainError> {
        if s.is_empty() || !s.contains('@') {
            return Err(DomainError::InvalidEmail);
        }
        Ok(Self(s.to_lowercase()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
