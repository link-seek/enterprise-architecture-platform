use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use sea_orm::DatabaseConnection;
use shared_common::enums::{LifecycleStatus, MaturityLevel};
use shared_common::{Page, PageInput};
use uuid::Uuid;

use crate::application::analysis::{Duplicate, Gap, GapAnalysisResult, Mergeable, RedundancyResult};
use crate::domain::capability::entity::BusinessCapability;
use crate::domain::capability::repository::CapabilityRepository;
use crate::domain::error::DomainError;
use crate::domain::process::entity::{BusinessProcess, ProcessStep};
use crate::domain::process::repository::{ProcessRepository, ProcessStepRepository};
use crate::domain::value_stream::entity::{ValueStream, ValueStreamStage};
use crate::domain::value_stream::repository::{
    ValueStreamRepository, ValueStreamStageRepository,
};
use crate::infrastructure::http::dto::*;
use crate::infrastructure::persistence::capability_repo::SeaOrmCapabilityRepo;
use crate::infrastructure::persistence::process_repo::SeaOrmProcessRepo;
use crate::infrastructure::persistence::value_stream_repo::SeaOrmValueStreamRepo;

pub struct BusinessArchitectureService {
    db: DatabaseConnection,
}

impl BusinessArchitectureService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub fn capability_repo(&self) -> SeaOrmCapabilityRepo {
        SeaOrmCapabilityRepo::new(self.db.clone())
    }

    pub fn process_repo(&self) -> SeaOrmProcessRepo {
        SeaOrmProcessRepo::new(self.db.clone())
    }

    pub fn value_stream_repo(&self) -> SeaOrmValueStreamRepo {
        SeaOrmValueStreamRepo::new(self.db.clone())
    }
}

pub struct ApiError(pub shared_common::AppError);

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        ApiError(match e {
            DomainError::CapabilityNotFound => {
                shared_common::AppError::NotFound("capability not found".into())
            }
            DomainError::ProcessNotFound => {
                shared_common::AppError::NotFound("process not found".into())
            }
            DomainError::ValueStreamNotFound => {
                shared_common::AppError::NotFound("value stream not found".into())
            }
            DomainError::ProcessVersionNotFound => {
                shared_common::AppError::NotFound("process version not found".into())
            }
            DomainError::CannotReferenceArchived => shared_common::AppError::BadRequest(
                "cannot reference archived process".into(),
            ),
            DomainError::NotOwner => {
                shared_common::AppError::Forbidden("only owner or admin can modify".into())
            }
            DomainError::Semver(m) => shared_common::AppError::BadRequest(m),
            DomainError::Database(m) => shared_common::AppError::Database(m),
        })
    }
}

impl From<sea_orm::DbErr> for ApiError {
    fn from(e: sea_orm::DbErr) -> Self {
        ApiError(shared_common::AppError::from(e))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_kind) = match self.0 {
            shared_common::AppError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            shared_common::AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            shared_common::AppError::Unauthorized(_) => {
                (StatusCode::UNAUTHORIZED, "unauthorized")
            }
            shared_common::AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "forbidden"),
            shared_common::AppError::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            shared_common::AppError::Database(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "database_error")
            }
            shared_common::AppError::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
            }
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

fn maturity_rank(m: MaturityLevel) -> u8 {
    match m {
        MaturityLevel::Level1 => 1,
        MaturityLevel::Level2 => 2,
        MaturityLevel::Level3 => 3,
        MaturityLevel::Level4 => 4,
        MaturityLevel::Level5 => 5,
    }
}

fn name_similarity(a: &str, b: &str) -> f64 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    if a_lower == b_lower {
        return 1.0;
    }
    let a_chars: std::collections::HashSet<char> = a_lower.chars().collect();
    let b_chars: std::collections::HashSet<char> = b_lower.chars().collect();
    let intersection = a_chars.intersection(&b_chars).count();
    let union = a_chars.union(&b_chars).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

