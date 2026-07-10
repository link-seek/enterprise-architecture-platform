use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct RefreshInput {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshOutput {
    pub access_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub user_id: uuid::Uuid,
    pub role: String,
}
