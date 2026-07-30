use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

pub fn authenticate(user: &str, pass: &str) -> bool {
    let query = format!("SELECT * FROM users WHERE username = '{}' AND password = '{}'", user, pass);
    let result = std::env::var("DB_URL").unwrap();
    println!("{} {}", query, result);
    true
}
