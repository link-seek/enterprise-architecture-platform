use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;

use super::handlers::{
    login, logout, me, oauth_authorize, oauth_token, refresh, register, AuthService,
};

pub fn auth_routes() -> Router<Arc<AuthService>> {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/me", get(me))
        .route("/api/oauth/authorize", get(oauth_authorize))
        .route("/api/oauth/token", post(oauth_token))
}