#[utoipa::path(
    post,
    path = "/api/business-architecture/capabilities",
    tag = "business-architecture",
    request_body = CreateCapabilityInput,
    responses(
        (status = 201, description = "创建能力", body = CapabilityDto),
        (status = 400, description = "参数错误"),
    )
)]
pub async fn create_capability(
    State(service): State<Arc<BusinessArchitectureService>>,
    Json(input): Json<CreateCapabilityInput>,
) -> Result<Json<CapabilityDto>, ApiError> {
    let now = Utc::now();
    let id = Uuid::new_v4();
    let cap = BusinessCapability {
        id,
        business_version: "0.1.0".to_string(),
        status: LifecycleStatus::Active,
        name: input.name,
        description: input.description,
        level: input.level,
        maturity: input.maturity,
        business_value: input.business_value,
        cost: input.cost,
        owner_id: input.owner_id,
        created_by: input.created_by,
        updated_by: input.created_by,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    let saved = service.capability_repo().save(&cap).await?;
    Ok(Json(saved.into()))
}

#[utoipa::path(
    get,
    path = "/api/business-architecture/capabilities",
    tag = "business-architecture",
    params(PageInput),
    responses(
        (status = 200, description = "能力列表", body = Page<CapabilityDto>),
    )
)]
pub async fn list_capabilities(
    State(service): State<Arc<BusinessArchitectureService>>,
    Query(params): Query<PageInput>,
) -> Result<Json<Page<CapabilityDto>>, ApiError> {
    let (caps, total) = service
        .capability_repo()
        .list_active(params.page, params.per_page)
        .await?;
    let items: Vec<CapabilityDto> = caps.into_iter().map(Into::into).collect();
    Ok(Json(Page::new(items, params.page, params.per_page, total)))
}

#[utoipa::path(
    get,
    path = "/api/business-architecture/capabilities/{id}",
    tag = "business-architecture",
    params(("id" = Uuid, Path, description = "能力 ID")),
    responses(
        (status = 200, description = "能力详情", body = CapabilityDto),
        (status = 404, description = "未找到"),
    )
)]
pub async fn get_capability(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<CapabilityDto>, ApiError> {
    let cap = service
        .capability_repo()
        .find_by_id(id)
        .await?
        .ok_or_else(|| ApiError(shared_common::AppError::NotFound("capability not found".into())))?;
    Ok(Json(cap.into()))
}

#[utoipa::path(
    put,
    path = "/api/business-architecture/capabilities/{id}",
    tag = "business-architecture",
    request_body = UpdateCapabilityInput,
    responses(
        (status = 200, description = "更新成功", body = CapabilityDto),
        (status = 404, description = "未找到"),
    )
)]
pub async fn update_capability(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateCapabilityInput>,
) -> Result<Json<CapabilityDto>, ApiError> {
    let repo = service.capability_repo();
    let mut cap = repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| ApiError(shared_common::AppError::NotFound("capability not found".into())))?;

    if let Some(name) = input.name {
        cap.name = name;
    }
    if let Some(desc) = input.description {
        cap.description = desc;
    }
    if let Some(level) = input.level {
        cap.level = level;
    }
    if let Some(maturity) = input.maturity {
        cap.maturity = maturity;
    }
    if let Some(bv) = input.business_value {
        cap.business_value = bv;
    }
    if let Some(cost) = input.cost {
        cap.cost = cost;
    }
    if let Some(owner) = input.owner_id {
        cap.owner_id = owner;
    }
    if let Some(updated_by) = input.updated_by {
        cap.updated_by = Some(updated_by);
    }
    cap.updated_at = Utc::now();

    let saved = repo.save(&cap).await?;
    Ok(Json(saved.into()))
}

