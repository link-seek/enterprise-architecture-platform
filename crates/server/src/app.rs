use std::sync::Arc;

use axum::extract::State;
use axum::response::Json;
use axum::Router;
use serde_json::json;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa_axum::router::OpenApiRouter;

use user_management::infrastructure::http::handlers::AuthService;
use business_architecture::infrastructure::http::handlers::BusinessArchitectureService;

use crate::ai::backend::LlmBackend;
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

    let governor_config = std::sync::Arc::new(
        tower_governor::governor::GovernorConfigBuilder::default()
            .per_second(2)
            .burst_size(10)
            .finish()
            .unwrap(),
    );
    let governor_limiter = tower_governor::GovernorLayer::new(governor_config);

    // === 用 OpenApiRouter 分组构建，routes! 宏自动收集 OpenAPI path ===

    // health + ai handlers (State = AppState)
    let main_router = OpenApiRouter::new()
        .routes(utoipa_axum::routes!(health_handler))
        .routes(utoipa_axum::routes!(crate::ai::handlers::suggest_handler))
        .routes(utoipa_axum::routes!(crate::ai::handlers::stream_handler))
        .with_state(state.clone());

    // auth handlers (State = Arc<AuthService>)
    let auth_router = OpenApiRouter::new()
        .routes(utoipa_axum::routes!(
            user_management::infrastructure::http::handlers::register
        ))
        .routes(utoipa_axum::routes!(
            user_management::infrastructure::http::handlers::login
        ))
        .routes(utoipa_axum::routes!(
            user_management::infrastructure::http::handlers::refresh
        ))
        .routes(utoipa_axum::routes!(
            user_management::infrastructure::http::handlers::logout
        ))
        .routes(utoipa_axum::routes!(
            user_management::infrastructure::http::handlers::me
        ))
        .routes(utoipa_axum::routes!(
            user_management::infrastructure::http::handlers::oauth_authorize
        ))
        .routes(utoipa_axum::routes!(
            user_management::infrastructure::http::handlers::oauth_token
        ))
        .with_state(auth_service);

    // business-architecture handlers (State = Arc<BusinessArchitectureService>)
    let ba_router = OpenApiRouter::new()
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::create_capability
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::list_capabilities
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::get_capability
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::update_capability
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::delete_capability
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::get_capability_processes
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::link_capability_process
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::create_process
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::list_processes
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::get_process
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::update_process
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::delete_process
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::publish_process_version
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::get_process_versions
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::get_process_steps
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::create_process_step
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::create_value_stream
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::list_value_streams
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::get_value_stream
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::update_value_stream
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::delete_value_stream
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::get_value_stream_stages
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::create_value_stream_stage
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::link_stage_capability
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::gap_analysis
        ))
        .routes(utoipa_axum::routes!(
            business_architecture::infrastructure::http::handlers::redundancy_analysis
        ))
        .with_state(ba_service);

    // 合并所有 router 和 OpenAPI spec
    let merged = main_router.merge(auth_router).merge(ba_router);
    let (router, api) = merged.split_for_parts();

    // 合并 schemas 到 OpenAPI spec
    let api = crate::api_doc::merge_schemas(api);

    router
        .merge(crate::api_doc::swagger_ui_from(api))
        .route_service("/graphql", async_graphql_axum::GraphQL::new(graphql_schema))
        .layer(governor_limiter)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
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
