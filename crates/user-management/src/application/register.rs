use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct RegisterInput {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AuthOutput {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub user: UserDto,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UserDto {
    pub id: uuid::Uuid,
    pub email: String,
    pub name: String,
    pub role: String,
    pub status: String,
}