#[utoipa::path(
    delete,
    path = "/api/business-architecture/capabilities/{id}",
    tag = "business-architecture",
    responses(
        (status = 204, description = "删除成功"),
        (status = 404, description = "未找到"),
    )
)]
pub async fn delete_capability(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    service.capability_repo().soft_delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/business-architecture/capabilities/{id}/processes",
    tag = "business-architecture",
    responses(
        (status = 200, description = "关联流程列表", body = Vec<uuid::Uuid>),
    )
)]
pub async fn get_capability_processes(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Uuid>>, ApiError> {
    let process_ids = service.capability_repo().find_processes(id).await?;
    Ok(Json(process_ids))
}

#[utoipa::path(
    post,
    path = "/api/business-architecture/capabilities/{id}/processes",
    tag = "business-architecture",
    request_body = LinkProcessInput,
    responses(
        (status = 201, description = "关联成功"),
    )
)]
pub async fn link_capability_process(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
    Json(input): Json<LinkProcessInput>,
) -> Result<StatusCode, ApiError> {
    service
        .capability_repo()
        .link_process(id, input.process_id)
        .await?;
    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    post,
    path = "/api/business-architecture/processes",
    tag = "business-architecture",
    request_body = CreateProcessInput,
    responses(
        (status = 201, description = "创建流程", body = ProcessDto),
    )
)]
pub async fn create_process(
    State(service): State<Arc<BusinessArchitectureService>>,
    Json(input): Json<CreateProcessInput>,
) -> Result<Json<ProcessDto>, ApiError> {
    let now = Utc::now();
    let id = Uuid::new_v4();
    let proc = BusinessProcess {
        id,
        logical_id: id,
        business_version: "0.1.0".to_string(),
        status: LifecycleStatus::Active,
        name: input.name,
        description: input.description,
        sla: input.sla,
        cost_per_transaction: input.cost_per_transaction,
        cycle_time: input.cycle_time,
        owner_id: input.owner_id,
        created_by: input.created_by,
        updated_by: input.created_by,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    let saved = ProcessRepository::save(&service.process_repo(), &proc).await?;
    Ok(Json(saved.into()))
}

#[utoipa::path(
    get,
    path = "/api/business-architecture/processes",
    tag = "business-architecture",
    params(PageInput),
    responses(
        (status = 200, description = "流程列表", body = Page<ProcessDto>),
    )
)]
pub async fn list_processes(
    State(service): State<Arc<BusinessArchitectureService>>,
    Query(params): Query<PageInput>,
) -> Result<Json<Page<ProcessDto>>, ApiError> {
    let (procs, total) = service
        .process_repo()
        .find_all_active(params.page, params.per_page)
        .await?;
    let items: Vec<ProcessDto> = procs.into_iter().map(Into::into).collect();
    Ok(Json(Page::new(items, params.page, params.per_page, total)))
}

#[utoipa::path(
    get,
    path = "/api/business-architecture/processes/{id}",
    tag = "business-architecture",
    responses(
        (status = 200, description = "流程详情", body = ProcessDto),
        (status = 404, description = "未找到"),
    )
)]
pub async fn get_process(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ProcessDto>, ApiError> {
    let proc = service
        .process_repo()
        .find_by_id(id)
        .await?
        .ok_or_else(|| ApiError(shared_common::AppError::NotFound("process not found".into())))?;
    Ok(Json(proc.into()))
}

#[utoipa::path(
    put,
    path = "/api/business-architecture/processes/{id}",
    tag = "business-architecture",
    request_body = UpdateProcessInput,
    responses(
        (status = 200, description = "更新成功", body = ProcessDto),
        (status = 404, description = "未找到"),
    )
)]
pub async fn update_process(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateProcessInput>,
) -> Result<Json<ProcessDto>, ApiError> {
    let repo = service.process_repo();
    let mut proc = repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| ApiError(shared_common::AppError::NotFound("process not found".into())))?;

    if let Some(name) = input.name {
        proc.name = name;
    }
    if let Some(desc) = input.description {
        proc.description = desc;
    }
    if let Some(sla) = input.sla {
        proc.sla = sla;
    }
    if let Some(cost) = input.cost_per_transaction {
        proc.cost_per_transaction = cost;
    }
    if let Some(ct) = input.cycle_time {
        proc.cycle_time = ct;
    }
    if let Some(owner) = input.owner_id {
        proc.owner_id = owner;
    }
    if let Some(updated_by) = input.updated_by {
        proc.updated_by = Some(updated_by);
    }
    proc.updated_at = Utc::now();

    let saved = ProcessRepository::save(&repo, &proc).await?;
    Ok(Json(saved.into()))
}

