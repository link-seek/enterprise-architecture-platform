use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use jsonwebtoken::{decode, DecodingKey, Validation};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub user_id: uuid::Uuid,
    pub role: String,
}

pub async fn jwt_auth_middleware(
    jwt_secret: String,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    match auth_header {
        Some(token) => {
            let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
            validation.validate_exp = true;

            match decode::<Claims>(token, &DecodingKey::from_secret(jwt_secret.as_bytes()), &validation) {
                Ok(data) => {
                    req.extensions_mut().insert(data.claims);
                    Ok(next.run(req).await)
                }
                Err(_) => Err(StatusCode::UNAUTHORIZED),
            }
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
