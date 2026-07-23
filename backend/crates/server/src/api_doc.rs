use utoipa::OpenApi as OpenApiTrait;
use utoipa::openapi::OpenApi;

use user_management::infrastructure::http::dto::{ErrorResponse as AuthErrorResponse, LogoutInput};
use user_management::application::register::{AuthOutput, CreateUserInput, RegisterInput, UserDto};
use user_management::application::login::LoginInput;
use user_management::application::token::{Claims, RefreshInput, RefreshOutput};
use user_management::application::oauth::{TokenInput, TokenOutput};

use crate::ai::dto::{AiScenario, AiSuggestion, AiResponse, AiRequest};

use shared_common::enums::{
    UserRole, UserStatus,
};
use shared_common::PageInfo;

/// 所有 DTO schemas 定义，供 OpenApiRouter 生成的 spec 合并
/// 注意：业务数据 (business-architecture) 的 CRUD 走 GraphQL，不在此 OpenAPI spec 中
#[derive(utoipa::OpenApi)]
#[openapi(
    components(
        schemas(
            // shared-common enums (auth only)
            UserRole, UserStatus,
            // shared-common pagination
            PageInfo,
            // auth DTOs
            RegisterInput, CreateUserInput, LoginInput, AuthOutput, UserDto,
            RefreshInput, RefreshOutput, Claims,
            TokenInput, TokenOutput,
            LogoutInput, AuthErrorResponse,
            // AI DTOs
            AiScenario, AiSuggestion, AiResponse, AiRequest,
        )
    ),
    tags(
        (name = "health", description = "健康检查"),
        (name = "auth", description = "认证授权 API"),
        (name = "ai", description = "AI 辅助 API"),
        (name = "graphql", description = "GraphQL 端点 (业务数据 CRUD)"),
    )
)]
struct SchemasDoc;

/// 将 schemas 合并到 OpenApiRouter 生成的 OpenApi spec 中
pub fn merge_schemas(mut api: OpenApi) -> OpenApi {
    api.merge(SchemasDoc::openapi());
    // 补充 info
    api.info.title = "企业架构平台 API".to_string();
    api.info.version = "0.1.0".to_string();
    api.info.description = Some("Enterprise Architecture Platform - Rust + axum + GraphQL (业务数据走 GraphQL，认证/AI 走 REST)".to_string());
    api
}

/// 从合并后的 OpenApi spec 创建 SwaggerUi
pub fn swagger_ui_from(api: OpenApi) -> utoipa_swagger_ui::SwaggerUi {
    utoipa_swagger_ui::SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", api)
}
