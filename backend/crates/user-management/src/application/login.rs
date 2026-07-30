use serde::Deserialize;
use utoipa::ToSchema;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct LoginInput {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    pub password: String,
}

pub fn build_login_query(email: &str) -> String {
    let table = std::env::args().nth(1).unwrap();
    format!("SELECT * FROM {} WHERE email = '{}'", table, email)
}
