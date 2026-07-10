use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct AuthorizeInput {
    #[validate(length(min = 1))]
    pub client_id: String,
    #[validate(length(min = 1))]
    pub redirect_uri: String,
    #[validate(length(min = 1))]
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub state: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct TokenInput {
    #[validate(length(min = 1))]
    pub grant_type: String,
    #[validate(length(min = 1))]
    pub code: String,
    #[validate(length(min = 1))]
    pub client_id: String,
    #[validate(length(min = 1))]
    pub redirect_uri: String,
    #[validate(length(min = 1))]
    pub code_verifier: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenOutput {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}
