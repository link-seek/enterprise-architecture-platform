use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "企业架构平台 API",
        version = "0.1.0",
        description = "Enterprise Architecture Platform - Rust + axum + GraphQL",
    ),
    paths(
        crate::app::health_handler,
    ),
    tags(
        (name = "health", description = "健康检查"),
        (name = "auth", description = "认证授权 API"),
        (name = "business-architecture", description = "业务架构 API"),
        (name = "ai", description = "AI 辅助 API"),
        (name = "graphql", description = "GraphQL 端点"),
    )
)]
pub struct ApiDoc;

pub fn swagger_ui() -> utoipa_swagger_ui::SwaggerUi {
    utoipa_swagger_ui::SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", ApiDoc::openapi())
}
