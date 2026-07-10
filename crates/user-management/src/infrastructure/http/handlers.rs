use std::sync::Arc;

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sea_orm::DatabaseConnection;
use sha2::{Digest, Sha256};
use shared_common::enums::{UserRole, UserStatus};
use uuid::Uuid;
use validator::Validate;

use crate::application::login::LoginInput;
use crate::application::oauth::{AuthorizeInput, TokenInput, TokenOutput};
use crate::application::register::{AuthOutput, RegisterInput, UserDto};
use crate::application::token::{Claims, RefreshInput, RefreshOutput};
use crate::domain::auth::entity::{OAuthAuthorizationCode, RefreshToken};
use crate::domain::auth::repository::{AuthCodeRepository, RefreshTokenRepository};
use crate::domain::error::DomainError;
use crate::domain::user::entity::User;
use crate::domain::user::repository::UserRepository;
use crate::infrastructure::http::dto::{user_to_dto, ErrorResponse, LogoutInput};
use crate::infrastructure::persistence::auth_repo::{SeaOrmAuthCodeRepo, SeaOrmRefreshTokenRepo};
use crate::infrastructure::persistence::user_repo::SeaOrmUserRepo;

pub struct OAuthClientConfig {
    pub client_id: String,
    pub redirect_uris: Vec<String>,
}

pub struct AuthService {
    db: DatabaseConnection,
    jwt_secret: String,
    jwt_expires_in: u64,
    refresh_expires_in: u64,
    oauth_clients: Vec<OAuthClientConfig>,
}

impl AuthService {
    pub fn new(
        db: DatabaseConnection,
        jwt_secret: String,
        jwt_expires_in: u64,
        refresh_expires_in: u64,
        oauth_clients: Vec<OAuthClientConfig>,
    ) -> Self {
        Self {
            db,
            jwt_secret,
            jwt_expires_in,
            refresh_expires_in,
            oauth_clients,
        }
    }

    fn user_repo(&self) -> SeaOrmUserRepo {
        SeaOrmUserRepo::new(self.db.clone())
    }

    fn refresh_token_repo(&self) -> SeaOrmRefreshTokenRepo {
        SeaOrmRefreshTokenRepo::new(self.db.clone())
    }

    fn auth_code_repo(&self) -> SeaOrmAuthCodeRepo {
        SeaOrmAuthCodeRepo::new(self.db.clone())
    }

    fn find_oauth_client(&self, client_id: &str) -> Option<&OAuthClientConfig> {
        self.oauth_clients.iter().find(|c| c.client_id == client_id)
    }
}

pub struct ApiError(pub shared_common::AppError);

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        ApiError(match e {
            DomainError::InvalidEmail => shared_common::AppError::BadRequest("invalid email".into()),
            DomainError::UserNotFound => shared_common::AppError::NotFound("user not found".into()),
            DomainError::EmailExists => shared_common::AppError::Conflict("email already exists".into()),
            DomainError::InvalidCredentials => shared_common::AppError::Unauthorized("invalid credentials".into()),
            DomainError::UserInactive => shared_common::AppError::Forbidden("user is not active".into()),
            DomainError::TokenExpired => shared_common::AppError::Unauthorized("token expired".into()),
            DomainError::TokenRevoked => shared_common::AppError::Unauthorized("token revoked".into()),
            DomainError::InvalidAuthCode => shared_common::AppError::BadRequest("invalid authorization code".into()),
            DomainError::PkceFailed => shared_common::AppError::BadRequest("pkce verification failed".into()),
            DomainError::Database(m) => shared_common::AppError::Database(m),
        })
    }
}

impl From<sea_orm::DbErr> for ApiError {
    fn from(e: sea_orm::DbErr) -> Self {
        ApiError(shared_common::AppError::from(e))
    }
}

impl From<argon2::password_hash::Error> for ApiError {
    fn from(_: argon2::password_hash::Error) -> Self {
        ApiError(shared_common::AppError::Internal("password hash error".into()))
    }
}

impl From<jsonwebtoken::errors::Error> for ApiError {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        ApiError(shared_common::AppError::Unauthorized(e.to_string()))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_kind) = match self.0 {
            shared_common::AppError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            shared_common::AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            shared_common::AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "unauthorized"),
            shared_common::AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "forbidden"),
            shared_common::AppError::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            shared_common::AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "database_error"),
            shared_common::AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };
        let msg = self.0.to_string();
        (
            status,
            Json(ErrorResponse {
                error: error_kind.into(),
                message: msg,
            }),
        )
            .into_response()
    }
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();
    Ok(hash)
}

fn verify_password(password: &str, hash: &str) -> Result<bool, ApiError> {
    let parsed = PasswordHash::new(hash)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

fn generate_token() -> String {
    Uuid::new_v4().to_string()
}

fn sign_jwt(secret: &str, user: &User, expires_in: u64) -> Result<String, ApiError> {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user.id.to_string(),
        exp: now + (expires_in as usize),
        iat: now,
        user_id: user.id,
        role: format!("{:?}", user.role).to_lowercase(),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok(token)
}

fn verify_jwt(secret: &str, token: &str) -> Result<Claims, ApiError> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}

