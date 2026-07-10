use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::application::register::UserDto;

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LogoutInput {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

pub fn user_to_dto(user: &crate::domain::user::entity::User) -> UserDto {
    UserDto {
        id: user.id,
        email: user.email.clone(),
        name: user.name.clone(),
        role: format!("{:?}", user.role).to_lowercase(),
        status: format!("{:?}", user.status).to_lowercase(),
    }
}
