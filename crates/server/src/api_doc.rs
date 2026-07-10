use utoipa::OpenApi as OpenApiTrait;
use utoipa::openapi::OpenApi;

use user_management::infrastructure::http::dto::{ErrorResponse as AuthErrorResponse, LogoutInput};
use user_management::application::register::{AuthOutput, RegisterInput, UserDto};
use user_management::application::login::LoginInput;
use user_management::application::token::{Claims, RefreshInput, RefreshOutput};
use user_management::application::oauth::{TokenInput, TokenOutput};

use business_architecture::infrastructure::http::dto::{
    ErrorResponse as BaErrorResponse,
    CreateCapabilityInput, UpdateCapabilityInput, CapabilityDto,
    CreateProcessInput, UpdateProcessInput, ProcessDto,
    CreateProcessStepInput, ProcessStepDto,
    CreateValueStreamInput, UpdateValueStreamInput, ValueStreamDto,
    CreateValueStreamStageInput, ValueStreamStageDto,
    LinkProcessInput, LinkStageCapabilityInput,
    GapAnalysisInput, RedundancyInput,
};
use business_architecture::application::analysis::{GapAnalysisResult, RedundancyResult, Gap, Duplicate, Mergeable};

use crate::ai::dto::{AiScenario, AiSuggestion, AiResponse, AiRequest};

use shared_common::enums::{
    CapabilityLevel, MaturityLevel, BusinessValueRating, CostRating,
    LifecycleStatus, ValueStreamImportance, UserRole, UserStatus,
};
use shared_common::value_objects::{NaturalId, StringVec, NaturalIdVec, StringStringMap};
use shared_common::PageInfo;

/// 所有 DTO schemas 定义，供 OpenApiRouter 生成的 spec 合并
#[derive(utoipa::OpenApi)]
#[openapi(
    components(
        schemas(
            // shared-common enums
            CapabilityLevel, MaturityLevel, BusinessValueRating, CostRating,
            LifecycleStatus, ValueStreamImportance, UserRole, UserStatus,
            // shared-common value objects
            NaturalId, StringVec, NaturalIdVec, StringStringMap,
            // shared-common pagination
            PageInfo,
            // auth DTOs
            RegisterInput, LoginInput, AuthOutput, UserDto,
            RefreshInput, RefreshOutput, Claims,
            TokenInput, TokenOutput,
            LogoutInput, AuthErrorResponse,
            // business-architecture DTOs
            CreateCapabilityInput, UpdateCapabilityInput, CapabilityDto,
            CreateProcessInput, UpdateProcessInput, ProcessDto,
            CreateProcessStepInput, ProcessStepDto,
            CreateValueStreamInput, UpdateValueStreamInput, ValueStreamDto,
            CreateValueStreamStageInput, ValueStreamStageDto,
            LinkProcessInput, LinkStageCapabilityInput,
            GapAnalysisInput, RedundancyInput,
            BaErrorResponse,
            // analysis results
            GapAnalysisResult, Gap, RedundancyResult, Duplicate, Mergeable,
            // AI DTOs
            AiScenario, AiSuggestion, AiResponse, AiRequest,
        )
    ),
    tags(
        (name = "health", description = "健康检查"),
        (name = "auth", description = "认证授权 API"),
        (name = "business-architecture", description = "业务架构 API"),
        (name = "ai", description = "AI 辅助 API"),
        (name = "graphql", description = "GraphQL 端点"),
    )
)]
struct SchemasDoc;

/// 将 schemas 合并到 OpenApiRouter 生成的 OpenApi spec 中
pub fn merge_schemas(mut api: OpenApi) -> OpenApi {
    api.merge(SchemasDoc::openapi());
    // 补充 info
    api.info.title = "企业架构平台 API".to_string();
    api.info.version = "0.1.0".to_string();
    api.info.description = Some("Enterprise Architecture Platform - Rust + axum + GraphQL".to_string());
    api
}

/// 从合并后的 OpenApi spec 创建 SwaggerUi
pub fn swagger_ui_from(api: OpenApi) -> utoipa_swagger_ui::SwaggerUi {
    utoipa_swagger_ui::SwaggerUi::new("/swagger-ui")
        .url("/api-docs/openapi.json", api)
}
