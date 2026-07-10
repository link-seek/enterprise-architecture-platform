use std::sync::Arc;

use axum::extract::State;
use axum::response::Json;
use axum::routing::get;
use axum::Router;
use serde_json::json;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use user_management::infrastructure::http::handlers::AuthService;
use user_management::infrastructure::http::routes::auth_routes;
use business_architecture::infrastructure::http::handlers::BusinessArchitectureService;
use business_architecture::infrastructure::http::routes::business_architecture_routes;

use crate::ai::backend::LlmBackend;
use crate::ai::routes::ai_routes;
use crate::graphql::GraphqlSchema;
use crate::state::AppState;

pub fn build_router(state: AppState, graphql_schema: GraphqlSchema) -> Router {
    let auth_service = Arc::new(AuthService::new(
        state.db.clone(),
        state.config.jwt.rsa_private_key_pem.clone(),
        state.config.jwt.access_token_ttl_minutes * 60,
        state.config.jwt.refresh_token_ttl_days * 24 * 60 * 60,
        state
            .config
            .oauth
            .clients
            .iter()
            .map(|c| user_management::infrastructure::http::handlers::OAuthClientConfig {
                client_id: c.client_id.clone(),
                redirect_uris: c.redirect_uris.clone(),
            })
            .collect(),
    ));

    let ba_service = Arc::new(BusinessArchitectureService::new(state.db.clone()));

    let governor_config = tower_governor::governor::GovernorConfig::default();
    let governor_limiter = tower_governor::GovernorLayer::new(governor_config);

    Router::new()
        .route("/health", get(health_handler))
        .merge(crate::api_doc::swagger_ui())
        .merge(auth_routes().with_state(auth_service))
        .merge(business_architecture_routes().with_state(ba_service))
        .nest("/api/ai", ai_routes())
        .route_service("/graphql", async_graphql_axum::GraphQL::new(graphql_schema))
        .layer(governor_limiter)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// 健康检查
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "服务状态", body = inline(serde_json::Value)),
    )
)]
async fn health_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let db_status = if state.db.ping().await.is_ok() {
        "up"
    } else {
        "down"
    };

    let llm_backend = LlmBackend::from_config(&state.config.llm);
    let llm_status = if llm_backend.is_available() { "up" } else { "down" };

    let overall = if db_status == "up" && llm_status == "up" {
        "ok"
    } else {
        "degraded"
    };

    Json(json!({
        "status": overall,
        "db": db_status,
        "llm": llm_status,
    }))
}
