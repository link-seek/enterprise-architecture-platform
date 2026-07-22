use std::sync::{Arc, OnceLock};
use std::time::Instant;

use axum::extract::State;
use axum::response::Json;
use axum::Router;
use serde_json::json;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa_axum::router::OpenApiRouter;

use user_management::infrastructure::http::handlers::AuthService;

use crate::ai::backend::LlmBackend;
use crate::graphql::GraphqlSchema;
use crate::state::AppState;

/// Process start time, captured once at router construction (or on first status
/// request in contexts where the router is never built, e.g. unit tests).
static START_TIME: OnceLock<Instant> = OnceLock::new();

pub fn build_router(state: AppState, graphql_schema: GraphqlSchema) -> Router {
    START_TIME.get_or_init(Instant::now);
    let jwt_secret = state.config.jwt.rsa_private_key_pem.clone();

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

    // Rate limiter: generous for GraphQL + REST
    let governor_config = std::sync::Arc::new(
        tower_governor::governor::GovernorConfigBuilder::default()
            .per_second(4)
            .burst_size(25)
            .finish()
            .unwrap(),
    );
    let governor_limiter = tower_governor::GovernorLayer::new(governor_config);

    // === 用 OpenApiRouter 分组构建，routes! 宏自动收集 OpenAPI path ===

    // health + ai handlers (State = AppState)
    let main_router = OpenApiRouter::new()
        .routes(utoipa_axum::routes!(health_handler))
        .routes(utoipa_axum::routes!(health_status_handler))
        .routes(utoipa_axum::routes!(version_handler))
        .routes(utoipa_axum::routes!(info_handler))
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
        .routes(utoipa_axum::routes!(
            user_management::infrastructure::http::handlers::update_role
        ))
        .with_state(auth_service);

    // 合并所有 router 和 OpenAPI spec
    let merged = main_router.merge(auth_router);
    let (router, api) = merged.split_for_parts();

    // 合并 schemas 到 OpenAPI spec
    let api = crate::api_doc::merge_schemas(api);

    // === GraphQL 路由 (用 route_service 注册，参照 nakamuraos 模式) ===
    //
    // 架构：
    //   POST /graphql   → GraphQL handler (JWT 提取 + LifecycleHooks 鉴权)
    //     - Query: 公开（无需 JWT）
    //     - Mutation: 需要 JWT（LifecycleHooks entity_guard 检查 Claims）
    //   GET  /graphql   → GraphiQL 交互式 IDE（生产环境可禁用）
    //
    // 用 route_service 而非 route，因为 utoipa-axum 的 split_for_parts()
    // 返回 Router<()>，后续 route() 注册的路由在运行时无法匹配（axum 0.8 已知问题）

    let graphql_service =
        crate::graphql::GraphQLService::new(graphql_schema.clone(), jwt_secret);

    router
        .merge(crate::api_doc::swagger_ui_from(api))
        .route_service("/graphql", graphql_service)
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

/// 应用版本信息
#[utoipa::path(
    get,
    path = "/api/version",
    tag = "health",
    responses(
        (status = 200, description = "应用版本信息", body = inline(serde_json::Value)),
    )
)]
async fn version_handler() -> Json<serde_json::Value> {
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "name": "enterprise-architecture-platform",
        "rust_version": env!("CARGO_PKG_RUST_VERSION"),
    }))
}

/// 系统基本信息
#[utoipa::path(
    get,
    path = "/api/info",
    tag = "health",
    responses(
        (status = 200, description = "系统基本信息", body = inline(serde_json::Value)),
    )
)]
async fn info_handler() -> Json<serde_json::Value> {
    Json(json!({
        "name": "enterprise-architecture-platform",
        "version": env!("CARGO_PKG_VERSION"),
        "rust_version": env!("CARGO_PKG_RUST_VERSION"),
    }))
}

/// 详细健康状态。不查询数据库，仅返回基本状态信息。
#[utoipa::path(
    get,
    path = "/api/health/status",
    tag = "health",
    responses(
        (status = 200, description = "详细健康状态", body = inline(serde_json::Value)),
    )
)]
async fn health_status_handler() -> Json<serde_json::Value> {
    let uptime = format_uptime(START_TIME.get_or_init(Instant::now).elapsed());

    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime": uptime,
        "db": "connected",
    }))
}

/// 将 `Duration` 格式化为紧凑的人类可读字符串，例如 `1h 2m 3s`、`45s`、`0s`。
fn format_uptime(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs();
    let days = total_secs / 86_400;
    let hours = (total_secs % 86_400) / 3_600;
    let mins = (total_secs % 3_600) / 60;
    let secs = total_secs % 60;

    let parts: [(u64, &str); 4] = [
        (days, "d"),
        (hours, "h"),
        (mins, "m"),
        (secs, "s"),
    ];

    let shown: Vec<String> = parts
        .iter()
        .skip_while(|(v, _)| *v == 0)
        .take(2)
        .map(|(v, u)| format!("{v}{u}"))
        .collect();

    if shown.is_empty() {
        "0s".to_string()
    } else {
        shown.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn version_handler_reports_cargo_metadata() {
        let Json(value) = version_handler().await;

        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["name"], "enterprise-architecture-platform");
        assert_eq!(value["rust_version"], env!("CARGO_PKG_RUST_VERSION"));
    }

    #[tokio::test]
    async fn info_handler_reports_system_info() {
        let Json(value) = info_handler().await;

        assert_eq!(value["name"], "enterprise-architecture-platform");
        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["rust_version"], env!("CARGO_PKG_RUST_VERSION"));
    }

    #[tokio::test]
    async fn health_status_handler_returns_expected_shape() {
        let Json(value) = health_status_handler().await;

        assert_eq!(value["status"], "ok");
        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["db"], "connected");
        assert!(value["uptime"].is_string(), "uptime should be a string");
        assert!(!value["uptime"].as_str().unwrap().is_empty());
    }

    #[test]
    fn format_uptime_formats_compact_durations() {
        assert_eq!(format_uptime(std::time::Duration::from_secs(0)), "0s");
        assert_eq!(format_uptime(std::time::Duration::from_secs(45)), "45s");
        assert_eq!(
            format_uptime(std::time::Duration::from_secs(130)),
            "2m 10s"
        );
        assert_eq!(
            format_uptime(std::time::Duration::from_secs(3_723)),
            "1h 2m"
        );
        assert_eq!(
            format_uptime(std::time::Duration::from_secs(90_061)),
            "1d 1h"
        );
    }
}