#[utoipa::path(
    delete,
    path = "/api/business-architecture/processes/{id}",
    tag = "business-architecture",
    responses(
        (status = 204, description = "删除成功"),
        (status = 404, description = "未找到"),
    )
)]
pub async fn delete_process(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    ProcessRepository::soft_delete(&service.process_repo(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/business-architecture/processes/{id}/publish",
    tag = "business-architecture",
    responses(
        (status = 200, description = "发布新版本", body = ProcessDto),
    )
)]
pub async fn publish_process_version(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ProcessDto>, ApiError> {
    let new_version = service.process_repo().publish_new_version(id).await?;
    Ok(Json(new_version.into()))
}

#[utoipa::path(
    get,
    path = "/api/business-architecture/processes/{id}/versions",
    tag = "business-architecture",
    responses(
        (status = 200, description = "版本列表", body = Vec<ProcessDto>),
    )
)]
pub async fn get_process_versions(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ProcessDto>>, ApiError> {
    let versions = service.process_repo().find_all_versions(id).await?;
    let items: Vec<ProcessDto> = versions.into_iter().map(Into::into).collect();
    Ok(Json(items))
}

#[utoipa::path(
    get,
    path = "/api/business-architecture/processes/{id}/steps",
    tag = "business-architecture",
    responses(
        (status = 200, description = "步骤列表", body = Vec<ProcessStepDto>),
    )
)]
pub async fn get_process_steps(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ProcessStepDto>>, ApiError> {
    let steps = service.process_repo().find_by_process(id).await?;
    let items: Vec<ProcessStepDto> = steps.into_iter().map(Into::into).collect();
    Ok(Json(items))
}

#[utoipa::path(
    post,
    path = "/api/business-architecture/processes/{id}/steps",
    tag = "business-architecture",
    request_body = CreateProcessStepInput,
    responses(
        (status = 201, description = "创建步骤", body = ProcessStepDto),
    )
)]
pub async fn create_process_step(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
    Json(input): Json<CreateProcessStepInput>,
) -> Result<Json<ProcessStepDto>, ApiError> {
    let now = Utc::now();
    let step = ProcessStep {
        id: Uuid::new_v4(),
        name: input.name,
        description: input.description,
        sequence_order: input.sequence_order,
        business_rules: input.business_rules,
        required_inputs: input.required_inputs,
        produced_outputs: input.produced_outputs,
        role_id: input.role_id,
        process_id: id,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    let saved = ProcessStepRepository::save(&service.process_repo(), &step).await?;
    Ok(Json(saved.into()))
}

#[utoipa::path(
    post,
    path = "/api/business-architecture/value-streams",
    tag = "business-architecture",
    request_body = CreateValueStreamInput,
    responses(
        (status = 201, description = "创建价值流", body = ValueStreamDto),
    )
)]
pub async fn create_value_stream(
    State(service): State<Arc<BusinessArchitectureService>>,
    Json(input): Json<CreateValueStreamInput>,
) -> Result<Json<ValueStreamDto>, ApiError> {
    let now = Utc::now();
    let id = Uuid::new_v4();
    let vs = ValueStream {
        id,
        business_version: "0.1.0".to_string(),
        status: LifecycleStatus::Active,
        name: input.name,
        description: input.description,
        triggering_event: input.triggering_event,
        end_deliverable: input.end_deliverable,
        owner_id: input.owner_id,
        importance: input.importance,
        stakeholders: input.stakeholders,
        performance_metrics: input.performance_metrics,
        created_by: input.created_by,
        updated_by: input.created_by,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    let saved = ValueStreamRepository::save(&service.value_stream_repo(), &vs).await?;
    Ok(Json(saved.into()))
}

#[utoipa::path(
    get,
    path = "/api/business-architecture/value-streams",
    tag = "business-architecture",
    params(PageInput),
    responses(
        (status = 200, description = "价值流列表", body = Page<ValueStreamDto>),
    )
)]
pub async fn list_value_streams(
    State(service): State<Arc<BusinessArchitectureService>>,
    Query(params): Query<PageInput>,
) -> Result<Json<Page<ValueStreamDto>>, ApiError> {
    let (vss, total) = service
        .value_stream_repo()
        .list_active(params.page, params.per_page)
        .await?;
    let items: Vec<ValueStreamDto> = vss.into_iter().map(Into::into).collect();
    Ok(Json(Page::new(items, params.page, params.per_page, total)))
}