fn extract_bearer_token(headers: &HeaderMap) -> Result<String, ApiError> {
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError(shared_common::AppError::Unauthorized("missing authorization header".into())))?;
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError(shared_common::AppError::Unauthorized("invalid authorization header".into())))?;
    Ok(token.to_string())
}

fn create_refresh_token_entry(user_id: Uuid, expires_in: u64) -> (String, RefreshToken) {
    let token = generate_token();
    let token_hash = hash_token(&token);
    let now = Utc::now();
    let refresh = RefreshToken {
        id: Uuid::new_v4(),
        user_id,
        token_hash,
        expires_at: now + chrono::Duration::seconds(expires_in as i64),
        revoked_at: None,
        created_at: now,
    };
    (token, refresh)
}

fn validate_input<T: Validate>(input: &T) -> Result<(), ApiError> {
    input
        .validate()
        .map_err(|e| ApiError(shared_common::AppError::BadRequest(e.to_string())))
}

pub async fn register(
    State(service): State<Arc<AuthService>>,
    Json(input): Json<RegisterInput>,
) -> Result<Json<AuthOutput>, ApiError> {
    validate_input(&input)?;

    let repo = service.user_repo();
    if repo.find_by_email(&input.email).await?.is_some() {
        return Err(ApiError(shared_common::AppError::Conflict("email already exists".into())));
    }

    let password_hash = hash_password(&input.password)?;

    let mut user = User::new(input.email, input.name, password_hash, UserRole::Viewer);
    let saved = repo.save(&user).await?;
    user = saved;

    let access_token = sign_jwt(&service.jwt_secret, &user, service.jwt_expires_in)?;
    let (refresh_token_str, refresh_entity) =
        create_refresh_token_entry(user.id, service.refresh_expires_in);
    service.refresh_token_repo().save(&refresh_entity).await?;

    Ok(Json(AuthOutput {
        access_token,
        refresh_token: refresh_token_str,
        expires_in: service.jwt_expires_in,
        user: user_to_dto(&user),
    }))
}

pub async fn login(
    State(service): State<Arc<AuthService>>,
    Json(input): Json<LoginInput>,
) -> Result<Json<AuthOutput>, ApiError> {
    validate_input(&input)?;

    let repo = service.user_repo();
    let user = repo
        .find_by_email(&input.email)
        .await?
        .ok_or_else(|| ApiError(shared_common::AppError::Unauthorized("invalid credentials".into())))?;

    if !verify_password(&input.password, &user.password_hash)? {
        return Err(ApiError(shared_common::AppError::Unauthorized(
            "invalid credentials".into(),
        )));
    }

    if user.status != UserStatus::Active {
        return Err(ApiError(shared_common::AppError::Forbidden(
            "user is not active".into(),
        )));
    }

    let access_token = sign_jwt(&service.jwt_secret, &user, service.jwt_expires_in)?;
    let (refresh_token_str, refresh_entity) =
        create_refresh_token_entry(user.id, service.refresh_expires_in);
    service.refresh_token_repo().save(&refresh_entity).await?;

    Ok(Json(AuthOutput {
        access_token,
        refresh_token: refresh_token_str,
        expires_in: service.jwt_expires_in,
        user: user_to_dto(&user),
    }))
}

pub async fn refresh(
    State(service): State<Arc<AuthService>>,
    Json(input): Json<RefreshInput>,
) -> Result<Json<RefreshOutput>, ApiError> {
    let token_hash = hash_token(&input.refresh_token);
    let repo = service.refresh_token_repo();
    let stored = repo
        .find_by_hash(&token_hash)
        .await?
        .ok_or_else(|| ApiError(shared_common::AppError::Unauthorized("invalid refresh token".into())))?;

    if stored.expires_at < Utc::now() {
        return Err(ApiError(shared_common::AppError::Unauthorized(
            "token expired".into(),
        )));
    }

    if stored.revoked_at.is_some() {
        return Err(ApiError(shared_common::AppError::Unauthorized(
            "token revoked".into(),
        )));
    }

    let user_repo = service.user_repo();
    let user = user_repo
        .find_by_id(stored.user_id)
        .await?
        .ok_or_else(|| ApiError(shared_common::AppError::NotFound("user not found".into())))?;

    let access_token = sign_jwt(&service.jwt_secret, &user, service.jwt_expires_in)?;

    Ok(Json(RefreshOutput {
        access_token,
        expires_in: service.jwt_expires_in,
    }))
}

pub async fn logout(
    State(service): State<Arc<AuthService>>,
    Json(input): Json<LogoutInput>,
) -> Result<StatusCode, ApiError> {
    let token_hash = hash_token(&input.refresh_token);
    let repo = service.refresh_token_repo();
    let stored = repo
        .find_by_hash(&token_hash)
        .await?
        .ok_or_else(|| ApiError(shared_common::AppError::Unauthorized("invalid refresh token".into())))?;

    repo.revoke(stored.id).await?;

    Ok(StatusCode::OK)
}

pub async fn me(
    State(service): State<Arc<AuthService>>,
    headers: HeaderMap,
) -> Result<Json<UserDto>, ApiError> {
    let token = extract_bearer_token(&headers)?;
    let claims = verify_jwt(&service.jwt_secret, &token)?;

    let repo = service.user_repo();
    let user = repo
        .find_by_id(claims.user_id)
        .await?
        .ok_or_else(|| ApiError(shared_common::AppError::NotFound("user not found".into())))?;

    Ok(Json(user_to_dto(&user)))
}

pub async fn oauth_authorize(
    State(service): State<Arc<AuthService>>,
    headers: HeaderMap,
    Query(input): Query<AuthorizeInput>,
) -> Result<Response, ApiError> {
    validate_input(&input)?;

    let client = service
        .find_oauth_client(&input.client_id)
        .ok_or_else(|| ApiError(shared_common::AppError::BadRequest("unknown client_id".into())))?;

    if !client.redirect_uris.contains(&input.redirect_uri) {
        return Err(ApiError(shared_common::AppError::BadRequest(
            "invalid redirect_uri".into(),
        )));
    }

    let token = extract_bearer_token(&headers)?;
    let claims = verify_jwt(&service.jwt_secret, &token)?;

    let code = generate_token();
    let code_hash = hash_token(&code);
    let now = Utc::now();
    let auth_code = OAuthAuthorizationCode {
        id: Uuid::new_v4(),
        client_id: input.client_id.clone(),
        user_id: claims.user_id,
        code_hash,
        redirect_uri: input.redirect_uri.clone(),
        code_challenge: input.code_challenge.clone(),
        code_challenge_method: input.code_challenge_method.clone(),
        expires_at: now + chrono::Duration::minutes(10),
        used: false,
        created_at: now,
    };
    service.auth_code_repo().save(&auth_code).await?;

    let mut redirect_url = format!("{}?code={}", input.redirect_uri, code);
    if let Some(state) = input.state {
        redirect_url.push_str(&format!("&state={}", state));
    }

    Ok(Redirect::to(&redirect_url).into_response())
}

pub async fn oauth_token(
    State(service): State<Arc<AuthService>>,
    Json(input): Json<TokenInput>,
) -> Result<Json<TokenOutput>, ApiError> {
    validate_input(&input)?;

    if input.grant_type != "authorization_code" {
        return Err(ApiError(shared_common::AppError::BadRequest(
            "unsupported grant_type".into(),
        )));
    }

    let code_hash = hash_token(&input.code);
    let code_repo = service.auth_code_repo();
    let stored = code_repo
        .find_by_hash(&code_hash)
        .await?
        .ok_or_else(|| ApiError(shared_common::AppError::BadRequest("invalid authorization code".into())))?;

    if stored.client_id != input.client_id {
        return Err(ApiError(shared_common::AppError::BadRequest(
            "client_id mismatch".into(),
        )));
    }

    if stored.redirect_uri != input.redirect_uri {
        return Err(ApiError(shared_common::AppError::BadRequest(
            "redirect_uri mismatch".into(),
        )));
    }

    if stored.used {
        return Err(ApiError(shared_common::AppError::BadRequest(
            "authorization code already used".into(),
        )));
    }

    if stored.expires_at < Utc::now() {
        return Err(ApiError(shared_common::AppError::BadRequest(
            "authorization code expired".into(),
        )));
    }

    if stored.code_challenge_method != "S256" {
        return Err(ApiError(shared_common::AppError::BadRequest(
            "only S256 code_challenge_method is supported".into(),
        )));
    }

    let mut hasher = Sha256::new();
    hasher.update(input.code_verifier.as_bytes());
    let hash = hasher.finalize();
    let computed_challenge = URL_SAFE_NO_PAD.encode(hash);

    if computed_challenge != stored.code_challenge {
        return Err(ApiError(shared_common::AppError::BadRequest(
            "pkce verification failed".into(),
        )));
    }

    let user_repo = service.user_repo();
    let user = user_repo
        .find_by_id(stored.user_id)
        .await?
        .ok_or_else(|| ApiError(shared_common::AppError::NotFound("user not found".into())))?;

    let access_token = sign_jwt(&service.jwt_secret, &user, service.jwt_expires_in)?;
    let (refresh_token_str, refresh_entity) =
        create_refresh_token_entry(user.id, service.refresh_expires_in);
    service.refresh_token_repo().save(&refresh_entity).await?;

    code_repo.mark_used(stored.id).await?;

    Ok(Json(TokenOutput {
        access_token,
        refresh_token: refresh_token_str,
        token_type: "Bearer".into(),
        expires_in: service.jwt_expires_in,
    }))
}