#[utoipa::path(
    get,
    path = "/api/business-architecture/value-streams/{id}",
    tag = "business-architecture",
    responses(
        (status = 200, description = "价值流详情", body = ValueStreamDto),
        (status = 404, description = "未找到"),
    )
)]
pub async fn get_value_stream(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ValueStreamDto>, ApiError> {
    let vs = service
        .value_stream_repo()
        .find_by_id(id)
        .await?
        .ok_or_else(|| ApiError(shared_common::AppError::NotFound("value stream not found".into())))?;
    Ok(Json(vs.into()))
}

#[utoipa::path(
    put,
    path = "/api/business-architecture/value-streams/{id}",
    tag = "business-architecture",
    request_body = UpdateValueStreamInput,
    responses(
        (status = 200, description = "更新成功", body = ValueStreamDto),
        (status = 404, description = "未找到"),
    )
)]
pub async fn update_value_stream(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateValueStreamInput>,
) -> Result<Json<ValueStreamDto>, ApiError> {
    let repo = service.value_stream_repo();
    let mut vs = repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| ApiError(shared_common::AppError::NotFound("value stream not found".into())))?;

    if let Some(name) = input.name {
        vs.name = name;
    }
    if let Some(desc) = input.description {
        vs.description = desc;
    }
    if let Some(te) = input.triggering_event {
        vs.triggering_event = te;
    }
    if let Some(ed) = input.end_deliverable {
        vs.end_deliverable = ed;
    }
    if let Some(owner) = input.owner_id {
        vs.owner_id = owner;
    }
    if let Some(imp) = input.importance {
        vs.importance = imp;
    }
    if let Some(s) = input.stakeholders {
        vs.stakeholders = s;
    }
    if let Some(pm) = input.performance_metrics {
        vs.performance_metrics = pm;
    }
    if let Some(updated_by) = input.updated_by {
        vs.updated_by = Some(updated_by);
    }
    vs.updated_at = Utc::now();

    let saved = ValueStreamRepository::save(&repo, &vs).await?;
    Ok(Json(saved.into()))
}

#[utoipa::path(
    delete,
    path = "/api/business-architecture/value-streams/{id}",
    tag = "business-architecture",
    responses(
        (status = 204, description = "删除成功"),
        (status = 404, description = "未找到"),
    )
)]
pub async fn delete_value_stream(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    ValueStreamRepository::soft_delete(&service.value_stream_repo(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/api/business-architecture/value-streams/{id}/stages",
    tag = "business-architecture",
    responses(
        (status = 200, description = "阶段列表", body = Vec<ValueStreamStageDto>),
    )
)]
pub async fn get_value_stream_stages(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ValueStreamStageDto>>, ApiError> {
    let stages = service.value_stream_repo().find_by_value_stream(id).await?;
    let items: Vec<ValueStreamStageDto> = stages.into_iter().map(Into::into).collect();
    Ok(Json(items))
}

#[utoipa::path(
    post,
    path = "/api/business-architecture/value-streams/{id}/stages",
    tag = "business-architecture",
    request_body = CreateValueStreamStageInput,
    responses(
        (status = 201, description = "创建阶段", body = ValueStreamStageDto),
    )
)]
pub async fn create_value_stream_stage(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path(id): Path<Uuid>,
    Json(input): Json<CreateValueStreamStageInput>,
) -> Result<Json<ValueStreamStageDto>, ApiError> {
    let now = Utc::now();
    let stage = ValueStreamStage {
        id: Uuid::new_v4(),
        name: input.name,
        sequence_order: input.sequence_order,
        input: input.input,
        output: input.output,
        value_stream_id: id,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    let saved = ValueStreamStageRepository::save(&service.value_stream_repo(), &stage).await?;
    Ok(Json(saved.into()))
}

#[utoipa::path(
    post,
    path = "/api/business-architecture/value-streams/stages/{stage_id}/capabilities",
    tag = "business-architecture",
    request_body = LinkStageCapabilityInput,
    responses(
        (status = 201, description = "关联成功"),
    )
)]
pub async fn link_stage_capability(
    State(service): State<Arc<BusinessArchitectureService>>,
    Path((stage_id,)): Path<(Uuid,)>,
    Json(input): Json<LinkStageCapabilityInput>,
) -> Result<StatusCode, ApiError> {
    service
        .value_stream_repo()
        .link_stage_capability(stage_id, input.capability_id)
        .await?;
    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    post,
    path = "/api/business-architecture/analysis/gap",
    tag = "business-architecture",
    request_body = GapAnalysisInput,
    responses(
        (status = 200, description = "差距分析结果", body = GapAnalysisResult),
    )
)]
pub async fn gap_analysis(
    State(service): State<Arc<BusinessArchitectureService>>,
    Json(input): Json<GapAnalysisInput>,
) -> Result<Json<GapAnalysisResult>, ApiError> {
    let (caps, _) = service.capability_repo().list_active(1, 10000).await?;
    let target_rank = maturity_rank(input.target_maturity);

    let gaps: Vec<Gap> = caps
        .iter()
        .filter(|c| maturity_rank(c.maturity) < target_rank)
        .map(|c| Gap {
            area: format!("capability: {}", c.name),
            current: format!("{:?}", c.maturity).to_lowercase(),
            target: format!("{:?}", input.target_maturity).to_lowercase(),
            recommendation: format!(
                "Improve '{}' maturity from {:?} to {:?}",
                c.name, c.maturity, input.target_maturity
            ),
        })
        .collect();

    let summary = format!("Found {} capability gaps below target maturity.", gaps.len());

    Ok(Json(GapAnalysisResult { gaps, summary }))
}

#[utoipa::path(
    post,
    path = "/api/business-architecture/analysis/redundancy",
    tag = "business-architecture",
    request_body = RedundancyInput,
    responses(
        (status = 200, description = "冗余分析结果", body = RedundancyResult),
    )
)]
pub async fn redundancy_analysis(
    State(service): State<Arc<BusinessArchitectureService>>,
    Json(input): Json<RedundancyInput>,
) -> Result<Json<RedundancyResult>, ApiError> {
    let threshold = input.threshold.unwrap_or(0.8);
    let (caps, _) = service.capability_repo().list_active(1, 10000).await?;

    let mut duplicates: Vec<Duplicate> = Vec::new();
    let mut mergeable: Vec<Mergeable> = Vec::new();
    let mut seen: std::collections::HashSet<usize> = std::collections::HashSet::new();

    for i in 0..caps.len() {
        if seen.contains(&i) {
            continue;
        }
        let mut group: Vec<usize> = vec![i];
        for j in (i + 1)..caps.len() {
            if seen.contains(&j) {
                continue;
            }
            let sim = name_similarity(&caps[i].name, &caps[j].name);
            if sim >= threshold {
                group.push(j);
            }
        }
        if group.len() > 1 {
            let ids: Vec<Uuid> = group.iter().map(|&idx| caps[idx].id).collect();
            for &idx in &group {
                seen.insert(idx);
            }
            duplicates.push(Duplicate {
                entity_type: "capability".to_string(),
                ids: ids.clone(),
                reason: format!("Names similar above {} threshold", threshold),
            });
            if ids.len() >= 2 {
                for k in 1..ids.len() {
                    mergeable.push(Mergeable {
                        entity_type: "capability".to_string(),
                        source_id: ids[k],
                        target_id: ids[0],
                        reason: "Similar name, candidate for merge".to_string(),
                    });
                }
            }
        }
    }

    let summary = format!(
        "Found {} duplicate groups and {} mergeable pairs.",
        duplicates.len(),
        mergeable.len()
    );

    Ok(Json(RedundancyResult {
        duplicates,
        mergeable,
        summary,
    }))
}
