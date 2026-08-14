use axum::response::IntoResponse;
use sea_orm::DatabaseConnection;
use seaography::{Builder, BuilderContext, GuardAction, LifecycleHooks, LifecycleHooksInterface, OperationType, RelatedEntityFilter, RelationBuilder, TimeLibrary, TypesMapConfig};
use uuid::Uuid;
use async_graphql::ErrorExtensions;

use user_management::infrastructure::persistence::entities::{
    oauth_authorization_code, refresh_token, user,
};
use business_architecture::infrastructure::persistence::entities::{
    business_capability, business_process, capability_process, process_step, stage_capability,
    value_stream, value_stream_stage, space, space_member,
    application_component, application_process, application_process_step,
    capability_realization,
    organizational_unit, business_role, functional_module, application_interface,
    assignment, participation, module_containment, interface_exposure,
    process_reference, orchestration,
};
use business_architecture::application::value_stream_service::ValueStreamService;
use business_architecture::application::space_service::SpaceService;
use business_architecture::domain::value_stream::entity::ValueStream as DomainValueStream;
use business_architecture::domain::value_stream::repository::ValueStreamRepository;
use business_architecture::domain::process::entity::BusinessProcess as DomainBusinessProcess;
use business_architecture::domain::process::repository::ProcessRepository;
use business_architecture::domain::space::entity::{Space as DomainSpace, SpaceMember as DomainSpaceMember};
use business_architecture::domain::error::DomainError;
use business_architecture::infrastructure::persistence::value_stream_repo::SeaOrmValueStreamRepo;
use business_architecture::infrastructure::persistence::process_repo::SeaOrmProcessRepo;
use business_architecture::infrastructure::persistence::space_repo::{SeaOrmSpaceRepo, SeaOrmMembershipRepo};
use business_architecture::infrastructure::persistence::space_audit_repo::SeaOrmAuditLogRepo;
use shared_common::enums::ValueStreamImportance;
use shared_common::enums::{SpaceRole, SpaceVisibility};
use shared_common::enums::{
    AutomationLevel, BusinessValueRating, CapabilityLevel, CapabilityStatus, CostRating,
    LifecycleStatus, MaturityLevel,
    ApplicationComponentType, ApplicationComponentStatus, ApplicationProcessTrigger,
    RaciRole, OrganizationalUnitType, FunctionalModuleStatus, ApplicationInterfaceProtocol,
    CapabilityRealizationTargetType,
};

pub type GraphqlSchema = async_graphql::dynamic::Schema;

// ============================================================================
// GraphQL Auth Guard (seaography LifecycleHooks)
// ============================================================================

/// User-management entities: only Admin can manage users.
const USER_ENTITIES: &[&str] = &[
    "users",
    "refresh_tokens",
    "oauth_authorization_codes",
];

/// Entities whose membership/identity data should not be exposed to anonymous
/// readers. Reading these requires an authenticated session; user records and
/// space membership records additionally require admin (cross-tenant
/// enumeration is prevented; space-scoped reads go through custom queries that
/// enforce membership/visibility).
const PRIVATE_READ_ENTITIES: &[&str] = &[
    "users",
    "refresh_tokens",
    "oauth_authorization_codes",
    "space_members",
];

/// Entities that require admin to read even when authenticated via the
/// seaography auto-generated query. This prevents cross-tenant enumeration of
/// membership and bypass of private-space row-level ACL: business content is
/// read through custom space-scoped queries (`spaces`/`spaceById`/`*BySpace`)
/// that enforce visibility + membership instead. `space_members` is admin-only
/// via auto-query; the membership-scoped `spaceMembersBySpace` custom query
/// enforces its own access.
const ADMIN_READ_ENTITIES: &[&str] = &[
    "space_members",
    "organizations",
    "value_streams",
    "business_capabilities",
    "business_processes",
    "process_steps",
    "value_stream_stages",
    "capability_processes",
    "stage_capabilities",
    "application_components",
    "application_processes",
    "application_process_steps",
    "capability_realizations",
    "organizational_units",
    "business_roles",
    "functional_modules",
    "application_interfaces",
    "assignments",
    "participations",
    "module_containments",
    "interface_exposures",
    "process_references",
    "orchestrations",
];

/// Fields hidden from all users (including Admin) in queries.
const HIDDEN_FIELDS: &[(&str, &str)] = &[
    ("users", "password_hash"),
    ("refresh_tokens", "token_hash"),
    ("oauth_authorization_codes", "code_hash"),
];

/// Fields restricted to Admin only.
const ADMIN_ONLY_FIELDS: &[(&str, &str)] = &[
    ("users", "email"),
];

pub struct GraphqlAuthGuard;

impl LifecycleHooksInterface for GraphqlAuthGuard {
    fn entity_guard(
        &self,
        ctx: &async_graphql::dynamic::ResolverContext,
        entity: &str,
        action: OperationType,
    ) -> GuardAction {
        let claims = ctx.data_opt::<crate::middleware::Claims>();
        tracing::debug!(
            "entity_guard: entity={}, action={:?}, has_claims={}",
            entity, action, claims.is_some()
        );

        match action {
            OperationType::Read => {
                // Anonymous users may read only entities that are neither
                // private-read nor admin-read. Business content and spaces are
                // admin-only via the auto-generated query (cross-tenant / private
                // row-level ACL); space-scoped custom queries enforce visibility +
                // membership and are the intended read path for non-admins.
                if PRIVATE_READ_ENTITIES.contains(&entity) {
                    let Some(claims) = claims else {
                        return GuardAction::Block(Some("Authentication required.".to_string()));
                    };
                    let role = claims.user_role();
                    if USER_ENTITIES.contains(&entity) && !role.can_manage_users() {
                        return GuardAction::Block(Some(
                            "Only admins can read user records.".to_string(),
                        ));
                    }
                    if ADMIN_READ_ENTITIES.contains(&entity) && !role.is_admin() {
                        return GuardAction::Block(Some(
                            "Only admins can read this resource directly.".to_string(),
                        ));
                    }
                } else if ADMIN_READ_ENTITIES.contains(&entity) {
                    // Admin-only entities that are not in PRIVATE_READ_ENTITIES
                    // (business content / spaces): block anonymous and non-admin.
                    let Some(claims) = claims else {
                        return GuardAction::Block(Some(
                            "Only admins can read this resource directly.".to_string(),
                        ));
                    };
                    if !claims.user_role().is_admin() {
                        return GuardAction::Block(Some(
                            "Only admins can read this resource directly.".to_string(),
                        ));
                    }
                }
                GuardAction::Allow
            }

            OperationType::Create => {
                let Some(claims) = claims else {
                    return GuardAction::Block(Some("Authentication required for mutations.".to_string()));
                };
                let role = claims.user_role();

                if USER_ENTITIES.contains(&entity) && !role.can_manage_users() {
                    return GuardAction::Block(Some(
                        "Only admins can create user records.".to_string(),
                    ));
                }

                if !role.can_create() {
                    return GuardAction::Block(Some(
                        "Viewers cannot create resources.".to_string(),
                    ));
                }

                GuardAction::Allow
            }

            OperationType::Update => {
                let Some(claims) = claims else {
                    return GuardAction::Block(Some("Authentication required for mutations.".to_string()));
                };
                let role = claims.user_role();

                if USER_ENTITIES.contains(&entity) && !role.can_manage_users() {
                    return GuardAction::Block(Some(
                        "Only admins can update user records.".to_string(),
                    ));
                }

                if !role.can_update() {
                    return GuardAction::Block(Some(
                        "Viewers cannot update resources.".to_string(),
                    ));
                }

                GuardAction::Allow
            }

            OperationType::Delete => {
                let Some(claims) = claims else {
                    return GuardAction::Block(Some("Authentication required for mutations.".to_string()));
                };
                let role = claims.user_role();

                if USER_ENTITIES.contains(&entity) && !role.can_manage_users() {
                    return GuardAction::Block(Some(
                        "Only admins can delete user records.".to_string(),
                    ));
                }

                if !role.can_delete() {
                    return GuardAction::Block(Some(
                        "Viewers cannot delete resources.".to_string(),
                    ));
                }

                GuardAction::Allow
            }
        }
    }

    fn field_guard(
        &self,
        ctx: &async_graphql::dynamic::ResolverContext,
        entity: &str,
        field: &str,
        _action: OperationType,
    ) -> GuardAction {
        if HIDDEN_FIELDS.iter().any(|&(e, f)| e == entity && f == field) {
            return GuardAction::Block(Some(
                format!("Field '{}' on '{}' is not accessible.", field, entity),
            ));
        }

        if ADMIN_ONLY_FIELDS.iter().any(|&(e, f)| e == entity && f == field) {
            let claims = ctx.data_opt::<crate::middleware::Claims>();
            let Some(claims) = claims else {
                return GuardAction::Block(Some(
                    format!("Field '{}' on '{}' requires authentication.", field, entity),
                ));
            };
            if !claims.user_role().is_admin() {
                return GuardAction::Block(Some(
                    format!("Field '{}' on '{}' is admin-only.", field, entity),
                ));
            }
        }

        GuardAction::Allow
    }
}

// ============================================================================
// JWT extraction helper
// ============================================================================

/// Extract JWT Claims from Authorization header.
/// Returns None if no valid JWT is present (public queries still work).
pub fn extract_claims_from_headers(
    headers: &axum::http::HeaderMap,
    jwt_secret: &str,
) -> Option<crate::middleware::Claims> {
    use jsonwebtoken::{decode, DecodingKey, Validation};

    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))?;

    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = true;

    decode::<crate::middleware::Claims>(
        auth_header,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .ok()
    .map(|data| data.claims)
}

// ============================================================================
// GraphQL Service (POST + GET handler)
// ============================================================================

/// Custom tower Service that handles GraphQL requests on /graphql.
/// - GET: returns GraphiQL interactive IDE HTML
/// - POST: executes GraphQL query/mutation with JWT extraction
///
/// JWT Claims are injected into async_graphql context for LifecycleHooks entity_guard.
/// Queries are public (no JWT required), mutations require valid JWT.
#[derive(Clone)]
pub struct GraphQLService {
    schema: GraphqlSchema,
    jwt_secret: String,
    endpoint: String,
}

impl GraphQLService {
    pub fn new(schema: GraphqlSchema, jwt_secret: String) -> Self {
        Self {
            schema,
            jwt_secret,
            endpoint: "/graphql".to_string(),
        }
    }
}

impl tower::Service<axum::extract::Request> for GraphQLService {
    type Response = axum::response::Response;
    type Error = std::convert::Infallible;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: axum::extract::Request) -> Self::Future {
        let schema = self.schema.clone();
        let jwt_secret = self.jwt_secret.clone();
        let endpoint = self.endpoint.clone();

        Box::pin(async move {
            match req.method() {
                // GET → GraphiQL interactive IDE
                &axum::http::Method::GET => {
                    let html = async_graphql::http::GraphiQLSource::build()
                        .endpoint(&endpoint)
                        .finish();
                    Ok(axum::response::Html(html).into_response())
                }
                // POST → Execute GraphQL query/mutation
                &axum::http::Method::POST => {
                    let has_jwt =
                        crate::graphql::extract_claims_from_headers(req.headers(), &jwt_secret);

                    let bytes = match axum::body::to_bytes(req.into_body(), 1024 * 1024).await {
                        Ok(b) => b,
                        Err(_) => {
                            return Ok((
                                axum::http::StatusCode::BAD_REQUEST,
                                axum::Json(serde_json::json!({"error": "body_too_large"})),
                            )
                                .into_response());
                        }
                    };

                    let mut request: async_graphql::Request =
                        match serde_json::from_slice(&bytes) {
                            Ok(r) => r,
                            Err(e) => {
                                return Ok((
                                    axum::http::StatusCode::BAD_REQUEST,
                                    axum::Json(serde_json::json!({"error":
                                        format!("invalid request: {e}")})),
                                )
                                    .into_response());
                            }
                        };

                    // Inject Claims into GraphQL context if JWT was valid.
                    // entity_guard handles auth for seaography mutations;
                    // custom ValueStream mutations check Claims explicitly.
                    if let Some(claims) = has_jwt {
                        request = request.data(claims);
                    }

                    let response = schema.execute(request).await;
                    Ok(axum::Json(response).into_response())
                }
                _ => Ok(axum::http::StatusCode::METHOD_NOT_ALLOWED.into_response()),
            }
        })
    }
}

// ============================================================================
// Schema builder
// ============================================================================

#[derive(Copy, Clone, Debug, sea_orm::EnumIter)]
enum NoRelation {}

impl RelationBuilder for NoRelation {
    fn get_relation_name(&self, _: &'static BuilderContext) -> String {
        unreachable!()
    }
    fn get_relation(
        &self,
        _: &'static BuilderContext,
    ) -> async_graphql::dynamic::Field {
        unreachable!()
    }
    fn get_related_entity_filter(
        &self,
        _: &'static BuilderContext,
    ) -> seaography::RelatedEntityFilterField {
        unreachable!()
    }
}

fn register_entity<T>(builder: &mut Builder)
where
    T: sea_orm::EntityTrait,
    <T as sea_orm::EntityTrait>::Model: Sync,
{
    let context = builder.context;
    let filter = RelatedEntityFilter::<T>::build::<NoRelation>(context);
    builder.register_entity::<T>(vec![], &filter);
}

fn register_entity_with_mutations<T, A>(builder: &mut Builder)
where
    T: sea_orm::EntityTrait,
    <T as sea_orm::EntityTrait>::Model: Sync,
    <T as sea_orm::EntityTrait>::Model: sea_orm::IntoActiveModel<A>,
    A: sea_orm::ActiveModelTrait<Entity = T> + sea_orm::ActiveModelBehavior + Send + 'static,
{
    register_entity::<T>(builder);
    builder.register_entity_mutations::<T, A>();
}

// ============================================================================
// Domain → SeaORM Model conversion (for FieldValue::owned_any)
// ============================================================================

/// Convert a domain ValueStream back to a SeaORM Model so that
/// seaography's field resolvers can downcast and resolve all fields.
fn domain_vs_to_model(vs: &DomainValueStream) -> value_stream::Model {
    vs.into()
}

/// Convert a domain BusinessProcess back to a SeaORM Model so that
/// seaography's field resolvers can downcast and resolve all fields.
fn domain_process_to_model(p: &DomainBusinessProcess) -> business_process::Model {
    p.into()
}

// ============================================================================
// Capability↔Process version-anchoring result types
// ============================================================================

/// One capability↔process link enriched with the process name, business
/// version, lifecycle status and derived validity (`valid = status ∈
/// {active, deprecated}`). Used by the `capabilityProcessRelations` query.
#[derive(Clone, Debug)]
struct CapabilityProcessRelation {
    capability_id: String,
    process_id: String,
    logical_id: String,
    process_name: String,
    business_version: String,
    status: String,
    valid: bool,
}

/// A capability that referenced the old process version before a publish.
#[derive(Clone, Debug)]
struct AffectedProcessLinkOutput {
    capability_id: String,
    capability_name: String,
    old_version: String,
    new_version: String,
}

/// Result of the `processPublishVersion` mutation: the newly created active
/// version plus the capability links that now point at the deprecated old
/// version.
#[derive(Clone, Debug)]
struct ProcessPublishVersionOutput {
    id: String,
    business_version: String,
    status: String,
    affected_links: Vec<AffectedProcessLinkOutput>,
}

// ============================================================================
// Custom ValueStream Domain Mutations
// ============================================================================

/// Check authentication and authorization for ValueStream domain mutations.
/// This mirrors the entity_guard logic that seaography applies to auto-generated mutations.
fn check_value_stream_auth(
    ctx: &async_graphql::dynamic::ResolverContext,
    action: OperationType,
) -> async_graphql::Result<()> {
    let claims = ctx
        .data_opt::<crate::middleware::Claims>()
        .ok_or_else(|| async_graphql::Error::new("Authentication required for mutations."))?;

    let role = claims.user_role();

    let allowed = match action {
        OperationType::Create => role.can_create(),
        OperationType::Update => role.can_update(),
        OperationType::Delete => role.can_delete(),
        OperationType::Read => true,
    };

    if !allowed {
        return Err(async_graphql::Error::new(
            "Insufficient permissions for this operation.",
        ));
    }

    Ok(())
}

/// Parse a GraphQL enum/string into ValueStreamImportance.
fn parse_importance(s: &str) -> async_graphql::Result<ValueStreamImportance> {
    match s {
        "Critical" => Ok(ValueStreamImportance::Critical),
        "High" => Ok(ValueStreamImportance::High),
        "Medium" => Ok(ValueStreamImportance::Medium),
        "Low" => Ok(ValueStreamImportance::Low),
        _ => Err(async_graphql::Error::new(format!(
            "Invalid importance: '{}'. Expected: Critical, High, Medium, Low",
            s
        ))),
    }
}

/// Require the actor to be a member (editor/owner) of the given space, or an admin.
/// This enforces the space-level ACL that the coarse entity_guard cannot.
async fn ensure_space_edit_access(
    ctx: &async_graphql::dynamic::ResolverContext<'_>,
    db: &DatabaseConnection,
    space_id: Uuid,
) -> async_graphql::Result<()> {
    let claims = require_claims(ctx)?;
    let service = space_service(db);
    service
        .ensure_can_edit(space_id, claims.user_id, claims.user_role())
        .await
        .map_err(domain_err_to_graphql)
}

/// Require the actor to be the entity's owner (or a global admin).
/// Space editors who are not the entity owner are rejected even though they
/// pass `ensure_space_edit_access` — entity-level ownership protects the
/// "I built this, nobody else should edit it" guarantee.
async fn ensure_entity_owner_or_admin(
    ctx: &async_graphql::dynamic::ResolverContext<'_>,
    owner_id: Option<Uuid>,
) -> async_graphql::Result<()> {
    let claims = require_claims(ctx)?;
    if claims.user_role().is_admin() {
        return Ok(());
    }
    if owner_id == Some(claims.user_id) {
        return Ok(());
    }
    Err(domain_err_to_graphql(DomainError::NotEntityOwner))
}

/// Require `user_id` to be a member (any role) of `space_id`, or return a
/// "not a member" GraphQL error. Used to validate `transferOwnership` targets.
async fn ensure_space_member(
    ctx: &async_graphql::dynamic::ResolverContext<'_>,
    db: &DatabaseConnection,
    space_id: Uuid,
    user_id: Uuid,
) -> async_graphql::Result<()> {
    let claims = require_claims(ctx)?;
    if claims.user_role().is_admin() {
        return Ok(());
    }
    let service = space_service(db);
    let membership = service
        .my_membership(space_id, user_id)
        .await
        .map_err(domain_err_to_graphql)?;
    if membership.is_none() {
        return Err(graphql_err_with_code(
            &DomainError::NotSpaceMember,
            "FORBIDDEN_SPACE_NOT_MEMBER",
        ));
    }
    Ok(())
}

/// Register custom ValueStream mutations that go through the domain model.
/// These replace seaography's auto-generated CRUD mutations for value_stream.
///
/// Custom mutations skip `entity_guard` (it only applies to seaography-generated
/// resolvers), so each mutation manually calls `check_value_stream_auth` for
/// role-based authorization and `ensure_space_edit_access` for space-level ACL.
fn register_value_stream_domain_mutations(builder: &mut Builder) {
    use async_graphql::dynamic::{Field, FieldFuture, FieldValue, InputValue, TypeRef};

    // ── valueStreamCreate ──────────────────────────────────────────────
    let create_field = Field::new(
        "valueStreamCreate",
        TypeRef::named_nn("ValueStreams"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Create)?;

                let db = ctx.data::<DatabaseConnection>()?;

                let space_id_str = ctx.args.try_get("spaceId")?.string()?;
                let space_id = Uuid::parse_str(space_id_str)
                    .map_err(|e| async_graphql::Error::new(format!("Invalid UUID: {e}")))?;
                let name = ctx.args.try_get("name")?.string()?.to_owned();
                let description = ctx.args.get("description").and_then(|v| v.string().ok()).map(|s| s.to_owned());
                let business_version = ctx.args.try_get("businessVersion")?.string()?.to_owned();
                let importance = parse_importance(ctx.args.try_get("importance")?.enum_name()?)?;
                let stakeholders = match ctx.args.get("stakeholders") {
                    Some(_) => Some(parse_string_vec_arg(&ctx, "stakeholders")?),
                    None => None,
                };
                let triggering_event = ctx.args.get("triggeringEvent").and_then(|v| v.string().ok()).map(|s| s.to_owned());
                let end_deliverable = ctx.args.get("endDeliverable").and_then(|v| v.string().ok()).map(|s| s.to_owned());
                let performance_metrics = parse_string_string_map_arg(&ctx, "performanceMetrics")?;

                ensure_space_edit_access(&ctx, db, space_id).await?;

                // The creator automatically becomes the entity owner; a
                // client-supplied ownerId is ignored (transfer is only allowed
                // via the dedicated transferOwnership mutations).
                let claims = require_claims(&ctx)?;
                let owner_id = Some(claims.user_id);

                let repo = SeaOrmValueStreamRepo::new(db.clone());
                let service = ValueStreamService::new(repo);
                let vs = service
                    .create(space_id, name, description, business_version, importance, stakeholders, triggering_event, end_deliverable, owner_id, performance_metrics)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;

                let model = domain_vs_to_model(&vs);
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("businessVersion", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("importance", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("stakeholders", TypeRef::named_nn_list(TypeRef::STRING)))
    .argument(InputValue::new("triggeringEvent", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("endDeliverable", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("performanceMetrics", TypeRef::named(TypeRef::STRING)));

    builder.mutations.push(create_field);

    // ── valueStreamUpdate ──────────────────────────────────────────────
    let update_field = Field::new(
        "valueStreamUpdate",
        TypeRef::named_nn("ValueStreams"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Update)?;

                let db = ctx.data::<DatabaseConnection>()?;

                let id_str = ctx.args.try_get("id")?.string()?;
                let id = Uuid::parse_str(id_str)
                    .map_err(|e| async_graphql::Error::new(format!("Invalid UUID: {e}")))?;

                let name = ctx.args.get("name").and_then(|v| v.string().ok()).map(|s| s.to_owned());
                let description = match ctx.args.get("description") {
                    Some(v) if v.is_null() => Some(None),
                    Some(v) => v.string().ok().map(|s| Some(s.to_owned())),
                    None => None,
                };
                let importance = match ctx.args.get("importance") {
                    Some(v) if !v.is_null() => Some(parse_importance(v.enum_name()?)?),
                    _ => None,
                };
                let stakeholders = match ctx.args.get("stakeholders") {
                    Some(_) => Some(parse_string_vec_arg(&ctx, "stakeholders")?),
                    None => None,
                };
                let triggering_event = match ctx.args.get("triggeringEvent") {
                    Some(v) if v.is_null() => Some(None),
                    Some(v) => v.string().ok().map(|s| Some(s.to_owned())),
                    None => None,
                };
                let end_deliverable = match ctx.args.get("endDeliverable") {
                    Some(v) if v.is_null() => Some(None),
                    Some(v) => v.string().ok().map(|s| Some(s.to_owned())),
                    None => None,
                };
                // Absent → no change; explicit null → clear (empty map).
                let performance_metrics = parse_string_string_map_arg(&ctx, "performanceMetrics")?;

                let repo = SeaOrmValueStreamRepo::new(db.clone());
                // Enforce space-level ACL + entity ownership before mutating.
                let existing = repo
                    .find_by_id(id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Value stream not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;
                ensure_entity_owner_or_admin(&ctx, existing.owner_id).await?;

                let service = ValueStreamService::new(repo);
                let vs = service
                    .update(id, name, description, importance, stakeholders, triggering_event, end_deliverable, None, performance_metrics)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;

                let model = domain_vs_to_model(&vs);
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("importance", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("stakeholders", TypeRef::named_nn_list(TypeRef::STRING)))
    .argument(InputValue::new("triggeringEvent", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("endDeliverable", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("performanceMetrics", TypeRef::named(TypeRef::STRING)));

    builder.mutations.push(update_field);

    // ── valueStreamArchive ─────────────────────────────────────────────
    let archive_field = Field::new(
        "valueStreamArchive",
        TypeRef::named_nn(TypeRef::BOOLEAN),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Delete)?;

                let db = ctx.data::<DatabaseConnection>()?;

                let id_str = ctx.args.try_get("id")?.string()?;
                let id = Uuid::parse_str(id_str)
                    .map_err(|e| async_graphql::Error::new(format!("Invalid UUID: {e}")))?;

                let repo = SeaOrmValueStreamRepo::new(db.clone());
                // Enforce space-level ACL + entity ownership before archiving.
                let existing = repo
                    .find_by_id(id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Value stream not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;
                ensure_entity_owner_or_admin(&ctx, existing.owner_id).await?;

                let service = ValueStreamService::new(repo);
                service
                    .archive(id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;

                Ok(Some(async_graphql::Value::Boolean(true)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));

    builder.mutations.push(archive_field);

    // ── valueStreamDelete ─────────────────────────────────────────────
    // Soft-delete: sets `deleted_at` (row kept for audit/recovery), unlike
    // `valueStreamArchive` which only flips the lifecycle status. Same auth
    // chain as archive (role permission + space ACL + entity owner/admin).
    let delete_field = Field::new(
        "valueStreamDelete",
        TypeRef::named_nn(TypeRef::BOOLEAN),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Delete)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;

                let repo = SeaOrmValueStreamRepo::new(db.clone());
                // Enforce space-level ACL + entity ownership before deleting.
                let existing = repo
                    .find_by_id(id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Value stream not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;
                ensure_entity_owner_or_admin(&ctx, existing.owner_id).await?;

                let service = ValueStreamService::new(repo);
                service
                    .delete(id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;

                Ok(Some(async_graphql::Value::Boolean(true)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));

    builder.mutations.push(delete_field);

    // ── valueStreamCreateVersion ───────────────────────────────────────
    let create_version_field = Field::new(
        "valueStreamCreateVersion",
        TypeRef::named_nn("ValueStreams"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Create)?;

                let db = ctx.data::<DatabaseConnection>()?;

                let current_id_str = ctx.args.try_get("currentId")?.string()?;
                let current_id = Uuid::parse_str(current_id_str)
                    .map_err(|e| async_graphql::Error::new(format!("Invalid UUID: {e}")))?;

                let new_version = ctx.args.try_get("newVersion")?.string()?.to_owned();
                let new_name = ctx.args.get("newName").and_then(|v| v.string().ok()).map(|s| s.to_owned());
                let new_description = ctx.args.get("newDescription").and_then(|v| v.string().ok()).map(|s| s.to_owned());

                let repo = SeaOrmValueStreamRepo::new(db.clone());
                // Enforce space-level ACL + entity ownership before versioning.
                let existing = repo
                    .find_by_id(current_id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Value stream not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;
                ensure_entity_owner_or_admin(&ctx, existing.owner_id).await?;

                let service = ValueStreamService::new(repo);
                let vs = service
                    .create_version(current_id, new_version, new_name, new_description)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;

                let model = domain_vs_to_model(&vs);
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("currentId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("newVersion", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("newName", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("newDescription", TypeRef::named(TypeRef::STRING)));

    builder.mutations.push(create_version_field);

    // ── valueStreamTransferOwnership ─────────────────────────────────
    // Only the current owner (or an admin) may transfer. The target must be
    // a member of the same space. The new owner is set as `owner_id` of the
    // value stream; stages continue to follow the parent value stream's owner.
    let transfer_field = Field::new(
        "valueStreamTransferOwnership",
        TypeRef::named_nn("ValueStreams"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Update)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;
                let new_owner_id = parse_uuid_arg(&ctx, "newOwnerId")?;

                let repo = SeaOrmValueStreamRepo::new(db.clone());
                let existing = repo
                    .find_by_id(id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Value stream not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;
                ensure_entity_owner_or_admin(&ctx, existing.owner_id).await?;
                ensure_space_member(&ctx, db, existing.space_id, new_owner_id).await?;

                let service = ValueStreamService::new(repo);
                let vs = service
                    .transfer_ownership(id, new_owner_id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;

                let model = domain_vs_to_model(&vs);
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("newOwnerId", TypeRef::named_nn(TypeRef::STRING)));

    builder.mutations.push(transfer_field);
}

// ============================================================================
// Custom Space Domain Mutations
// ============================================================================

fn domain_space_to_model(s: &DomainSpace) -> space::Model {
    space::Model {
        id: s.id,
        name: s.name.clone(),
        description: s.description.clone(),
        visibility: s.visibility,
        created_at: s.created_at,
        updated_at: s.updated_at,
        deleted_at: s.deleted_at,
    }
}

fn domain_member_to_model(m: &DomainSpaceMember) -> space_member::Model {
    space_member::Model {
        space_id: m.space_id,
        user_id: m.user_id,
        role: match m.role {
            SpaceRole::Owner => "owner".to_owned(),
            SpaceRole::Editor => "editor".to_owned(),
        },
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

/// Require an authenticated session and return the claims.
fn require_claims<'a>(ctx: &'a async_graphql::dynamic::ResolverContext<'a>) -> async_graphql::Result<&'a crate::middleware::Claims> {
    ctx.data_opt::<crate::middleware::Claims>()
        .ok_or_else(|| async_graphql::Error::new("Authentication required for mutations."))
}

fn parse_space_role(s: &str) -> async_graphql::Result<SpaceRole> {
    SpaceRole::from_str(s)
        .ok_or_else(|| async_graphql::Error::new(format!("Invalid space role: {s}")))
}

fn parse_uuid_arg<'a>(ctx: &'a async_graphql::dynamic::ResolverContext<'a>, name: &str) -> async_graphql::Result<Uuid> {
    let s = ctx.args.try_get(name)?.string()?;
    Uuid::parse_str(s).map_err(|e| async_graphql::Error::new(format!("Invalid UUID for {name}: {e}")))
}

/// Parse a `visibility` argument into `SpaceVisibility`. Returns an error for
/// unrecognized values so a typo (e.g. `"Private"`, `"PRIVATE"`, `"internal"`)
/// surfaces as a GraphQL error rather than silently widening a private space to
/// public. When the argument is omitted, defaults to `Private` (least
/// privilege) so a caller bug that drops the argument cannot accidentally
/// expose a space publicly.
fn parse_visibility_arg(
    ctx: &async_graphql::dynamic::ResolverContext<'_>,
    name: &str,
) -> Result<SpaceVisibility, async_graphql::Error> {
    match ctx.args.get(name).and_then(|v| v.string().ok()) {
        Some("public") => Ok(SpaceVisibility::Public),
        Some("private") => Ok(SpaceVisibility::Private),
        Some(other) => Err(async_graphql::Error::new(format!(
            "Invalid visibility value '{other}': expected 'public' or 'private'"
        ))),
        // When the argument is omitted, default to the least-privileged
        // visibility (Private) rather than Public. This prevents a caller bug
        // that omits the argument from accidentally exposing a space publicly.
        None => Ok(SpaceVisibility::Private),
    }
}

/// Build a `SpaceService` wired to the SeaORM repos (space, membership, audit).
fn space_service(db: &DatabaseConnection) -> SpaceService<SeaOrmSpaceRepo, SeaOrmMembershipRepo, SeaOrmAuditLogRepo> {
    SpaceService::new(
        SeaOrmSpaceRepo::new(db.clone()),
        SeaOrmMembershipRepo::new(db.clone()),
        SeaOrmAuditLogRepo::new(db.clone()),
    )
    .with_strict_audit()
}

/// Wrap a `DomainError` as a GraphQL error with a semantic `extensions.code`.
fn graphql_err_with_code(e: &DomainError, code: &str) -> async_graphql::Error {
    async_graphql::Error::new(e.to_string()).extend_with(|_err, extensions| {
        extensions.set("code", code.to_owned());
    })
}

/// Map a domain access error to a GraphQL error carrying a semantic code.
/// Used by the read/edit guards so the frontend can branch on `extensions.code`.
fn domain_err_to_graphql(e: DomainError) -> async_graphql::Error {
    let code = match &e {
        DomainError::NotSpaceMember => "FORBIDDEN_SPACE_NOT_MEMBER",
        DomainError::NotSpaceEditor => "FORBIDDEN_SPACE_NOT_EDITOR",
        DomainError::NotSpaceOwner => "FORBIDDEN_SPACE_NOT_OWNER",
        DomainError::SpaceQuotaExceeded => "SPACE_QUOTA_EXCEEDED",
        DomainError::SpaceNotFound => "SPACE_NOT_FOUND",
        DomainError::ProcessNotFound | DomainError::ValueStreamNotFound
        | DomainError::ProcessVersionNotFound | DomainError::CapabilityNotFound
        | DomainError::InvitationNotFound => "NOT_FOUND",
        DomainError::SpaceNameEmpty | DomainError::Validation(_) => "VALIDATION_ERROR",
        DomainError::Semver(_) => "SEMVER_ERROR",
        DomainError::Database(_) => "INTERNAL_ERROR",
        DomainError::AuditLogFailed(_) => "AUDIT_LOG_FAILED",
        DomainError::InvalidTransition { .. } | DomainError::CannotModifyArchived { .. }
        | DomainError::CannotReferenceArchived | DomainError::AlreadyMember
        | DomainError::CannotRemoveLastOwner | DomainError::NotOwner
        | DomainError::NotEntityOwner => "FORBIDDEN",
    };
    // Database and audit-log errors may contain sensitive internal details
    // (SQL fragments, table/column names, constraint names, repository error
    // messages). Log the full message server-side but return a generic message
    // to the client.
    match &e {
        DomainError::Database(msg) => {
            tracing::error!("Database error: {msg}");
            async_graphql::Error::new("Internal server error").extend_with(|_err, extensions| {
                extensions.set("code", code.to_owned());
            })
        }
        DomainError::AuditLogFailed(msg) => {
            tracing::error!("Audit log failure: {msg}");
            async_graphql::Error::new("Audit log failure").extend_with(|_err, extensions| {
                extensions.set("code", code.to_owned());
            })
        }
        _ => graphql_err_with_code(&e, code),
    }
}

/// Map a SeaORM database error to a GraphQL error carrying a semantic
/// `extensions.code` of `INTERNAL_ERROR`. The raw database message is logged
/// server-side but never sent to the client to avoid leaking SQL fragments,
/// table/column names, or constraint names.
fn db_err_to_graphql(e: impl std::fmt::Display) -> async_graphql::Error {
    tracing::error!("Database error: {e}");
    async_graphql::Error::new("Internal server error").extend_with(|_err, extensions| {
        extensions.set("code", "INTERNAL_ERROR".to_owned());
    })
}

fn register_space_domain_mutations(builder: &mut Builder) {
    use async_graphql::dynamic::{Field, FieldFuture, FieldValue, InputValue, TypeRef};

    // ── spaceCreate ────────────────────────────────────────────────────
    let create = Field::new(
        "spaceCreate",
        TypeRef::named_nn("Organizations"),
        |ctx| {
            FieldFuture::new(async move {
                let claims = require_claims(&ctx)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let name = ctx.args.try_get("name")?.string()?.to_owned();
                let description = ctx.args.get("description").and_then(|v| v.string().ok()).map(|s| s.to_owned());
                let visibility = parse_visibility_arg(&ctx, "visibility")?;

                let service = space_service(db);
                let space_obj = service
                    .create_space(claims.user_id, claims.user_role(), name, description, visibility)
                    .await
                    .map_err(domain_err_to_graphql)?;
                Ok(Some(FieldValue::owned_any(domain_space_to_model(&space_obj))))
            })
        },
    )
    .argument(InputValue::new("name", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("visibility", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(create);

    // ── spaceUpdate ────────────────────────────────────────────────────
    // Visibility is intentionally NOT mutable here — use `spaceSetVisibility`.
    let update = Field::new(
        "spaceUpdate",
        TypeRef::named_nn("Organizations"),
        |ctx| {
            FieldFuture::new(async move {
                let claims = require_claims(&ctx)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "id")?;
                let name = ctx.args.get("name").and_then(|v| v.string().ok()).map(|s| s.to_owned());
                let description = match ctx.args.get("description") {
                    Some(v) if v.is_null() => Some(None),
                    Some(v) => v.string().ok().map(|s| Some(s.to_owned())),
                    None => None,
                };

                let service = space_service(db);
                let space_obj = service
                    .update_space(space_id, claims.user_id, claims.user_role(), name, description)
                    .await
                    .map_err(domain_err_to_graphql)?;
                Ok(Some(FieldValue::owned_any(domain_space_to_model(&space_obj))))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(update);

    // ── spaceSetVisibility ─────────────────────────────────────────────
    // Independent mutation (R4): only owner or Admin may change visibility.
    // Records an audit log entry (best-effort) inside the service.
    let set_visibility = Field::new(
        "spaceSetVisibility",
        TypeRef::named_nn("Organizations"),
        |ctx| {
            FieldFuture::new(async move {
                let claims = require_claims(&ctx)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "id")?;
                let visibility = parse_visibility_arg(&ctx, "visibility")?;
                let service = space_service(db);
                let space_obj = service
                    .set_visibility(space_id, claims.user_id, claims.user_role(), visibility)
                    .await
                    .map_err(domain_err_to_graphql)?;
                Ok(Some(FieldValue::owned_any(domain_space_to_model(&space_obj))))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("visibility", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(set_visibility);

    // ── spaceArchive ───────────────────────────────────────────────────
    let archive = Field::new(
        "spaceArchive",
        TypeRef::named_nn(TypeRef::BOOLEAN),
        |ctx| {
            FieldFuture::new(async move {
                let claims = require_claims(&ctx)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "id")?;
                let service = space_service(db);
                service
                    .archive_space(space_id, claims.user_id, claims.user_role())
                    .await
                    .map_err(domain_err_to_graphql)?;
                Ok(Some(async_graphql::Value::Boolean(true)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(archive);

    // ── spaceAddMember ─────────────────────────────────────────────────
    let add_member = Field::new(
        "spaceAddMember",
        TypeRef::named_nn("SpaceMembers"),
        |ctx| {
            FieldFuture::new(async move {
                let claims = require_claims(&ctx)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let user_id = parse_uuid_arg(&ctx, "userId")?;
                let role = parse_space_role(ctx.args.try_get("role")?.enum_name()?)?;

                let service = space_service(db);
                let member = service
                    .add_member(space_id, claims.user_id, claims.user_role(), user_id, role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                Ok(Some(FieldValue::owned_any(domain_member_to_model(&member))))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("userId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("role", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(add_member);

    // ── spaceRemoveMember ──────────────────────────────────────────────
    let remove_member = Field::new(
        "spaceRemoveMember",
        TypeRef::named_nn(TypeRef::BOOLEAN),
        |ctx| {
            FieldFuture::new(async move {
                let claims = require_claims(&ctx)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let user_id = parse_uuid_arg(&ctx, "userId")?;
                let service = space_service(db);
                service
                    .remove_member(space_id, claims.user_id, claims.user_role(), user_id)
                    .await
                    .map_err(domain_err_to_graphql)?;
                Ok(Some(async_graphql::Value::Boolean(true)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("userId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(remove_member);
}
// ============================================================================
// Custom BusinessCapability Domain Mutations (space-level ACL enforced)
// ============================================================================

fn parse_enum<T: serde::de::DeserializeOwned>(s: &str) -> async_graphql::Result<T> {
    serde_json::from_value(serde_json::Value::String(s.to_owned()))
        .map_err(|e| async_graphql::Error::new(format!("Invalid enum value '{s}': {e}")))
}

fn get_enum_arg(ctx: &async_graphql::dynamic::ResolverContext<'_>, name: &str) -> Option<String> {
    let v = ctx.args.get(name)?;
    v.enum_name().ok().map(|s| s.to_owned())
}

fn register_capability_domain_mutations(builder: &mut Builder) {
    use async_graphql::dynamic::{Field, FieldFuture, FieldValue, InputValue, TypeRef};
    use sea_orm::ActiveValue::{NotSet, Set};
    use sea_orm::{EntityTrait, ActiveModelTrait};

    // ── capabilityCreate ─────────────────────────────────────────────
    let create = Field::new(
        "capabilityCreate",
        TypeRef::named_nn("BusinessCapabilities"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Create)?;
                let db = ctx.data::<DatabaseConnection>()?;

                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let name = ctx.args.try_get("name")?.string()?.to_owned();
                let description = ctx.args.get("description").and_then(|v| v.string().ok()).map(|s| s.to_owned()).unwrap_or_default();
                let level = parse_enum::<CapabilityLevel>(ctx.args.try_get("level")?.enum_name()?)?;
                let maturity = parse_enum::<MaturityLevel>(ctx.args.try_get("maturity")?.enum_name()?)?;
                let business_value = parse_enum::<BusinessValueRating>(ctx.args.try_get("businessValue")?.enum_name()?)?;
                let cost = match get_enum_arg(&ctx, "cost") {
                    Some(v) => parse_enum::<CostRating>(&v)?,
                    None => CostRating::Low,
                };
                let capability_status = match get_enum_arg(&ctx, "capabilityStatus") {
                    Some(v) => parse_enum::<CapabilityStatus>(&v)?,
                    None => CapabilityStatus::Active,
                };
                let metrics = parse_string_string_map_arg(&ctx, "metrics")?;

                ensure_space_edit_access(&ctx, db, space_id).await?;

                // The creator automatically becomes the entity owner; transfer
                // is only allowed via capabilityTransferOwnership.
                let claims = require_claims(&ctx)?;
                let owner_id = Some(claims.user_id);

                let now = chrono::Utc::now();
                let am = business_capability::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    logical_id: Set(Uuid::now_v7()),
                    business_version: Set("v1.0".to_owned()),
                    status: Set(LifecycleStatus::Active),
                    capability_status: Set(capability_status),
                    name: Set(name),
                    description: Set(description),
                    level: Set(level),
                    maturity: Set(maturity),
                    business_value: Set(business_value),
                    cost: Set(cost),
                    metrics: Set(metrics),
                    owner_id: Set(owner_id),
                    created_by: NotSet,
                    updated_by: NotSet,
                    created_at: Set(now),
                    updated_at: Set(now),
                    deleted_at: NotSet,
                    space_id: Set(space_id),
                };
                let model = am
                    .insert(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("level", TypeRef::named_nn("CapabilityLevelEnum")))
    .argument(InputValue::new("maturity", TypeRef::named_nn("MaturityLevelEnum")))
    .argument(InputValue::new("businessValue", TypeRef::named_nn("BusinessValueRatingEnum")))
    .argument(InputValue::new("cost", TypeRef::named("CostRatingEnum")))
    .argument(InputValue::new("capabilityStatus", TypeRef::named("CapabilityStatusEnum")))
    .argument(InputValue::new("metrics", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(create);

    // ── capabilityUpdate ─────────────────────────────────────────────
    let update = Field::new(
        "capabilityUpdate",
        TypeRef::named_nn("BusinessCapabilities"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Update)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;

                let existing = business_capability::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Capability not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;
                ensure_entity_owner_or_admin(&ctx, existing.owner_id).await?;

                let mut am: business_capability::ActiveModel = existing.into();
                if let Some(v) = ctx.args.get("name").and_then(|v| v.string().ok()) {
                    am.name = Set(v.to_owned());
                }
                if let Some(v) = ctx.args.get("description").and_then(|v| v.string().ok()) {
                    am.description = Set(v.to_owned());
                }
                if let Some(v) = get_enum_arg(&ctx, "level") {
                    am.level = Set(parse_enum::<CapabilityLevel>(&v)?);
                }
                if let Some(v) = get_enum_arg(&ctx, "maturity") {
                    am.maturity = Set(parse_enum::<MaturityLevel>(&v)?);
                }
                if let Some(v) = get_enum_arg(&ctx, "businessValue") {
                    am.business_value = Set(parse_enum::<BusinessValueRating>(&v)?);
                }
                if let Some(v) = get_enum_arg(&ctx, "cost") {
                    am.cost = Set(parse_enum::<CostRating>(&v)?);
                }
                if let Some(v) = get_enum_arg(&ctx, "capabilityStatus") {
                    am.capability_status = Set(parse_enum::<CapabilityStatus>(&v)?);
                }
                if let Some(metrics) = parse_string_string_map_arg(&ctx, "metrics")? {
                    am.metrics = Set(Some(metrics));
                }
                am.updated_at = Set(chrono::Utc::now());
                let model = am
                    .update(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("level", TypeRef::named("CapabilityLevelEnum")))
    .argument(InputValue::new("maturity", TypeRef::named("MaturityLevelEnum")))
    .argument(InputValue::new("businessValue", TypeRef::named("BusinessValueRatingEnum")))
    .argument(InputValue::new("cost", TypeRef::named("CostRatingEnum")))
    .argument(InputValue::new("capabilityStatus", TypeRef::named("CapabilityStatusEnum")))
    .argument(InputValue::new("metrics", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(update);

    // ── capabilityDelete ─────────────────────────────────────────────
    let delete = Field::new(
        "capabilityDelete",
        TypeRef::named_nn(TypeRef::BOOLEAN),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Delete)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;

                let existing = business_capability::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Capability not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;
                ensure_entity_owner_or_admin(&ctx, existing.owner_id).await?;

                business_capability::Entity::delete_by_id(id)
                    .exec(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(async_graphql::Value::Boolean(true)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);

    // ── capabilityTransferOwnership ──────────────────────────────────
    let transfer = Field::new(
        "capabilityTransferOwnership",
        TypeRef::named_nn("BusinessCapabilities"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Update)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;
                let new_owner_id = parse_uuid_arg(&ctx, "newOwnerId")?;

                let existing = business_capability::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Capability not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;
                ensure_entity_owner_or_admin(&ctx, existing.owner_id).await?;
                ensure_space_member(&ctx, db, existing.space_id, new_owner_id).await?;

                let mut am: business_capability::ActiveModel = existing.into();
                am.owner_id = Set(Some(new_owner_id));
                am.updated_at = Set(chrono::Utc::now());
                let model = am
                    .update(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("newOwnerId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(transfer);
}

// ============================================================================
// Custom BusinessProcess Domain Mutations (space-level ACL enforced)
// ============================================================================

fn register_process_domain_mutations(builder: &mut Builder) {
    use async_graphql::dynamic::{Field, FieldFuture, FieldValue, InputValue, TypeRef};
    use sea_orm::ActiveValue::{NotSet, Set};
    use sea_orm::{EntityTrait, ActiveModelTrait};

    // ── processCreate ────────────────────────────────────────────────
    let create = Field::new(
        "processCreate",
        TypeRef::named_nn("BusinessProcesses"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Create)?;
                let db = ctx.data::<DatabaseConnection>()?;

                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let name = ctx.args.try_get("name")?.string()?.to_owned();
                let description = ctx.args.get("description").and_then(|v| v.string().ok()).map(|s| s.to_owned()).unwrap_or_default();
                let inputs = parse_string_vec_arg(&ctx, "inputs")?;
                let outputs = parse_string_vec_arg(&ctx, "outputs")?;
                let sla = ctx.args.get("sla").and_then(|v| v.string().ok()).map(|s| s.to_owned());
                let cycle_time: Option<i64> = ctx.args.get("cycleTime").and_then(|v| v.i64().ok());
                let cost_per_transaction: Option<f64> = ctx.args.get("costPerTransaction").and_then(|v| v.f64().ok());
                let automation_level = match get_enum_arg(&ctx, "automationLevel") {
                    Some(v) => Some(parse_enum::<AutomationLevel>(&v)?),
                    None => None,
                };
                let maturity = match get_enum_arg(&ctx, "maturity") {
                    Some(v) => Some(parse_enum::<MaturityLevel>(&v)?),
                    None => None,
                };
                ensure_space_edit_access(&ctx, db, space_id).await?;

                // The creator automatically becomes the entity owner; transfer
                // is only allowed via processTransferOwnership.
                let claims = require_claims(&ctx)?;
                let owner_id = Some(claims.user_id);

                let now = chrono::Utc::now();
                let am = business_process::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    logical_id: Set(Uuid::now_v7()),
                    // Strict semver ("1.0.0"): `publish_new_version` bumps the
                    // minor component with semver, which rejects a leading 'v'.
                    business_version: Set("1.0.0".to_owned()),
                    status: Set(LifecycleStatus::Active),
                    name: Set(name),
                    description: Set(description),
                    inputs: Set(inputs),
                    outputs: Set(outputs),
                    sla: Set(sla),
                    cost_per_transaction: Set(cost_per_transaction),
                    cycle_time: Set(cycle_time),
                    automation_level: Set(automation_level),
                    maturity: Set(maturity),
                    owner_id: Set(owner_id),
                    created_by: NotSet,
                    updated_by: NotSet,
                    created_at: Set(now),
                    updated_at: Set(now),
                    deleted_at: NotSet,
                    space_id: Set(space_id),
                };
                let model = am
                    .insert(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("inputs", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("outputs", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("sla", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("cycleTime", TypeRef::named(TypeRef::INT)))
    .argument(InputValue::new("costPerTransaction", TypeRef::named(TypeRef::FLOAT)))
    .argument(InputValue::new("automationLevel", TypeRef::named("AutomationLevelEnum")))
    .argument(InputValue::new("maturity", TypeRef::named("MaturityLevelEnum")));
    builder.mutations.push(create);

    // ── processUpdate ────────────────────────────────────────────────
    let update = Field::new(
        "processUpdate",
        TypeRef::named_nn("BusinessProcesses"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Update)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;

                let existing = business_process::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Process not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;
                ensure_entity_owner_or_admin(&ctx, existing.owner_id).await?;

                let mut am: business_process::ActiveModel = existing.into();
                if let Some(v) = ctx.args.get("name").and_then(|v| v.string().ok()) {
                    am.name = Set(v.to_owned());
                }
                if let Some(v) = ctx.args.get("description").and_then(|v| v.string().ok()) {
                    am.description = Set(v.to_owned());
                }
                if ctx.args.get("inputs").is_some() {
                    am.inputs = Set(parse_string_vec_arg(&ctx, "inputs")?);
                }
                if ctx.args.get("outputs").is_some() {
                    am.outputs = Set(parse_string_vec_arg(&ctx, "outputs")?);
                }
                if let Some(v) = ctx.args.get("sla").and_then(|v| v.string().ok()) {
                    am.sla = Set(Some(v.to_owned()));
                }
                if let Some(v) = ctx.args.get("cycleTime").and_then(|v| v.i64().ok()) {
                    am.cycle_time = Set(Some(v));
                }
                if let Some(v) = ctx.args.get("costPerTransaction").and_then(|v| v.f64().ok()) {
                    am.cost_per_transaction = Set(Some(v));
                }
                if let Some(v) = get_enum_arg(&ctx, "automationLevel") {
                    am.automation_level = Set(Some(parse_enum::<AutomationLevel>(&v)?));
                }
                if let Some(v) = get_enum_arg(&ctx, "maturity") {
                    am.maturity = Set(Some(parse_enum::<MaturityLevel>(&v)?));
                }
                am.updated_at = Set(chrono::Utc::now());
                let model = am
                    .update(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("inputs", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("outputs", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("sla", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("cycleTime", TypeRef::named(TypeRef::INT)))
    .argument(InputValue::new("costPerTransaction", TypeRef::named(TypeRef::FLOAT)))
    .argument(InputValue::new("automationLevel", TypeRef::named("AutomationLevelEnum")))
    .argument(InputValue::new("maturity", TypeRef::named("MaturityLevelEnum")));
    builder.mutations.push(update);

    // ── processDelete ────────────────────────────────────────────────
    let delete = Field::new(
        "processDelete",
        TypeRef::named_nn(TypeRef::BOOLEAN),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Delete)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;

                let existing = business_process::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Process not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;
                ensure_entity_owner_or_admin(&ctx, existing.owner_id).await?;

                business_process::Entity::delete_by_id(id)
                    .exec(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(async_graphql::Value::Boolean(true)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);

    // ── processTransferOwnership ─────────────────────────────────────
    let transfer = Field::new(
        "processTransferOwnership",
        TypeRef::named_nn("BusinessProcesses"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Update)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;
                let new_owner_id = parse_uuid_arg(&ctx, "newOwnerId")?;

                let existing = business_process::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Process not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;
                ensure_entity_owner_or_admin(&ctx, existing.owner_id).await?;
                ensure_space_member(&ctx, db, existing.space_id, new_owner_id).await?;

                let mut am: business_process::ActiveModel = existing.into();
                am.owner_id = Set(Some(new_owner_id));
                am.updated_at = Set(chrono::Utc::now());
                let model = am
                    .update(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("newOwnerId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(transfer);

    // ── processPublishVersion ─────────────────────────────────────────
    // Publish a new minor version of the active process identified by
    // logicalId. The old active row becomes Deprecated (compatibility window)
    // and the result lists the capability links now pointing at the old
    // version so the UI can warn about version anchoring.
    let publish = Field::new(
        "processPublishVersion",
        TypeRef::named_nn("ProcessPublishVersionResult"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Create)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let logical_id = parse_uuid_arg(&ctx, "logicalId")?;

                let repo = SeaOrmProcessRepo::new(db.clone());
                // Enforce space-level ACL + entity ownership before publishing.
                let active = repo
                    .find_active_by_logical_id(logical_id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("No active process found for this logicalId."))?;
                ensure_space_edit_access(&ctx, db, active.space_id).await?;
                ensure_entity_owner_or_admin(&ctx, active.owner_id).await?;

                let result = repo
                    .publish_new_version(logical_id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;

                let model = domain_process_to_model(&result.new_process);
                Ok(Some(FieldValue::owned_any(ProcessPublishVersionOutput {
                    id: model.id.to_string(),
                    business_version: model.business_version,
                    status: format!("{:?}", model.status).to_lowercase(),
                    affected_links: result
                        .affected_links
                        .into_iter()
                        .map(|l| AffectedProcessLinkOutput {
                            capability_id: l.capability_id.to_string(),
                            capability_name: l.capability_name,
                            old_version: l.old_version,
                            new_version: l.new_version,
                        })
                        .collect(),
                })))
            })
        },
    )
    .argument(InputValue::new("logicalId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(publish);

    // ── processArchive ────────────────────────────────────────────────
    // Only `Deprecated → Archived` is allowed: the Deprecated compatibility
    // window cannot be skipped by archiving an Active process directly.
    let archive = Field::new(
        "processArchive",
        TypeRef::named_nn(TypeRef::BOOLEAN),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Update)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;

                let existing = business_process::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Process not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;
                ensure_entity_owner_or_admin(&ctx, existing.owner_id).await?;

                let repo = SeaOrmProcessRepo::new(db.clone());
                repo.archive(id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(async_graphql::Value::Boolean(true)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(archive);
}
// ============================================================================
// Custom Sub-Entity Domain Mutations (space-level ACL enforced)
// ----------------------------------------------------------------------------
// process_step, value_stream_stage, capability_process, and stage_capability are
// children of the value-stream/capability/process parents. They have no
// `space_id` column of their own, so the owning space is resolved from the
// referenced parent before any write and `ensure_space_edit_access` is invoked.
// This closes the tenant-isolation gap left by seaography's auto-CRUD, which
// only checked the coarse global `UserRole`.
// ============================================================================

/// Resolve the `space_id` of a business process.
async fn space_of_process(db: &DatabaseConnection, process_id: Uuid) -> async_graphql::Result<Uuid> {
    use sea_orm::EntityTrait;
    let p = business_process::Entity::find_by_id(process_id)
        .one(db)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Process not found."))?;
    Ok(p.space_id)
}

/// Resolve the `space_id` of a value stream.
async fn space_of_value_stream(db: &DatabaseConnection, vs_id: Uuid) -> async_graphql::Result<Uuid> {
    use sea_orm::EntityTrait;
    let vs = value_stream::Entity::find_by_id(vs_id)
        .one(db)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Value stream not found."))?;
    Ok(vs.space_id)
}

/// Resolve the `space_id` of a capability.
async fn space_of_capability(db: &DatabaseConnection, cap_id: Uuid) -> async_graphql::Result<Uuid> {
    use sea_orm::EntityTrait;
    let c = business_capability::Entity::find_by_id(cap_id)
        .one(db)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Capability not found."))?;
    Ok(c.space_id)
}

/// Resolve the `space_id` of a value-stream stage (via its value stream).
async fn space_of_stage(db: &DatabaseConnection, stage_id: Uuid) -> async_graphql::Result<Uuid> {
    use sea_orm::EntityTrait;
    let stage = value_stream_stage::Entity::find_by_id(stage_id)
        .one(db)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Value stream stage not found."))?;
    space_of_value_stream(db, stage.value_stream_id).await
}

/// Resolve the `owner_id` of a business process (for sub-entity ownership checks).
async fn owner_of_process(db: &DatabaseConnection, process_id: Uuid) -> async_graphql::Result<Option<Uuid>> {
    use sea_orm::EntityTrait;
    let p = business_process::Entity::find_by_id(process_id)
        .one(db)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Process not found."))?;
    Ok(p.owner_id)
}

/// Resolve the `owner_id` of a value stream (for sub-entity ownership checks).
async fn owner_of_value_stream(db: &DatabaseConnection, vs_id: Uuid) -> async_graphql::Result<Option<Uuid>> {
    use sea_orm::EntityTrait;
    let vs = value_stream::Entity::find_by_id(vs_id)
        .one(db)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Value stream not found."))?;
    Ok(vs.owner_id)
}

/// Resolve the `owner_id` of a capability (for sub-entity ownership checks).
async fn owner_of_capability(db: &DatabaseConnection, cap_id: Uuid) -> async_graphql::Result<Option<Uuid>> {
    use sea_orm::EntityTrait;
    let c = business_capability::Entity::find_by_id(cap_id)
        .one(db)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Capability not found."))?;
    Ok(c.owner_id)
}

/// Resolve the `owner_id` of the parent value stream of a stage.
async fn owner_of_stage_parent(db: &DatabaseConnection, stage_id: Uuid) -> async_graphql::Result<Option<Uuid>> {
    use sea_orm::EntityTrait;
    let stage = value_stream_stage::Entity::find_by_id(stage_id)
        .one(db)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Value stream stage not found."))?;
    owner_of_value_stream(db, stage.value_stream_id).await
}

/// Resolve the `space_id` of an application component.
async fn space_of_application_component(
    db: &DatabaseConnection,
    id: Uuid,
) -> async_graphql::Result<Uuid> {
    use sea_orm::EntityTrait;
    let c = application_component::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Application component not found."))?;
    Ok(c.space_id)
}

/// Resolve the `space_id` of an application process.
async fn space_of_application_process(
    db: &DatabaseConnection,
    id: Uuid,
) -> async_graphql::Result<Uuid> {
    use sea_orm::EntityTrait;
    let p = application_process::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Application process not found."))?;
    Ok(p.space_id)
}

/// Resolve the `space_id` of an application process step (via its process).
async fn space_of_application_process_step(
    db: &DatabaseConnection,
    id: Uuid,
) -> async_graphql::Result<Uuid> {
    use sea_orm::EntityTrait;
    let step = application_process_step::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Application process step not found."))?;
    space_of_application_process(db, step.process_id).await
}

/// Resolve the `space_id` of a business process step (via its process).
async fn space_of_process_step(db: &DatabaseConnection, step_id: Uuid) -> async_graphql::Result<Uuid> {
    use sea_orm::EntityTrait;
    let step = process_step::Entity::find_by_id(step_id)
        .one(db)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Process step not found."))?;
    space_of_process(db, step.process_id).await
}

/// Resolve the `space_id` of an organizational unit.
async fn space_of_organizational_unit(
    db: &DatabaseConnection,
    id: Uuid,
) -> async_graphql::Result<Uuid> {
    use sea_orm::EntityTrait;
    let o = organizational_unit::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Organizational unit not found."))?;
    Ok(o.space_id)
}

/// Resolve the `space_id` of a business role.
async fn space_of_business_role(
    db: &DatabaseConnection,
    id: Uuid,
) -> async_graphql::Result<Uuid> {
    use sea_orm::EntityTrait;
    let r = business_role::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Business role not found."))?;
    Ok(r.space_id)
}

/// Resolve the `space_id` of a functional module.
async fn space_of_functional_module(
    db: &DatabaseConnection,
    id: Uuid,
) -> async_graphql::Result<Uuid> {
    use sea_orm::EntityTrait;
    let m = functional_module::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Functional module not found."))?;
    Ok(m.space_id)
}

/// Resolve the `space_id` of an application interface.
async fn space_of_application_interface(
    db: &DatabaseConnection,
    id: Uuid,
) -> async_graphql::Result<Uuid> {
    use sea_orm::EntityTrait;
    let i = application_interface::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?
        .ok_or_else(|| async_graphql::Error::new("Application interface not found."))?;
    Ok(i.space_id)
}

/// Parse an optional `[String]` GraphQL argument into a `StringVec`.
fn parse_string_vec_arg(
    ctx: &async_graphql::dynamic::ResolverContext<'_>,
    name: &str,
) -> async_graphql::Result<shared_common::value_objects::StringVec> {
    match ctx.args.get(name) {
        Some(v) if v.is_null() => Ok(Default::default()),
        Some(v) => {
            let list = v
                .list()?
                .iter()
                .map(|item| item.string().map(|s| s.to_owned()))
                .collect::<async_graphql::Result<Vec<String>>>()?;
            Ok(shared_common::value_objects::StringVec(list))
        }
        None => Ok(Default::default()),
    }
}

/// Parse an optional `StringStringMap` argument from a JSON string.
/// Returns `None` when the argument is absent or explicitly `null`.
fn parse_string_string_map_arg(
    ctx: &async_graphql::dynamic::ResolverContext<'_>,
    name: &str,
) -> async_graphql::Result<Option<shared_common::value_objects::StringStringMap>> {
    match ctx.args.get(name) {
        Some(v) if v.is_null() => Ok(Some(Default::default())),
        Some(v) => {
            let json = v.string()?;
            let map: std::collections::HashMap<String, String> = serde_json::from_str(json)
                .map_err(|e| async_graphql::Error::new(format!("Invalid JSON for '{name}': {e}")))?;
            Ok(Some(shared_common::value_objects::StringStringMap(map)))
        }
        None => Ok(None),
    }
}

fn register_sub_entity_domain_mutations(builder: &mut Builder) {
    use async_graphql::dynamic::{Field, FieldFuture, FieldValue, InputValue, TypeRef};
    use sea_orm::ActiveValue::{NotSet, Set};
    use sea_orm::{EntityTrait, ActiveModelTrait};

    // ── processStepCreate ────────────────────────────────────────────
    let create = Field::new(
        "processStepCreate",
        TypeRef::named_nn("ProcessSteps"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Create)?;
                let db = ctx.data::<DatabaseConnection>()?;

                let process_id = parse_uuid_arg(&ctx, "processId")?;
                let space_id = space_of_process(db, process_id).await?;
                ensure_space_edit_access(&ctx, db, space_id).await?;
                let process_owner = owner_of_process(db, process_id).await?;
                ensure_entity_owner_or_admin(&ctx, process_owner).await?;

                let name = ctx.args.try_get("name")?.string()?.to_owned();
                let description = ctx
                    .args
                    .get("description")
                    .and_then(|v| v.string().ok())
                    .map(|s| s.to_owned())
                    .unwrap_or_default();
                let sequence_order: i32 = ctx.args.try_get("sequenceOrder")?.i64()? as i32;
                let business_rules = parse_string_vec_arg(&ctx, "businessRules")?;
                let required_inputs = parse_string_vec_arg(&ctx, "requiredInputs")?;
                let produced_outputs = parse_string_vec_arg(&ctx, "producedOutputs")?;
                let role_id = ctx
                    .args
                    .get("roleId")
                    .and_then(|v| v.string().ok())
                    .and_then(|s| Uuid::parse_str(s).ok());

                let now = chrono::Utc::now();
                let am = process_step::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    name: Set(name),
                    description: Set(description),
                    sequence_order: Set(sequence_order),
                    business_rules: Set(business_rules),
                    required_inputs: Set(required_inputs),
                    produced_outputs: Set(produced_outputs),
                    role_id: Set(role_id),
                    process_id: Set(process_id),
                    created_at: Set(now),
                    updated_at: Set(now),
                    deleted_at: NotSet,
                };
                let model = am
                    .insert(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("processId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("sequenceOrder", TypeRef::named_nn(TypeRef::INT)))
    .argument(InputValue::new("businessRules", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("requiredInputs", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("producedOutputs", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("roleId", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(create);

    // ── processStepUpdate ────────────────────────────────────────────
    let update = Field::new(
        "processStepUpdate",
        TypeRef::named_nn("ProcessSteps"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Update)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;

                let existing = process_step::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Process step not found."))?;
                let space_id = space_of_process(db, existing.process_id).await?;
                ensure_space_edit_access(&ctx, db, space_id).await?;
                let process_owner = owner_of_process(db, existing.process_id).await?;
                ensure_entity_owner_or_admin(&ctx, process_owner).await?;

                let mut am: process_step::ActiveModel = existing.into();
                if let Some(v) = ctx.args.get("name").and_then(|v| v.string().ok()) {
                    am.name = Set(v.to_owned());
                }
                if let Some(v) = ctx.args.get("description").and_then(|v| v.string().ok()) {
                    am.description = Set(v.to_owned());
                }
                if let Some(v) = ctx.args.get("sequenceOrder").and_then(|v| v.i64().ok()).map(|v| v as i32) {
                    am.sequence_order = Set(v);
                }
                if ctx.args.get("businessRules").is_some() {
                    am.business_rules = Set(parse_string_vec_arg(&ctx, "businessRules")?);
                }
                if ctx.args.get("requiredInputs").is_some() {
                    am.required_inputs = Set(parse_string_vec_arg(&ctx, "requiredInputs")?);
                }
                if ctx.args.get("producedOutputs").is_some() {
                    am.produced_outputs = Set(parse_string_vec_arg(&ctx, "producedOutputs")?);
                }
                match ctx.args.get("roleId") {
                    Some(v) if v.is_null() => am.role_id = Set(None),
                    Some(v) => {
                        if let Ok(s) = v.string() {
                            am.role_id = Set(Uuid::parse_str(s).ok());
                        }
                    }
                    None => {}
                }
                am.updated_at = Set(chrono::Utc::now());
                let model = am
                    .update(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("sequenceOrder", TypeRef::named(TypeRef::INT)))
    .argument(InputValue::new("businessRules", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("requiredInputs", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("producedOutputs", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("roleId", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(update);

    // ── processStepDelete ────────────────────────────────────────────
    let delete = Field::new(
        "processStepDelete",
        TypeRef::named_nn(TypeRef::BOOLEAN),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Delete)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;

                let existing = process_step::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Process step not found."))?;
                let space_id = space_of_process(db, existing.process_id).await?;
                ensure_space_edit_access(&ctx, db, space_id).await?;
                let process_owner = owner_of_process(db, existing.process_id).await?;
                ensure_entity_owner_or_admin(&ctx, process_owner).await?;

                process_step::Entity::delete_by_id(id)
                    .exec(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(async_graphql::Value::Boolean(true)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);

    // ── valueStreamStageCreate ───────────────────────────────────────
    let create = Field::new(
        "valueStreamStageCreate",
        TypeRef::named_nn("ValueStreamStages"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Create)?;
                let db = ctx.data::<DatabaseConnection>()?;

                let value_stream_id = parse_uuid_arg(&ctx, "valueStreamId")?;
                let space_id = space_of_value_stream(db, value_stream_id).await?;
                ensure_space_edit_access(&ctx, db, space_id).await?;
                // Sub-entity writes follow the parent value stream's owner.
                let vs_owner = owner_of_value_stream(db, value_stream_id).await?;
                ensure_entity_owner_or_admin(&ctx, vs_owner).await?;

                let name = ctx.args.try_get("name")?.string()?.to_owned();
                let sequence_order: i32 = ctx.args.try_get("sequenceOrder")?.i64()? as i32;
                let input = ctx
                    .args
                    .get("input")
                    .and_then(|v| v.string().ok())
                    .map(|s| s.to_owned());
                let output = ctx
                    .args
                    .get("output")
                    .and_then(|v| v.string().ok())
                    .map(|s| s.to_owned());
                let description = ctx
                    .args
                    .get("description")
                    .and_then(|v| v.string().ok())
                    .map(|s| s.to_owned());
                let entry_criteria = ctx
                    .args
                    .get("entryCriteria")
                    .and_then(|v| v.string().ok())
                    .map(|s| s.to_owned());
                let exit_criteria = ctx
                    .args
                    .get("exitCriteria")
                    .and_then(|v| v.string().ok())
                    .map(|s| s.to_owned());
                let owner_id = ctx
                    .args
                    .get("ownerId")
                    .and_then(|v| v.string().ok())
                    .and_then(|s| Uuid::parse_str(s).ok());
                let objective_metrics = parse_string_string_map_arg(&ctx, "objectiveMetrics")?;
                let key_metrics = parse_string_string_map_arg(&ctx, "keyMetrics")?;

                // Domain rule: sequence_order must be unique within the value stream.
                let repo = SeaOrmValueStreamRepo::new(db.clone());
                let existing_stages = repo
                    .find_stages_by_value_stream(value_stream_id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                let stage = business_architecture::domain::value_stream::entity::ValueStreamStage::create(
                    Uuid::now_v7(),
                    value_stream_id,
                    name,
                    sequence_order,
                    input,
                    output,
                    description,
                    objective_metrics,
                    entry_criteria,
                    exit_criteria,
                    owner_id,
                    key_metrics,
                    chrono::Utc::now(),
                );
                stage
                    .ensure_sequence_order_unique(&existing_stages)
                    .map_err(domain_err_to_graphql)?;

                let now = chrono::Utc::now();
                let am = value_stream_stage::ActiveModel {
                    id: Set(stage.id),
                    name: Set(stage.name),
                    sequence_order: Set(stage.sequence_order),
                    input: Set(stage.input.clone()),
                    output: Set(stage.output.clone()),
                    description: Set(stage.description.clone()),
                    objective_metrics: Set(Some(stage.objective_metrics.clone())),
                    entry_criteria: Set(stage.entry_criteria.clone()),
                    exit_criteria: Set(stage.exit_criteria.clone()),
                    owner_id: Set(stage.owner_id),
                    key_metrics: Set(Some(stage.key_metrics.clone())),
                    value_stream_id: Set(stage.value_stream_id),
                    created_at: Set(now),
                    updated_at: Set(now),
                    deleted_at: NotSet,
                };
                let model = am
                    .insert(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("valueStreamId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("sequenceOrder", TypeRef::named_nn(TypeRef::INT)))
    .argument(InputValue::new("input", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("output", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("objectiveMetrics", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("entryCriteria", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("exitCriteria", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("ownerId", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("keyMetrics", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(create);

    // ── valueStreamStageUpdate ───────────────────────────────────────
    let update = Field::new(
        "valueStreamStageUpdate",
        TypeRef::named_nn("ValueStreamStages"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Update)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;

                let existing = value_stream_stage::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Value stream stage not found."))?;
                let vs_id = existing.value_stream_id;
                let space_id = space_of_value_stream(db, vs_id).await?;
                ensure_space_edit_access(&ctx, db, space_id).await?;
                // Sub-entity writes follow the parent value stream's owner.
                let vs_owner = owner_of_value_stream(db, vs_id).await?;
                ensure_entity_owner_or_admin(&ctx, vs_owner).await?;

                let mut am: value_stream_stage::ActiveModel = existing.clone().into();
                if let Some(v) = ctx.args.get("name").and_then(|v| v.string().ok()) {
                    am.name = Set(v.to_owned());
                }
                let mut new_sequence_order = None;
                if let Some(v) = ctx.args.get("sequenceOrder").and_then(|v| v.i64().ok()).map(|v| v as i32) {
                    am.sequence_order = Set(v);
                    new_sequence_order = Some(v);
                }
                match ctx.args.get("input") {
                    Some(v) if v.is_null() => am.input = Set(None),
                    Some(v) => {
                        if let Ok(s) = v.string() {
                            am.input = Set(Some(s.to_owned()));
                        }
                    }
                    None => {}
                }
                match ctx.args.get("output") {
                    Some(v) if v.is_null() => am.output = Set(None),
                    Some(v) => {
                        if let Ok(s) = v.string() {
                            am.output = Set(Some(s.to_owned()));
                        }
                    }
                    None => {}
                }
                match ctx.args.get("description") {
                    Some(v) if v.is_null() => am.description = Set(None),
                    Some(v) => {
                        if let Ok(s) = v.string() {
                            am.description = Set(Some(s.to_owned()));
                        }
                    }
                    None => {}
                }
                if let Some(metrics) = parse_string_string_map_arg(&ctx, "objectiveMetrics")? {
                    am.objective_metrics = Set(Some(metrics));
                }
                match ctx.args.get("entryCriteria") {
                    Some(v) if v.is_null() => am.entry_criteria = Set(None),
                    Some(v) => {
                        if let Ok(s) = v.string() {
                            am.entry_criteria = Set(Some(s.to_owned()));
                        }
                    }
                    None => {}
                }
                match ctx.args.get("exitCriteria") {
                    Some(v) if v.is_null() => am.exit_criteria = Set(None),
                    Some(v) => {
                        if let Ok(s) = v.string() {
                            am.exit_criteria = Set(Some(s.to_owned()));
                        }
                    }
                    None => {}
                }
                if let Some(v) = ctx.args.get("ownerId").and_then(|v| v.string().ok()) {
                    if v.is_empty() {
                        am.owner_id = Set(None);
                    } else {
                        am.owner_id = Set(Uuid::parse_str(v).ok());
                    }
                }
                if let Some(metrics) = parse_string_string_map_arg(&ctx, "keyMetrics")? {
                    am.key_metrics = Set(Some(metrics));
                }

                // Domain rule: sequence_order must stay unique within the value stream.
                if let Some(order) = new_sequence_order {
                    let repo = SeaOrmValueStreamRepo::new(db.clone());
                    let siblings = repo
                        .find_stages_by_value_stream(vs_id)
                        .await
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    let mut candidate: business_architecture::domain::value_stream::entity::ValueStreamStage =
                        existing.into();
                    candidate.sequence_order = order;
                    candidate
                        .ensure_sequence_order_unique(&siblings)
                        .map_err(domain_err_to_graphql)?;
                }

                am.updated_at = Set(chrono::Utc::now());
                let model = am
                    .update(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("sequenceOrder", TypeRef::named(TypeRef::INT)))
    .argument(InputValue::new("input", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("output", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("objectiveMetrics", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("entryCriteria", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("exitCriteria", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("ownerId", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("keyMetrics", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(update);

    // ── valueStreamStageDelete ───────────────────────────────────────
    let delete = Field::new(
        "valueStreamStageDelete",
        TypeRef::named_nn(TypeRef::BOOLEAN),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Delete)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;

                let existing = value_stream_stage::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Value stream stage not found."))?;
                let space_id = space_of_value_stream(db, existing.value_stream_id).await?;
                ensure_space_edit_access(&ctx, db, space_id).await?;
                // Sub-entity writes follow the parent value stream's owner.
                let vs_owner = owner_of_value_stream(db, existing.value_stream_id).await?;
                ensure_entity_owner_or_admin(&ctx, vs_owner).await?;

                value_stream_stage::Entity::delete_by_id(id)
                    .exec(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(async_graphql::Value::Boolean(true)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);

    // ── capabilityProcessCreate ─────────────────────────────────────
    // Join table: both parents must exist and live in the same space.
    let create = Field::new(
        "capabilityProcessCreate",
        TypeRef::named_nn("CapabilityProcesses"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Create)?;
                let db = ctx.data::<DatabaseConnection>()?;

                let capability_id = parse_uuid_arg(&ctx, "capabilityId")?;
                let process_id = parse_uuid_arg(&ctx, "processId")?;
                let cap_space = space_of_capability(db, capability_id).await?;
                let proc_space = space_of_process(db, process_id).await?;
                if cap_space != proc_space {
                    return Err(async_graphql::Error::new(
                        "Capability and process must belong to the same space.",
                    ));
                }
                ensure_space_edit_access(&ctx, db, cap_space).await?;
                // Linking mutates both parents' relationship graphs: the actor
                // must own the capability and the process (or be an admin).
                ensure_entity_owner_or_admin(&ctx, owner_of_capability(db, capability_id).await?).await?;
                ensure_entity_owner_or_admin(&ctx, owner_of_process(db, process_id).await?).await?;

                let am = capability_process::ActiveModel {
                    capability_id: Set(capability_id),
                    process_id: Set(process_id),
                };
                let model = am
                    .insert(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("capabilityId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("processId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(create);

    // ── capabilityProcessDelete ─────────────────────────────────────
    let delete = Field::new(
        "capabilityProcessDelete",
        TypeRef::named_nn(TypeRef::BOOLEAN),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Delete)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let capability_id = parse_uuid_arg(&ctx, "capabilityId")?;
                let process_id = parse_uuid_arg(&ctx, "processId")?;

                let existing = capability_process::Entity::find_by_id((capability_id, process_id))
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Capability-process link not found."))?;
                let space_id = space_of_capability(db, existing.capability_id).await?;
                ensure_space_edit_access(&ctx, db, space_id).await?;
                ensure_entity_owner_or_admin(&ctx, owner_of_capability(db, existing.capability_id).await?).await?;
                ensure_entity_owner_or_admin(&ctx, owner_of_process(db, existing.process_id).await?).await?;

                capability_process::Entity::delete_by_id((capability_id, process_id))
                    .exec(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(async_graphql::Value::Boolean(true)))
            })
        },
    )
    .argument(InputValue::new("capabilityId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("processId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);

    // ── stageCapabilityCreate ───────────────────────────────────────
    // Join table: stage (→ value stream) and capability must share a space.
    let create = Field::new(
        "stageCapabilityCreate",
        TypeRef::named_nn("StageCapabilities"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Create)?;
                let db = ctx.data::<DatabaseConnection>()?;

                let stage_id = parse_uuid_arg(&ctx, "stageId")?;
                let capability_id = parse_uuid_arg(&ctx, "capabilityId")?;
                let stage_space = space_of_stage(db, stage_id).await?;
                let cap_space = space_of_capability(db, capability_id).await?;
                if stage_space != cap_space {
                    return Err(async_graphql::Error::new(
                        "Stage and capability must belong to the same space.",
                    ));
                }
                ensure_space_edit_access(&ctx, db, stage_space).await?;
                // The stage follows its parent value stream's owner; linking
                // also mutates the capability's relationship graph.
                ensure_entity_owner_or_admin(&ctx, owner_of_stage_parent(db, stage_id).await?).await?;
                ensure_entity_owner_or_admin(&ctx, owner_of_capability(db, capability_id).await?).await?;

                let am = stage_capability::ActiveModel {
                    stage_id: Set(stage_id),
                    capability_id: Set(capability_id),
                };
                // Idempotent: re-submitting an existing link is a no-op
                // instead of a duplicate-key error (matches the repo-level
                // `link_stage_capability` semantics).
                let model = match stage_capability::Entity::find_by_id((stage_id, capability_id))
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                {
                    Some(m) => m,
                    None => am
                        .insert(db)
                        .await
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?,
                };
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("stageId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("capabilityId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(create);

    // ── stageCapabilityDelete ───────────────────────────────────────
    let delete = Field::new(
        "stageCapabilityDelete",
        TypeRef::named_nn(TypeRef::BOOLEAN),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Delete)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let stage_id = parse_uuid_arg(&ctx, "stageId")?;
                let capability_id = parse_uuid_arg(&ctx, "capabilityId")?;

                let existing = stage_capability::Entity::find_by_id((stage_id, capability_id))
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Stage-capability link not found."))?;
                let space_id = space_of_stage(db, existing.stage_id).await?;
                ensure_space_edit_access(&ctx, db, space_id).await?;
                ensure_entity_owner_or_admin(&ctx, owner_of_stage_parent(db, existing.stage_id).await?).await?;
                ensure_entity_owner_or_admin(&ctx, owner_of_capability(db, existing.capability_id).await?).await?;

                stage_capability::Entity::delete_by_id((stage_id, capability_id))
                    .exec(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(async_graphql::Value::Boolean(true)))
            })
        },
    )
    .argument(InputValue::new("stageId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("capabilityId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);
}

// ============================================================================
// Custom Space-Scoped Queries (membership-enforced)
// ============================================================================

/// Require the actor to be a member (any role) of the given space, or an admin.
async fn ensure_space_read_access(
    ctx: &async_graphql::dynamic::ResolverContext<'_>,
    db: &DatabaseConnection,
    space_id: Uuid,
) -> async_graphql::Result<()> {
    let claims = require_claims(ctx)?;
    if claims.user_role().is_admin() {
        return Ok(());
    }
    let service = space_service(db);
    let membership = service
        .my_membership(space_id, claims.user_id)
        .await
        .map_err(domain_err_to_graphql)?;
    if membership.is_none() {
        return Err(graphql_err_with_code(&DomainError::NotSpaceMember, "FORBIDDEN_SPACE_NOT_MEMBER"));
    }
    Ok(())
}

/// Lightweight result type for `spaceUserByEmail`. Exposing only id/name
/// avoids routing sub-field resolution through the `Users` entity object (whose
/// `email` field is admin-only and `password_hash` is hidden), so a non-admin
/// space owner can look up users to invite without needing global admin. The
/// `email` is intentionally NOT returned: the caller already supplied it, and
/// echoing it back would re-expose admin-only PII and enable cross-tenant email
/// enumeration through this endpoint.
#[derive(Clone, Debug)]
struct SpaceUserLookup {
    id: String,
    name: String,
}

/// Result type for `myMembership`: returns only the caller's own role in a
/// space (or null for non-members). This avoids the admin-gated auto-generated
/// `spaceMembers` query so non-admin editors/owners can resolve their edit
/// permissions without global admin.
#[derive(Clone, Debug)]
struct MyMembership {
    role: String,
}

/// Result type for `spaceMembersBySpace`. Returns `{ userId, name, role }`
/// instead of the raw `SpaceMembers` entity so the member-management UI can
/// show a human-readable name rather than a truncated UUID. The `name` is
/// resolved by joining with the `users` table; `email` is intentionally
/// omitted (admin-only PII) to avoid cross-tenant enumeration through this
/// membership-scoped query.
#[derive(Clone, Debug)]
struct SpaceMemberWithUser {
    user_id: String,
    name: String,
    role: String,
}

// ============================================================================
// Custom Application Architecture Domain Mutations (P2+P3, space-level ACL)
// ============================================================================

fn register_application_component_domain_mutations(builder: &mut Builder) {
    use async_graphql::dynamic::{Field, FieldFuture, FieldValue, InputValue, TypeRef};
    use sea_orm::ActiveValue::{NotSet, Set};
    use sea_orm::{EntityTrait, ActiveModelTrait};

    // ── applicationComponentCreate ───────────────────────────────────
    let create = Field::new(
        "applicationComponentCreate",
        TypeRef::named_nn("ApplicationComponents"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Create)?;
                let db = ctx.data::<DatabaseConnection>()?;

                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let name = ctx.args.try_get("name")?.string()?.to_owned();
                let component_type = parse_enum::<ApplicationComponentType>(
                    ctx.args.try_get("type")?.enum_name()?,
                )?;
                let repo = ctx.args.try_get("repo")?.string()?.to_owned();
                let path = ctx.args.try_get("path")?.string()?.to_owned();
                let technology = ctx
                    .args
                    .get("technology")
                    .and_then(|v| v.string().ok())
                    .map(|s| s.to_owned());
                let status = parse_enum::<ApplicationComponentStatus>(
                    ctx.args.try_get("status")?.enum_name()?,
                )?;
                let version = ctx.args.try_get("version")?.string()?.to_owned();
                let owner_id = ctx
                    .args
                    .get("ownerId")
                    .and_then(|v| v.string().ok())
                    .and_then(|s| Uuid::parse_str(s).ok());

                ensure_space_edit_access(&ctx, db, space_id).await?;

                let now = chrono::Utc::now();
                let am = application_component::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    name: Set(name),
                    r#type: Set(component_type),
                    repo: Set(repo),
                    path: Set(path),
                    technology: Set(technology),
                    status: Set(status),
                    version: Set(version),
                    owner_id: Set(owner_id),
                    created_at: Set(now),
                    updated_at: Set(now),
                    deleted_at: NotSet,
                    space_id: Set(space_id),
                };
                let model = am
                    .insert(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("type", TypeRef::named_nn("ApplicationComponentTypeEnum")))
    .argument(InputValue::new("repo", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("path", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("technology", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("status", TypeRef::named_nn("ApplicationComponentStatusEnum")))
    .argument(InputValue::new("version", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("ownerId", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(create);

    // ── applicationComponentUpdate ───────────────────────────────────
    let update = Field::new(
        "applicationComponentUpdate",
        TypeRef::named_nn("ApplicationComponents"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Update)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;

                let existing = application_component::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Application component not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;

                let mut am: application_component::ActiveModel = existing.into();
                if let Some(v) = ctx.args.get("name").and_then(|v| v.string().ok()) {
                    am.name = Set(v.to_owned());
                }
                if let Some(v) = get_enum_arg(&ctx, "type") {
                    am.r#type = Set(parse_enum::<ApplicationComponentType>(&v)?);
                }
                if let Some(v) = ctx.args.get("repo").and_then(|v| v.string().ok()) {
                    am.repo = Set(v.to_owned());
                }
                if let Some(v) = ctx.args.get("path").and_then(|v| v.string().ok()) {
                    am.path = Set(v.to_owned());
                }
                match ctx.args.get("technology") {
                    Some(v) if v.is_null() => am.technology = Set(None),
                    Some(v) => {
                        if let Ok(s) = v.string() {
                            am.technology = Set(Some(s.to_owned()));
                        }
                    }
                    None => {}
                }
                if let Some(v) = get_enum_arg(&ctx, "status") {
                    am.status = Set(parse_enum::<ApplicationComponentStatus>(&v)?);
                }
                if let Some(v) = ctx.args.get("version").and_then(|v| v.string().ok()) {
                    am.version = Set(v.to_owned());
                }
                match ctx.args.get("ownerId") {
                    Some(v) if v.is_null() => am.owner_id = Set(None),
                    Some(v) => {
                        if let Ok(s) = v.string() {
                            am.owner_id = Set(Uuid::parse_str(s).ok());
                        }
                    }
                    None => {}
                }
                am.updated_at = Set(chrono::Utc::now());
                let model = am
                    .update(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("type", TypeRef::named("ApplicationComponentTypeEnum")))
    .argument(InputValue::new("repo", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("path", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("technology", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("status", TypeRef::named("ApplicationComponentStatusEnum")))
    .argument(InputValue::new("version", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("ownerId", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(update);

    // ── applicationComponentDelete ───────────────────────────────────
    let delete = Field::new(
        "applicationComponentDelete",
        TypeRef::named_nn(TypeRef::BOOLEAN),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Delete)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;

                let existing = application_component::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Application component not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;

                application_component::Entity::delete_by_id(id)
                    .exec(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(async_graphql::Value::Boolean(true)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);
}

fn register_application_process_domain_mutations(builder: &mut Builder) {
    use async_graphql::dynamic::{Field, FieldFuture, FieldValue, InputValue, TypeRef};
    use sea_orm::ActiveValue::{NotSet, Set};
    use sea_orm::{EntityTrait, ActiveModelTrait};

    // ── applicationProcessCreate ─────────────────────────────────────
    let create = Field::new(
        "applicationProcessCreate",
        TypeRef::named_nn("ApplicationProcesses"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Create)?;
                let db = ctx.data::<DatabaseConnection>()?;

                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let name = ctx.args.try_get("name")?.string()?.to_owned();
                let description = ctx
                    .args
                    .get("description")
                    .and_then(|v| v.string().ok())
                    .map(|s| s.to_owned())
                    .unwrap_or_default();
                let trigger = parse_enum::<ApplicationProcessTrigger>(
                    ctx.args.try_get("trigger")?.enum_name()?,
                )?;
                let inputs = parse_string_vec_arg(&ctx, "inputs")?;
                let outputs = parse_string_vec_arg(&ctx, "outputs")?;
                let timeout: Option<i32> = ctx
                    .args
                    .get("timeout")
                    .and_then(|v| v.i64().ok())
                    .map(|v| v as i32);
                let retry: Option<i32> = ctx
                    .args
                    .get("retry")
                    .and_then(|v| v.i64().ok())
                    .map(|v| v as i32);

                ensure_space_edit_access(&ctx, db, space_id).await?;

                let now = chrono::Utc::now();
                let am = application_process::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    name: Set(name),
                    description: Set(description),
                    trigger: Set(trigger),
                    inputs: Set(inputs),
                    outputs: Set(outputs),
                    timeout: Set(timeout),
                    retry: Set(retry),
                    status: Set(LifecycleStatus::Active),
                    logical_id: Set(Uuid::now_v7()),
                    business_version: Set("v1.0".to_owned()),
                    created_at: Set(now),
                    updated_at: Set(now),
                    deleted_at: NotSet,
                    space_id: Set(space_id),
                };
                let model = am
                    .insert(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("trigger", TypeRef::named_nn("ApplicationProcessTriggerEnum")))
    .argument(InputValue::new("inputs", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("outputs", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("timeout", TypeRef::named(TypeRef::INT)))
    .argument(InputValue::new("retry", TypeRef::named(TypeRef::INT)));
    builder.mutations.push(create);

    // ── applicationProcessUpdate ─────────────────────────────────────
    let update = Field::new(
        "applicationProcessUpdate",
        TypeRef::named_nn("ApplicationProcesses"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Update)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;

                let existing = application_process::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Application process not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;

                let mut am: application_process::ActiveModel = existing.into();
                if let Some(v) = ctx.args.get("name").and_then(|v| v.string().ok()) {
                    am.name = Set(v.to_owned());
                }
                if let Some(v) = ctx.args.get("description").and_then(|v| v.string().ok()) {
                    am.description = Set(v.to_owned());
                }
                if let Some(v) = get_enum_arg(&ctx, "trigger") {
                    am.trigger = Set(parse_enum::<ApplicationProcessTrigger>(&v)?);
                }
                if ctx.args.get("inputs").is_some() {
                    am.inputs = Set(parse_string_vec_arg(&ctx, "inputs")?);
                }
                if ctx.args.get("outputs").is_some() {
                    am.outputs = Set(parse_string_vec_arg(&ctx, "outputs")?);
                }
                match ctx.args.get("timeout") {
                    Some(v) if v.is_null() => am.timeout = Set(None),
                    Some(v) => {
                        if let Ok(n) = v.i64() {
                            am.timeout = Set(Some(n as i32));
                        }
                    }
                    None => {}
                }
                match ctx.args.get("retry") {
                    Some(v) if v.is_null() => am.retry = Set(None),
                    Some(v) => {
                        if let Ok(n) = v.i64() {
                            am.retry = Set(Some(n as i32));
                        }
                    }
                    None => {}
                }
                if let Some(v) = get_enum_arg(&ctx, "status") {
                    am.status = Set(parse_enum::<LifecycleStatus>(&v)?);
                }
                am.updated_at = Set(chrono::Utc::now());
                let model = am
                    .update(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("trigger", TypeRef::named("ApplicationProcessTriggerEnum")))
    .argument(InputValue::new("inputs", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("outputs", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("timeout", TypeRef::named(TypeRef::INT)))
    .argument(InputValue::new("retry", TypeRef::named(TypeRef::INT)))
    .argument(InputValue::new("status", TypeRef::named("LifecycleStatusEnum")));
    builder.mutations.push(update);

    // ── applicationProcessDelete ─────────────────────────────────────
    let delete = Field::new(
        "applicationProcessDelete",
        TypeRef::named_nn(TypeRef::BOOLEAN),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Delete)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;

                let existing = application_process::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Application process not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;

                application_process::Entity::delete_by_id(id)
                    .exec(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(async_graphql::Value::Boolean(true)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);
}

fn register_application_process_step_domain_mutations(builder: &mut Builder) {
    use async_graphql::dynamic::{Field, FieldFuture, FieldValue, InputValue, TypeRef};
    use sea_orm::ActiveValue::{NotSet, Set};
    use sea_orm::{EntityTrait, ActiveModelTrait};

    // ── applicationProcessStepCreate ─────────────────────────────────
    let create = Field::new(
        "applicationProcessStepCreate",
        TypeRef::named_nn("ApplicationProcessSteps"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Create)?;
                let db = ctx.data::<DatabaseConnection>()?;

                let process_id = parse_uuid_arg(&ctx, "processId")?;
                let space_id = space_of_application_process(db, process_id).await?;
                ensure_space_edit_access(&ctx, db, space_id).await?;

                let name = ctx.args.try_get("name")?.string()?.to_owned();
                let action = ctx.args.try_get("action")?.string()?.to_owned();
                let description = ctx
                    .args
                    .get("description")
                    .and_then(|v| v.string().ok())
                    .map(|s| s.to_owned())
                    .unwrap_or_default();
                let sequence_order: i32 = ctx.args.try_get("sequenceOrder")?.i64()? as i32;
                let inputs = parse_string_vec_arg(&ctx, "inputs")?;
                let outputs = parse_string_vec_arg(&ctx, "outputs")?;
                let dependencies = parse_string_vec_arg(&ctx, "dependencies")?;

                let now = chrono::Utc::now();
                let am = application_process_step::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    name: Set(name),
                    action: Set(action),
                    description: Set(description),
                    sequence_order: Set(sequence_order),
                    inputs: Set(inputs),
                    outputs: Set(outputs),
                    dependencies: Set(dependencies),
                    process_id: Set(process_id),
                    created_at: Set(now),
                    updated_at: Set(now),
                    deleted_at: NotSet,
                };
                let model = am
                    .insert(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("processId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("action", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("sequenceOrder", TypeRef::named_nn(TypeRef::INT)))
    .argument(InputValue::new("inputs", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("outputs", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("dependencies", TypeRef::named_list(TypeRef::STRING)));
    builder.mutations.push(create);

    // ── applicationProcessStepUpdate ─────────────────────────────────
    let update = Field::new(
        "applicationProcessStepUpdate",
        TypeRef::named_nn("ApplicationProcessSteps"),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Update)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;

                let existing = application_process_step::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Application process step not found."))?;
                let space_id = space_of_application_process(db, existing.process_id).await?;
                ensure_space_edit_access(&ctx, db, space_id).await?;

                let mut am: application_process_step::ActiveModel = existing.into();
                if let Some(v) = ctx.args.get("name").and_then(|v| v.string().ok()) {
                    am.name = Set(v.to_owned());
                }
                if let Some(v) = ctx.args.get("action").and_then(|v| v.string().ok()) {
                    am.action = Set(v.to_owned());
                }
                if let Some(v) = ctx.args.get("description").and_then(|v| v.string().ok()) {
                    am.description = Set(v.to_owned());
                }
                if let Some(v) = ctx.args.get("sequenceOrder").and_then(|v| v.i64().ok()).map(|v| v as i32) {
                    am.sequence_order = Set(v);
                }
                if ctx.args.get("inputs").is_some() {
                    am.inputs = Set(parse_string_vec_arg(&ctx, "inputs")?);
                }
                if ctx.args.get("outputs").is_some() {
                    am.outputs = Set(parse_string_vec_arg(&ctx, "outputs")?);
                }
                if ctx.args.get("dependencies").is_some() {
                    am.dependencies = Set(parse_string_vec_arg(&ctx, "dependencies")?);
                }
                am.updated_at = Set(chrono::Utc::now());
                let model = am
                    .update(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("action", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("sequenceOrder", TypeRef::named(TypeRef::INT)))
    .argument(InputValue::new("inputs", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("outputs", TypeRef::named_list(TypeRef::STRING)))
    .argument(InputValue::new("dependencies", TypeRef::named_list(TypeRef::STRING)));
    builder.mutations.push(update);

    // ── applicationProcessStepDelete ─────────────────────────────────
    let delete = Field::new(
        "applicationProcessStepDelete",
        TypeRef::named_nn(TypeRef::BOOLEAN),
        |ctx| {
            FieldFuture::new(async move {
                check_value_stream_auth(&ctx, OperationType::Delete)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let id = parse_uuid_arg(&ctx, "id")?;

                let existing = application_process_step::Entity::find_by_id(id)
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Application process step not found."))?;
                let space_id = space_of_application_process(db, existing.process_id).await?;
                ensure_space_edit_access(&ctx, db, space_id).await?;

                application_process_step::Entity::delete_by_id(id)
                    .exec(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(async_graphql::Value::Boolean(true)))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);
}

fn register_v21_entity_domain_mutations(builder: &mut Builder) {
    use async_graphql::dynamic::{Field, FieldFuture, FieldValue, InputValue, TypeRef};
    use sea_orm::ActiveValue::{NotSet, Set};
    use sea_orm::{EntityTrait, ActiveModelTrait};

    // organizationalUnit CRUD
    let create = Field::new("organizationalUnitCreate", TypeRef::named_nn("OrganizationalUnits"), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Create)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let space_id = parse_uuid_arg(&ctx, "spaceId")?;
            let name = ctx.args.try_get("name")?.string()?.to_owned();
            let unit_type = parse_enum::<OrganizationalUnitType>(ctx.args.try_get("type")?.enum_name()?)?;
            let parent_id = ctx.args.get("parentId").and_then(|v| v.string().ok()).and_then(|s| Uuid::parse_str(s).ok());
            let description = ctx.args.get("description").and_then(|v| v.string().ok()).map(|s| s.to_owned());
            let status = ctx.args.get("status").and_then(|v| v.string().ok()).map(|s| s.to_owned()).unwrap_or_else(|| "active".to_string());
            ensure_space_edit_access(&ctx, db, space_id).await?;
            let now = chrono::Utc::now();
            let am = organizational_unit::ActiveModel {
                id: Set(Uuid::now_v7()), name: Set(name), r#type: Set(unit_type), parent_id: Set(parent_id),
                description: Set(description), status: Set(status), created_at: Set(now), updated_at: Set(now),
                deleted_at: NotSet, space_id: Set(space_id),
            };
            let model = am.insert(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(FieldValue::owned_any(model)))
        })
    })
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("type", TypeRef::named_nn("OrganizationalUnitTypeEnum")))
    .argument(InputValue::new("parentId", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("status", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(create);

    let update = Field::new("organizationalUnitUpdate", TypeRef::named_nn("OrganizationalUnits"), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Update)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let id = parse_uuid_arg(&ctx, "id")?;
            let existing = organizational_unit::Entity::find_by_id(id).one(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?.ok_or_else(|| async_graphql::Error::new("Organizational unit not found."))?;
            ensure_space_edit_access(&ctx, db, existing.space_id).await?;
            let mut am: organizational_unit::ActiveModel = existing.into();
            if let Some(v) = ctx.args.get("name").and_then(|v| v.string().ok()) { am.name = Set(v.to_owned()); }
            if let Some(v) = get_enum_arg(&ctx, "type") { am.r#type = Set(parse_enum::<OrganizationalUnitType>(&v)?); }
            match ctx.args.get("parentId") {
                Some(v) if v.is_null() => am.parent_id = Set(None),
                Some(v) => { if let Ok(s) = v.string() { am.parent_id = Set(Uuid::parse_str(s).ok()); } }
                None => {}
            }
            if let Some(v) = ctx.args.get("description").and_then(|v| v.string().ok()) { am.description = Set(Some(v.to_owned())); }
            if let Some(v) = ctx.args.get("status").and_then(|v| v.string().ok()) { am.status = Set(v.to_owned()); }
            am.updated_at = Set(chrono::Utc::now());
            let model = am.update(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(FieldValue::owned_any(model)))
        })
    })
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("type", TypeRef::named("OrganizationalUnitTypeEnum")))
    .argument(InputValue::new("parentId", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("status", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(update);

    let delete = Field::new("organizationalUnitDelete", TypeRef::named_nn(TypeRef::BOOLEAN), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Delete)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let id = parse_uuid_arg(&ctx, "id")?;
            let existing = organizational_unit::Entity::find_by_id(id).one(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?.ok_or_else(|| async_graphql::Error::new("Organizational unit not found."))?;
            ensure_space_edit_access(&ctx, db, existing.space_id).await?;
            organizational_unit::Entity::delete_by_id(id).exec(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(async_graphql::Value::Boolean(true)))
        })
    })
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);

    // businessRole CRUD
    let create = Field::new("businessRoleCreate", TypeRef::named_nn("BusinessRoles"), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Create)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let space_id = parse_uuid_arg(&ctx, "spaceId")?;
            let name = ctx.args.try_get("name")?.string()?.to_owned();
            let responsibilities = ctx.args.get("responsibilities").and_then(|v| v.string().ok()).map(|s| s.to_owned());
            let organization_id = parse_uuid_arg(&ctx, "organizationId")?;
            let org_space = space_of_organizational_unit(db, organization_id).await?;
            if org_space != space_id { return Err(async_graphql::Error::new("Organization and space must match.")); }
            ensure_space_edit_access(&ctx, db, space_id).await?;
            let now = chrono::Utc::now();
            let am = business_role::ActiveModel {
                id: Set(Uuid::now_v7()), name: Set(name), responsibilities: Set(responsibilities),
                organization_id: Set(organization_id), created_at: Set(now), updated_at: Set(now),
                deleted_at: NotSet, space_id: Set(space_id),
            };
            let model = am.insert(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(FieldValue::owned_any(model)))
        })
    })
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("responsibilities", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("organizationId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(create);

    let update = Field::new("businessRoleUpdate", TypeRef::named_nn("BusinessRoles"), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Update)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let id = parse_uuid_arg(&ctx, "id")?;
            let existing = business_role::Entity::find_by_id(id).one(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?.ok_or_else(|| async_graphql::Error::new("Business role not found."))?;
            ensure_space_edit_access(&ctx, db, existing.space_id).await?;
            let mut am: business_role::ActiveModel = existing.into();
            if let Some(v) = ctx.args.get("name").and_then(|v| v.string().ok()) { am.name = Set(v.to_owned()); }
            if let Some(v) = ctx.args.get("responsibilities").and_then(|v| v.string().ok()) { am.responsibilities = Set(Some(v.to_owned())); }
            am.updated_at = Set(chrono::Utc::now());
            let model = am.update(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(FieldValue::owned_any(model)))
        })
    })
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("responsibilities", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(update);

    let delete = Field::new("businessRoleDelete", TypeRef::named_nn(TypeRef::BOOLEAN), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Delete)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let id = parse_uuid_arg(&ctx, "id")?;
            let existing = business_role::Entity::find_by_id(id).one(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?.ok_or_else(|| async_graphql::Error::new("Business role not found."))?;
            ensure_space_edit_access(&ctx, db, existing.space_id).await?;
            business_role::Entity::delete_by_id(id).exec(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(async_graphql::Value::Boolean(true)))
        })
    })
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);

    // functionalModule CRUD
    let create = Field::new("functionalModuleCreate", TypeRef::named_nn("FunctionalModules"), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Create)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let space_id = parse_uuid_arg(&ctx, "spaceId")?;
            let name = ctx.args.try_get("name")?.string()?.to_owned();
            let description = ctx.args.get("description").and_then(|v| v.string().ok()).map(|s| s.to_owned());
            let boundary = ctx.args.get("boundary").and_then(|v| v.string().ok()).map(|s| s.to_owned());
            let status = parse_enum::<FunctionalModuleStatus>(ctx.args.try_get("status")?.enum_name()?)?;
            let parent_id = ctx.args.get("parentId").and_then(|v| v.string().ok()).and_then(|s| Uuid::parse_str(s).ok());
            ensure_space_edit_access(&ctx, db, space_id).await?;
            let now = chrono::Utc::now();
            let am = functional_module::ActiveModel {
                id: Set(Uuid::now_v7()), name: Set(name), description: Set(description), boundary: Set(boundary),
                status: Set(status), parent_id: Set(parent_id), created_at: Set(now), updated_at: Set(now),
                deleted_at: NotSet, space_id: Set(space_id),
            };
            let model = am.insert(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(FieldValue::owned_any(model)))
        })
    })
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("boundary", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("status", TypeRef::named_nn("FunctionalModuleStatusEnum")))
    .argument(InputValue::new("parentId", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(create);

    let update = Field::new("functionalModuleUpdate", TypeRef::named_nn("FunctionalModules"), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Update)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let id = parse_uuid_arg(&ctx, "id")?;
            let existing = functional_module::Entity::find_by_id(id).one(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?.ok_or_else(|| async_graphql::Error::new("Functional module not found."))?;
            ensure_space_edit_access(&ctx, db, existing.space_id).await?;
            let mut am: functional_module::ActiveModel = existing.into();
            if let Some(v) = ctx.args.get("name").and_then(|v| v.string().ok()) { am.name = Set(v.to_owned()); }
            if let Some(v) = ctx.args.get("description").and_then(|v| v.string().ok()) { am.description = Set(Some(v.to_owned())); }
            if let Some(v) = ctx.args.get("boundary").and_then(|v| v.string().ok()) { am.boundary = Set(Some(v.to_owned())); }
            if let Some(v) = get_enum_arg(&ctx, "status") { am.status = Set(parse_enum::<FunctionalModuleStatus>(&v)?); }
            match ctx.args.get("parentId") {
                Some(v) if v.is_null() => am.parent_id = Set(None),
                Some(v) => { if let Ok(s) = v.string() { am.parent_id = Set(Uuid::parse_str(s).ok()); } }
                None => {}
            }
            am.updated_at = Set(chrono::Utc::now());
            let model = am.update(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(FieldValue::owned_any(model)))
        })
    })
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("boundary", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("status", TypeRef::named("FunctionalModuleStatusEnum")))
    .argument(InputValue::new("parentId", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(update);

    let delete = Field::new("functionalModuleDelete", TypeRef::named_nn(TypeRef::BOOLEAN), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Delete)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let id = parse_uuid_arg(&ctx, "id")?;
            let existing = functional_module::Entity::find_by_id(id).one(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?.ok_or_else(|| async_graphql::Error::new("Functional module not found."))?;
            ensure_space_edit_access(&ctx, db, existing.space_id).await?;
            functional_module::Entity::delete_by_id(id).exec(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(async_graphql::Value::Boolean(true)))
        })
    })
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);

    // applicationInterface CRUD
    let create = Field::new("applicationInterfaceCreate", TypeRef::named_nn("ApplicationInterfaces"), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Create)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let space_id = parse_uuid_arg(&ctx, "spaceId")?;
            let name = ctx.args.try_get("name")?.string()?.to_owned();
            let protocol = parse_enum::<ApplicationInterfaceProtocol>(ctx.args.try_get("protocol")?.enum_name()?)?;
            let contract = ctx.args.get("contract").and_then(|v| v.string().ok()).map(|s| s.to_owned());
            let provider_module_id = parse_uuid_arg(&ctx, "providerModuleId")?;
            let consumer_module_id = ctx.args.get("consumerModuleId").and_then(|v| v.string().ok()).and_then(|s| Uuid::parse_str(s).ok());
            let mod_space = space_of_functional_module(db, provider_module_id).await?;
            if mod_space != space_id { return Err(async_graphql::Error::new("Provider module and space must match.")); }
            ensure_space_edit_access(&ctx, db, space_id).await?;
            let now = chrono::Utc::now();
            let am = application_interface::ActiveModel {
                id: Set(Uuid::now_v7()), name: Set(name), protocol: Set(protocol), contract: Set(contract),
                input_schema: NotSet, output_schema: NotSet, provider_module_id: Set(provider_module_id),
                consumer_module_id: Set(consumer_module_id), created_at: Set(now), updated_at: Set(now),
                deleted_at: NotSet, space_id: Set(space_id),
            };
            let model = am.insert(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(FieldValue::owned_any(model)))
        })
    })
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("protocol", TypeRef::named_nn("ApplicationInterfaceProtocolEnum")))
    .argument(InputValue::new("contract", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("providerModuleId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("consumerModuleId", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(create);

    let update = Field::new("applicationInterfaceUpdate", TypeRef::named_nn("ApplicationInterfaces"), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Update)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let id = parse_uuid_arg(&ctx, "id")?;
            let existing = application_interface::Entity::find_by_id(id).one(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?.ok_or_else(|| async_graphql::Error::new("Application interface not found."))?;
            ensure_space_edit_access(&ctx, db, existing.space_id).await?;
            let mut am: application_interface::ActiveModel = existing.into();
            if let Some(v) = ctx.args.get("name").and_then(|v| v.string().ok()) { am.name = Set(v.to_owned()); }
            if let Some(v) = get_enum_arg(&ctx, "protocol") { am.protocol = Set(parse_enum::<ApplicationInterfaceProtocol>(&v)?); }
            if let Some(v) = ctx.args.get("contract").and_then(|v| v.string().ok()) { am.contract = Set(Some(v.to_owned())); }
            match ctx.args.get("consumerModuleId") {
                Some(v) if v.is_null() => am.consumer_module_id = Set(None),
                Some(v) => { if let Ok(s) = v.string() { am.consumer_module_id = Set(Uuid::parse_str(s).ok()); } }
                None => {}
            }
            am.updated_at = Set(chrono::Utc::now());
            let model = am.update(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(FieldValue::owned_any(model)))
        })
    })
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("protocol", TypeRef::named("ApplicationInterfaceProtocolEnum")))
    .argument(InputValue::new("contract", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("consumerModuleId", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(update);

    let delete = Field::new("applicationInterfaceDelete", TypeRef::named_nn(TypeRef::BOOLEAN), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Delete)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let id = parse_uuid_arg(&ctx, "id")?;
            let existing = application_interface::Entity::find_by_id(id).one(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?.ok_or_else(|| async_graphql::Error::new("Application interface not found."))?;
            ensure_space_edit_access(&ctx, db, existing.space_id).await?;
            application_interface::Entity::delete_by_id(id).exec(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(async_graphql::Value::Boolean(true)))
        })
    })
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);
}

fn register_realization_domain_mutations(builder: &mut Builder) {
    use async_graphql::dynamic::{Field, FieldFuture, FieldValue, InputValue, TypeRef};
    use sea_orm::ActiveValue::Set;
    use sea_orm::{EntityTrait, ActiveModelTrait};

    let create = Field::new("capabilityRealizationCreate", TypeRef::named_nn("CapabilityRealizations"), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Create)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let capability_id = parse_uuid_arg(&ctx, "capabilityId")?;
            let process_id = parse_uuid_arg(&ctx, "processId")?;
            let process_type = parse_enum::<CapabilityRealizationTargetType>(ctx.args.try_get("processType")?.enum_name()?)?;
            let cap_space = space_of_capability(db, capability_id).await?;
            let proc_space = match process_type {
                CapabilityRealizationTargetType::BusinessProcess => space_of_process(db, process_id).await?,
                CapabilityRealizationTargetType::ApplicationProcess => space_of_application_process(db, process_id).await?,
            };
            if cap_space != proc_space { return Err(async_graphql::Error::new("Capability and process must belong to the same space.")); }
            ensure_space_edit_access(&ctx, db, cap_space).await?;
            let am = capability_realization::ActiveModel { capability_id: Set(capability_id), process_id: Set(process_id), process_type: Set(process_type) };
            let model = am.insert(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(FieldValue::owned_any(model)))
        })
    })
    .argument(InputValue::new("capabilityId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("processId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("processType", TypeRef::named_nn("CapabilityRealizationTargetTypeEnum")));
    builder.mutations.push(create);

    let delete = Field::new("capabilityRealizationDelete", TypeRef::named_nn(TypeRef::BOOLEAN), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Delete)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let capability_id = parse_uuid_arg(&ctx, "capabilityId")?;
            let process_id = parse_uuid_arg(&ctx, "processId")?;
            let process_type = parse_enum::<CapabilityRealizationTargetType>(ctx.args.try_get("processType")?.enum_name()?)?;
            let existing = capability_realization::Entity::find_by_id((capability_id, process_id, process_type)).one(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?.ok_or_else(|| async_graphql::Error::new("Capability realization link not found."))?;
            let space_id = space_of_capability(db, existing.capability_id).await?;
            ensure_space_edit_access(&ctx, db, space_id).await?;
            capability_realization::Entity::delete_by_id((capability_id, process_id, process_type)).exec(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(async_graphql::Value::Boolean(true)))
        })
    })
    .argument(InputValue::new("capabilityId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("processId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("processType", TypeRef::named_nn("CapabilityRealizationTargetTypeEnum")));
    builder.mutations.push(delete);

    let create = Field::new("assignmentCreate", TypeRef::named_nn("Assignments"), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Create)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let organization_id = parse_uuid_arg(&ctx, "organizationId")?;
            let business_role_id = parse_uuid_arg(&ctx, "businessRoleId")?;
            let s1 = space_of_organizational_unit(db, organization_id).await?;
            let s2 = space_of_business_role(db, business_role_id).await?;
            if s1 != s2 { return Err(async_graphql::Error::new("Organization and business role must belong to the same space.")); }
            ensure_space_edit_access(&ctx, db, s1).await?;
            let am = assignment::ActiveModel { organization_id: Set(organization_id), business_role_id: Set(business_role_id) };
            let model = am.insert(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(FieldValue::owned_any(model)))
        })
    })
    .argument(InputValue::new("organizationId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("businessRoleId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(create);

    let delete = Field::new("assignmentDelete", TypeRef::named_nn(TypeRef::BOOLEAN), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Delete)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let organization_id = parse_uuid_arg(&ctx, "organizationId")?;
            let business_role_id = parse_uuid_arg(&ctx, "businessRoleId")?;
            let existing = assignment::Entity::find_by_id((organization_id, business_role_id)).one(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?.ok_or_else(|| async_graphql::Error::new("Assignment not found."))?;
            let space_id = space_of_organizational_unit(db, existing.organization_id).await?;
            ensure_space_edit_access(&ctx, db, space_id).await?;
            assignment::Entity::delete_by_id((organization_id, business_role_id)).exec(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(async_graphql::Value::Boolean(true)))
        })
    })
    .argument(InputValue::new("organizationId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("businessRoleId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);

    let create = Field::new("participationCreate", TypeRef::named_nn("Participations"), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Create)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let business_role_id = parse_uuid_arg(&ctx, "businessRoleId")?;
            let business_process_id = parse_uuid_arg(&ctx, "businessProcessId")?;
            let raci_role = parse_enum::<RaciRole>(ctx.args.try_get("raciRole")?.enum_name()?)?;
            let s1 = space_of_business_role(db, business_role_id).await?;
            let s2 = space_of_process(db, business_process_id).await?;
            if s1 != s2 { return Err(async_graphql::Error::new("Business role and business process must belong to the same space.")); }
            ensure_space_edit_access(&ctx, db, s1).await?;
            let am = participation::ActiveModel { business_role_id: Set(business_role_id), business_process_id: Set(business_process_id), raci_role: Set(raci_role) };
            let model = am.insert(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(FieldValue::owned_any(model)))
        })
    })
    .argument(InputValue::new("businessRoleId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("businessProcessId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("raciRole", TypeRef::named_nn("RaciRoleEnum")));
    builder.mutations.push(create);

    let delete = Field::new("participationDelete", TypeRef::named_nn(TypeRef::BOOLEAN), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Delete)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let business_role_id = parse_uuid_arg(&ctx, "businessRoleId")?;
            let business_process_id = parse_uuid_arg(&ctx, "businessProcessId")?;
            let existing = participation::Entity::find_by_id((business_role_id, business_process_id)).one(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?.ok_or_else(|| async_graphql::Error::new("Participation not found."))?;
            let space_id = space_of_business_role(db, existing.business_role_id).await?;
            ensure_space_edit_access(&ctx, db, space_id).await?;
            participation::Entity::delete_by_id((business_role_id, business_process_id)).exec(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(async_graphql::Value::Boolean(true)))
        })
    })
    .argument(InputValue::new("businessRoleId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("businessProcessId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);

    let create = Field::new("moduleContainmentCreate", TypeRef::named_nn("ModuleContainments"), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Create)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let functional_module_id = parse_uuid_arg(&ctx, "functionalModuleId")?;
            let application_component_id = parse_uuid_arg(&ctx, "applicationComponentId")?;
            let s1 = space_of_functional_module(db, functional_module_id).await?;
            let s2 = space_of_application_component(db, application_component_id).await?;
            if s1 != s2 { return Err(async_graphql::Error::new("Functional module and application component must belong to the same space.")); }
            ensure_space_edit_access(&ctx, db, s1).await?;
            let am = module_containment::ActiveModel { functional_module_id: Set(functional_module_id), application_component_id: Set(application_component_id) };
            let model = am.insert(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(FieldValue::owned_any(model)))
        })
    })
    .argument(InputValue::new("functionalModuleId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("applicationComponentId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(create);

    let delete = Field::new("moduleContainmentDelete", TypeRef::named_nn(TypeRef::BOOLEAN), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Delete)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let functional_module_id = parse_uuid_arg(&ctx, "functionalModuleId")?;
            let application_component_id = parse_uuid_arg(&ctx, "applicationComponentId")?;
            let existing = module_containment::Entity::find_by_id((functional_module_id, application_component_id)).one(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?.ok_or_else(|| async_graphql::Error::new("Module containment not found."))?;
            let space_id = space_of_functional_module(db, existing.functional_module_id).await?;
            ensure_space_edit_access(&ctx, db, space_id).await?;
            module_containment::Entity::delete_by_id((functional_module_id, application_component_id)).exec(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(async_graphql::Value::Boolean(true)))
        })
    })
    .argument(InputValue::new("functionalModuleId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("applicationComponentId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);

    let create = Field::new("interfaceExposureCreate", TypeRef::named_nn("InterfaceExposures"), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Create)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let functional_module_id = parse_uuid_arg(&ctx, "functionalModuleId")?;
            let application_interface_id = parse_uuid_arg(&ctx, "applicationInterfaceId")?;
            let s1 = space_of_functional_module(db, functional_module_id).await?;
            let s2 = space_of_application_interface(db, application_interface_id).await?;
            if s1 != s2 { return Err(async_graphql::Error::new("Functional module and application interface must belong to the same space.")); }
            ensure_space_edit_access(&ctx, db, s1).await?;
            let am = interface_exposure::ActiveModel { functional_module_id: Set(functional_module_id), application_interface_id: Set(application_interface_id) };
            let model = am.insert(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(FieldValue::owned_any(model)))
        })
    })
    .argument(InputValue::new("functionalModuleId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("applicationInterfaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(create);

    let delete = Field::new("interfaceExposureDelete", TypeRef::named_nn(TypeRef::BOOLEAN), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Delete)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let functional_module_id = parse_uuid_arg(&ctx, "functionalModuleId")?;
            let application_interface_id = parse_uuid_arg(&ctx, "applicationInterfaceId")?;
            let existing = interface_exposure::Entity::find_by_id((functional_module_id, application_interface_id)).one(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?.ok_or_else(|| async_graphql::Error::new("Interface exposure not found."))?;
            let space_id = space_of_functional_module(db, existing.functional_module_id).await?;
            ensure_space_edit_access(&ctx, db, space_id).await?;
            interface_exposure::Entity::delete_by_id((functional_module_id, application_interface_id)).exec(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(async_graphql::Value::Boolean(true)))
        })
    })
    .argument(InputValue::new("functionalModuleId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("applicationInterfaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);

    let create = Field::new("processReferenceCreate", TypeRef::named_nn("ProcessReferences"), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Create)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let application_process_id = parse_uuid_arg(&ctx, "applicationProcessId")?;
            let business_process_id = parse_uuid_arg(&ctx, "businessProcessId")?;
            let s1 = space_of_application_process(db, application_process_id).await?;
            let s2 = space_of_process(db, business_process_id).await?;
            if s1 != s2 { return Err(async_graphql::Error::new("Application process and business process must belong to the same space.")); }
            ensure_space_edit_access(&ctx, db, s1).await?;
            let am = process_reference::ActiveModel { application_process_id: Set(application_process_id), business_process_id: Set(business_process_id) };
            let model = am.insert(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(FieldValue::owned_any(model)))
        })
    })
    .argument(InputValue::new("applicationProcessId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("businessProcessId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(create);

    let delete = Field::new("processReferenceDelete", TypeRef::named_nn(TypeRef::BOOLEAN), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Delete)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let application_process_id = parse_uuid_arg(&ctx, "applicationProcessId")?;
            let business_process_id = parse_uuid_arg(&ctx, "businessProcessId")?;
            let existing = process_reference::Entity::find_by_id((application_process_id, business_process_id)).one(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?.ok_or_else(|| async_graphql::Error::new("Process reference not found."))?;
            let space_id = space_of_application_process(db, existing.application_process_id).await?;
            ensure_space_edit_access(&ctx, db, space_id).await?;
            process_reference::Entity::delete_by_id((application_process_id, business_process_id)).exec(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(async_graphql::Value::Boolean(true)))
        })
    })
    .argument(InputValue::new("applicationProcessId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("businessProcessId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);

    let create = Field::new("orchestrationCreate", TypeRef::named_nn("Orchestrations"), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Create)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let application_process_id = parse_uuid_arg(&ctx, "applicationProcessId")?;
            let functional_module_id = parse_uuid_arg(&ctx, "functionalModuleId")?;
            let s1 = space_of_application_process(db, application_process_id).await?;
            let s2 = space_of_functional_module(db, functional_module_id).await?;
            if s1 != s2 { return Err(async_graphql::Error::new("Application process and functional module must belong to the same space.")); }
            ensure_space_edit_access(&ctx, db, s1).await?;
            let am = orchestration::ActiveModel { application_process_id: Set(application_process_id), functional_module_id: Set(functional_module_id) };
            let model = am.insert(db).await.map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(FieldValue::owned_any(model)))
        })
    })
    .argument(InputValue::new("applicationProcessId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("functionalModuleId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(create);

    let delete = Field::new("orchestrationDelete", TypeRef::named_nn(TypeRef::BOOLEAN), |ctx| {
        FieldFuture::new(async move {
            check_value_stream_auth(&ctx, OperationType::Delete)?;
            let db = ctx.data::<DatabaseConnection>()?;
            let application_process_id = parse_uuid_arg(&ctx, "applicationProcessId")?;
            let functional_module_id = parse_uuid_arg(&ctx, "functionalModuleId")?;
            let existing = orchestration::Entity::find_by_id((application_process_id, functional_module_id)).one(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?.ok_or_else(|| async_graphql::Error::new("Orchestration not found."))?;
            let space_id = space_of_application_process(db, existing.application_process_id).await?;
            ensure_space_edit_access(&ctx, db, space_id).await?;
            orchestration::Entity::delete_by_id((application_process_id, functional_module_id)).exec(db).await
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
            Ok(Some(async_graphql::Value::Boolean(true)))
        })
    })
    .argument(InputValue::new("applicationProcessId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("functionalModuleId", TypeRef::named_nn(TypeRef::STRING)));
    builder.mutations.push(delete);

}
fn register_space_scoped_queries(builder: &mut Builder) {
    use async_graphql::dynamic::{Field, FieldFuture, FieldValue, InputValue, Object, TypeRef};
    use sea_orm::{EntityTrait, ColumnTrait, QueryFilter, PaginatorTrait};

    // ── SpaceUserLookup output type ───────────────────────────────────
    let space_user_type = Object::new("SpaceUserLookup")
        .field(Field::new("id", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<SpaceUserLookup>()?;
                Ok(Some(FieldValue::value(v.id.clone())))
            })
        }))
        .field(Field::new("name", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<SpaceUserLookup>()?;
                Ok(Some(FieldValue::value(v.name.clone())))
            })
        }));
    builder.outputs.push(space_user_type);

    // ── MyMembership output type ──────────────────────────────────────
    let my_membership_type = Object::new("MyMembership")
        .field(Field::new("role", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<MyMembership>()?;
                Ok(Some(FieldValue::value(v.role.clone())))
            })
        }));
    builder.outputs.push(my_membership_type);

    // ── SpaceMemberWithUser output type ───────────────────────────────
    let member_with_user_type = Object::new("SpaceMemberWithUser")
        .field(Field::new("userId", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<SpaceMemberWithUser>()?;
                Ok(Some(FieldValue::value(v.user_id.clone())))
            })
        }))
        .field(Field::new("name", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<SpaceMemberWithUser>()?;
                Ok(Some(FieldValue::value(v.name.clone())))
            })
        }))
        .field(Field::new("role", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<SpaceMemberWithUser>()?;
                Ok(Some(FieldValue::value(v.role.clone())))
            })
        }));
    builder.outputs.push(member_with_user_type);

    // ── CapabilityProcessRelation output type ─────────────────────────
    let cap_proc_relation_type = Object::new("CapabilityProcessRelation")
        .field(Field::new("capabilityId", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<CapabilityProcessRelation>()?;
                Ok(Some(FieldValue::value(v.capability_id.clone())))
            })
        }))
        .field(Field::new("processId", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<CapabilityProcessRelation>()?;
                Ok(Some(FieldValue::value(v.process_id.clone())))
            })
        }))
        .field(Field::new("logicalId", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<CapabilityProcessRelation>()?;
                Ok(Some(FieldValue::value(v.logical_id.clone())))
            })
        }))
        .field(Field::new("processName", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<CapabilityProcessRelation>()?;
                Ok(Some(FieldValue::value(v.process_name.clone())))
            })
        }))
        .field(Field::new("businessVersion", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<CapabilityProcessRelation>()?;
                Ok(Some(FieldValue::value(v.business_version.clone())))
            })
        }))
        .field(Field::new("status", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<CapabilityProcessRelation>()?;
                Ok(Some(FieldValue::value(v.status.clone())))
            })
        }))
        .field(Field::new("valid", TypeRef::named_nn(TypeRef::BOOLEAN), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<CapabilityProcessRelation>()?;
                Ok(Some(FieldValue::value(v.valid)))
            })
        }));
    builder.outputs.push(cap_proc_relation_type);

    // ── AffectedProcessLink output type ───────────────────────────────
    let affected_link_type = Object::new("AffectedProcessLink")
        .field(Field::new("capabilityId", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<AffectedProcessLinkOutput>()?;
                Ok(Some(FieldValue::value(v.capability_id.clone())))
            })
        }))
        .field(Field::new("capabilityName", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<AffectedProcessLinkOutput>()?;
                Ok(Some(FieldValue::value(v.capability_name.clone())))
            })
        }))
        .field(Field::new("oldVersion", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<AffectedProcessLinkOutput>()?;
                Ok(Some(FieldValue::value(v.old_version.clone())))
            })
        }))
        .field(Field::new("newVersion", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<AffectedProcessLinkOutput>()?;
                Ok(Some(FieldValue::value(v.new_version.clone())))
            })
        }));
    builder.outputs.push(affected_link_type);

    // ── ProcessPublishVersionResult output type ───────────────────────
    let publish_result_type = Object::new("ProcessPublishVersionResult")
        .field(Field::new("id", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<ProcessPublishVersionOutput>()?;
                Ok(Some(FieldValue::value(v.id.clone())))
            })
        }))
        .field(Field::new("businessVersion", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<ProcessPublishVersionOutput>()?;
                Ok(Some(FieldValue::value(v.business_version.clone())))
            })
        }))
        .field(Field::new("status", TypeRef::named_nn(TypeRef::STRING), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<ProcessPublishVersionOutput>()?;
                Ok(Some(FieldValue::value(v.status.clone())))
            })
        }))
        .field(Field::new("affectedLinks", TypeRef::named_nn_list_nn("AffectedProcessLink"), |ctx| {
            FieldFuture::new(async move {
                let v = ctx.parent_value.try_downcast_ref::<ProcessPublishVersionOutput>()?;
                let values: Vec<FieldValue> = v
                    .affected_links
                    .iter()
                    .cloned()
                    .map(FieldValue::owned_any)
                    .collect();
                Ok(Some(FieldValue::list(values)))
            })
        }));
    builder.outputs.push(publish_result_type);

    // ── myMembership ──────────────────────────────────────────────────
    // Returns the caller's own role in a space (or null for non-members).
    // Unlike the admin-gated auto-generated `spaceMembers` query, this is
    // safe for non-admin editors/owners and only discloses the caller's own
    // membership — never other users'.
    let my_membership_query = Field::new(
        "myMembership",
        TypeRef::named("MyMembership"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let claims = require_claims(&ctx)?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let service = space_service(db);
                let membership = service
                    .my_membership(space_id, claims.user_id)
                    .await
                    .map_err(domain_err_to_graphql)?;
                Ok(membership.map(|m| FieldValue::owned_any(MyMembership {
                    role: match m.role {
                        SpaceRole::Owner => "owner".to_owned(),
                        SpaceRole::Editor => "editor".to_owned(),
                    },
                })))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(my_membership_query);

    // ── spaceMembersBySpace ──────────────────────────────────────────
    // Returns `{ userId, name, role }` for each member of the space. The name
    // is resolved by joining with the `users` table so the member-management
    // UI can show a human-readable name instead of a truncated UUID. This is
    // membership-scoped (ensure_space_read_access) and exposes no admin-only
    // PII (email is omitted).
    let members_query = Field::new(
        "spaceMembersBySpace",
        TypeRef::named_nn_list_nn("SpaceMemberWithUser"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                ensure_space_read_access(&ctx, db, space_id).await?;

                let members = space_member::Entity::find()
                    .filter(space_member::Column::SpaceId.eq(space_id))
                    .all(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                let user_ids: Vec<Uuid> = members.iter().map(|m| m.user_id).collect();
                let users = if user_ids.is_empty() {
                    Vec::new()
                } else {
                    user::Entity::find()
                        .filter(user::Column::Id.is_in(user_ids))
                        .all(db)
                        .await
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?
                };
                let name_by_id: std::collections::HashMap<Uuid, String> =
                    users.into_iter().map(|u| (u.id, u.name)).collect();
                let list: Vec<FieldValue> = members
                    .into_iter()
                    .map(|m| {
                        FieldValue::owned_any(SpaceMemberWithUser {
                            user_id: m.user_id.to_string(),
                            name: name_by_id
                                .get(&m.user_id)
                                .cloned()
                                .unwrap_or_else(|| m.user_id.to_string()),
                            role: m.role,
                        })
                    })
                    .collect();
                Ok(Some(FieldValue::list(list)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(members_query);

    // ── spaceUserByEmail ─────────────────────────────────────────────
    // Allows a space owner/editor to look up a user by email for the purpose
    // of adding them as a member, without requiring global admin. Authorization
    // is enforced via `ensure_space_edit_access` (caller must be able to edit
    // the space). The result is a SpaceUserLookup (id/name only) rather than
    // the full Users entity, so sensitive fields (password_hash, tokens) and
    // admin-only fields (email) are never exposed through this path.
    let user_lookup = Field::new(
        "spaceUserByEmail",
        TypeRef::named("SpaceUserLookup"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                ensure_space_edit_access(&ctx, db, space_id).await?;
                let email = ctx.args.try_get("email")?.string()?.to_owned();

                let model = user::Entity::find()
                    .filter(user::Column::Email.eq(email))
                    .one(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(model.map(|m| FieldValue::owned_any(SpaceUserLookup {
                    id: m.id.to_string(),
                    name: m.name,
                })))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("email", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(user_lookup);

    // ========================================================================
    // Visibility-aware space-scoped read queries (R2).
    //
    // The seaography auto-generated query for `organizations`/`value_streams`/
    // `business_capabilities`/`business_processes` and their children is
    // admin-only (see ADMIN_READ_ENTITIES). Non-admin / anonymous callers must
    // use these custom queries, which enforce visibility + membership before
    // returning any row. Public spaces are readable by anyone (including
    // anonymous); private spaces require membership (Admin bypasses).
    // ========================================================================

    /// Resolve the caller's `(user_id, role)` for visibility checks. Anonymous
    /// callers get `(None, UserRole::Viewer)` — Viewer has no global bypass, so
    /// the visibility branch correctly falls through to "public only".
    fn caller_identity(ctx: &async_graphql::dynamic::ResolverContext<'_>) -> (Option<Uuid>, shared_common::enums::UserRole) {
        match ctx.data_opt::<crate::middleware::Claims>() {
            Some(c) => (Some(c.user_id), c.user_role()),
            None => (None, shared_common::enums::UserRole::Viewer),
        }
    }

    // ── spaces ─────────────────────────────────────────────────────────
    // Anonymous: public non-deleted spaces. Authenticated: public + private
    // spaces they are a member of. Admin: all non-deleted spaces.
    let spaces_query = Field::new(
        "spaces",
        TypeRef::named_nn_list_nn("Organizations"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                let list = if let Some(uid) = actor_id {
                    service.list_visible(uid, actor_role).await
                } else {
                    service.list_public().await
                }
                .map_err(domain_err_to_graphql)?;
                let values: Vec<FieldValue> = list
                    .into_iter()
                    .map(|s| FieldValue::owned_any(domain_space_to_model(&s)))
                    .collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    );
    builder.queries.push(spaces_query);

    // ── spaceById ──────────────────────────────────────────────────────
    let space_by_id = Field::new(
        "spaceById",
        TypeRef::named("Organizations"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "id")?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let space = service
                    .find_space(space_id)
                    .await
                    .map_err(domain_err_to_graphql)?;
                Ok(space.map(|s| FieldValue::owned_any(domain_space_to_model(&s))))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(space_by_id);

    // Helper macro-like closures are awkward in Rust closures; inline the
    // visibility check per query for clarity (mirrors `spaceById`).

    // ── valueStreamsBySpace ────────────────────────────────────────────
    let vs_by_space = Field::new(
        "valueStreamsBySpace",
        TypeRef::named_nn_list_nn("ValueStreams"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = value_stream::Entity::find()
                    .filter(value_stream::Column::SpaceId.eq(space_id))
                    .filter(value_stream::Column::DeletedAt.is_null())
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(vs_by_space);

    // ── businessCapabilitiesBySpace ───────────────────────────────────
    let cap_by_space = Field::new(
        "businessCapabilitiesBySpace",
        TypeRef::named_nn_list_nn("BusinessCapabilities"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = business_capability::Entity::find()
                    .filter(business_capability::Column::SpaceId.eq(space_id))
                    .filter(business_capability::Column::DeletedAt.is_null())
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(cap_by_space);

    // ── businessProcessesBySpace ──────────────────────────────────────
    let proc_by_space = Field::new(
        "businessProcessesBySpace",
        TypeRef::named_nn_list_nn("BusinessProcesses"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = business_process::Entity::find()
                    .filter(business_process::Column::SpaceId.eq(space_id))
                    .filter(business_process::Column::DeletedAt.is_null())
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(proc_by_space);

    // ── valueStreamCountBySpace ───────────────────────────────────────
    // Lightweight count query for dashboard stats; avoids loading full rows.
    let vs_count = Field::new(
        "valueStreamCountBySpace",
        TypeRef::named_nn(TypeRef::INT),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let count = value_stream::Entity::find()
                    .filter(value_stream::Column::SpaceId.eq(space_id))
                    .filter(value_stream::Column::DeletedAt.is_null())
                    .count(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                Ok(Some(FieldValue::value(count as i64)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(vs_count);

    // ── businessCapabilityCountBySpace ────────────────────────────────
    let cap_count = Field::new(
        "businessCapabilityCountBySpace",
        TypeRef::named_nn(TypeRef::INT),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let count = business_capability::Entity::find()
                    .filter(business_capability::Column::SpaceId.eq(space_id))
                    .filter(business_capability::Column::DeletedAt.is_null())
                    .count(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                Ok(Some(FieldValue::value(count as i64)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(cap_count);

    // ── businessProcessCountBySpace ───────────────────────────────────
    let proc_count = Field::new(
        "businessProcessCountBySpace",
        TypeRef::named_nn(TypeRef::INT),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let count = business_process::Entity::find()
                    .filter(business_process::Column::SpaceId.eq(space_id))
                    .filter(business_process::Column::DeletedAt.is_null())
                    .count(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                Ok(Some(FieldValue::value(count as i64)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(proc_count);

    // ── valueStreamById ───────────────────────────────────────────────
    // Resolves the owning space from the value stream, then enforces
    // visibility. Used by value-stream-detail.tsx.
    let vs_by_id = Field::new(
        "valueStreamById",
        TypeRef::named("ValueStreams"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let id = parse_uuid_arg(&ctx, "id")?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                // Filter by both id and SpaceId so a caller with read access to
                // one space cannot fetch a value stream belonging to a different
                // space by guessing its id.
                let row = value_stream::Entity::find()
                    .filter(value_stream::Column::Id.eq(id))
                    .filter(value_stream::Column::SpaceId.eq(space_id))
                    .filter(value_stream::Column::DeletedAt.is_null())
                    .one(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                Ok(row.map(FieldValue::owned_any))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(vs_by_id);

    // ── valueStreamsBySpaceAndLogicalId ───────────────────────────────
    // Used by version-control.tsx to fetch all versions of a value stream.
    let vs_by_logical = Field::new(
        "valueStreamsBySpaceAndLogicalId",
        TypeRef::named_nn_list_nn("ValueStreams"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let logical_id = parse_uuid_arg(&ctx, "logicalId")?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = value_stream::Entity::find()
                    .filter(value_stream::Column::SpaceId.eq(space_id))
                    .filter(value_stream::Column::LogicalId.eq(logical_id))
                    .filter(value_stream::Column::DeletedAt.is_null())
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("logicalId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(vs_by_logical);

    // ── Child-entity by-parent queries ────────────────────────────────
    // These close the cross-tenant read gap: the auto-query for child
    // entities is admin-only, so non-admins must use these. Each resolves the
    // owning space from the parent and enforces visibility before filtering by
    // the parent id.

    // processStepsByProcess
    let steps_by_process = Field::new(
        "processStepsByProcess",
        TypeRef::named_nn_list_nn("ProcessSteps"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let process_id = parse_uuid_arg(&ctx, "processId")?;
                let space_id = space_of_process(db, process_id).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = process_step::Entity::find()
                    .filter(process_step::Column::ProcessId.eq(process_id))
                    .filter(process_step::Column::DeletedAt.is_null())
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("processId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(steps_by_process);

    // valueStreamStagesByValueStream
    let stages_by_vs = Field::new(
        "valueStreamStagesByValueStream",
        TypeRef::named_nn_list_nn("ValueStreamStages"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let vs_id = parse_uuid_arg(&ctx, "valueStreamId")?;
                let space_id = space_of_value_stream(db, vs_id).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = value_stream_stage::Entity::find()
                    .filter(value_stream_stage::Column::ValueStreamId.eq(vs_id))
                    .filter(value_stream_stage::Column::DeletedAt.is_null())
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("valueStreamId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(stages_by_vs);

    // capabilityProcessesByCapability
    let cp_by_cap = Field::new(
        "capabilityProcessesByCapability",
        TypeRef::named_nn_list_nn("CapabilityProcesses"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let cap_id = parse_uuid_arg(&ctx, "capabilityId")?;
                let space_id = space_of_capability(db, cap_id).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = capability_process::Entity::find()
                    .filter(capability_process::Column::CapabilityId.eq(cap_id))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("capabilityId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(cp_by_cap);

    // capabilityProcessRelations
    // Version-anchored view of a capability's process links: enriches each raw
    // join row with the process name, business version, lifecycle status and
    // derived validity (`valid = status ∈ {active, deprecated}`). Lets the UI
    // warn when a process publish left a link pointing at a deprecated row.
    let cp_relations = Field::new(
        "capabilityProcessRelations",
        TypeRef::named_nn_list_nn("CapabilityProcessRelation"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let cap_id = parse_uuid_arg(&ctx, "capabilityId")?;
                let space_id = space_of_capability(db, cap_id).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let links = capability_process::Entity::find()
                    .filter(capability_process::Column::CapabilityId.eq(cap_id))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let process_ids: Vec<Uuid> = links.iter().map(|l| l.process_id).collect();
                let processes = if process_ids.is_empty() {
                    Vec::new()
                } else {
                    business_process::Entity::find()
                        .filter(business_process::Column::Id.is_in(process_ids.clone()))
                        .all(db)
                        .await
                        .map_err(db_err_to_graphql)?
                };
                let mut relations: Vec<CapabilityProcessRelation> = processes
                    .iter()
                    .map(|p| CapabilityProcessRelation {
                        capability_id: cap_id.to_string(),
                        process_id: p.id.to_string(),
                        logical_id: p.logical_id.to_string(),
                        process_name: p.name.clone(),
                        business_version: p.business_version.clone(),
                        status: format!("{:?}", p.status).to_lowercase(),
                        valid: matches!(
                            p.status,
                            LifecycleStatus::Active | LifecycleStatus::Deprecated
                        ),
                    })
                    .collect();
                // Preserve the join-row order.
                relations.sort_by_key(|r| {
                    process_ids
                        .iter()
                        .position(|id| id.to_string() == r.process_id)
                        .unwrap_or(0)
                });
                let values: Vec<FieldValue> =
                    relations.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("capabilityId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(cp_relations);

    // stageCapabilitiesByStage
    let sc_by_stage = Field::new(
        "stageCapabilitiesByStage",
        TypeRef::named_nn_list_nn("StageCapabilities"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let stage_id = parse_uuid_arg(&ctx, "stageId")?;
                let space_id = space_of_stage(db, stage_id).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = stage_capability::Entity::find()
                    .filter(stage_capability::Column::StageId.eq(stage_id))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("stageId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(sc_by_stage);

    // capabilitiesByStage
    // Resolves a stage's linked capabilities directly as `BusinessCapabilities`
    // entities (id/name/status via the seaography object type) instead of raw
    // `StageCapabilities` join rows. Used by the value-stream stage detail UI.
    let caps_by_stage = Field::new(
        "capabilitiesByStage",
        TypeRef::named_nn_list_nn("BusinessCapabilities"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let stage_id = parse_uuid_arg(&ctx, "stageId")?;
                let space_id = space_of_stage(db, stage_id).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let links = stage_capability::Entity::find()
                    .filter(stage_capability::Column::StageId.eq(stage_id))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let cap_ids: Vec<Uuid> = links.iter().map(|l| l.capability_id).collect();
                let rows = if cap_ids.is_empty() {
                    Vec::new()
                } else {
                    business_capability::Entity::find()
                        .filter(business_capability::Column::Id.is_in(cap_ids))
                        .all(db)
                        .await
                        .map_err(db_err_to_graphql)?
                };
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("stageId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(caps_by_stage);

    // capabilitiesByProcess
    // Lists the capabilities currently linked to a process. Used by the
    // publish-version confirmation dialog to show which capabilities would be
    // affected (their links would point at the soon-deprecated old version).
    let caps_by_process = Field::new(
        "capabilitiesByProcess",
        TypeRef::named_nn_list_nn("BusinessCapabilities"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let process_id = parse_uuid_arg(&ctx, "processId")?;
                let space_id = space_of_process(db, process_id).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let links = capability_process::Entity::find()
                    .filter(capability_process::Column::ProcessId.eq(process_id))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let cap_ids: Vec<Uuid> = links.iter().map(|l| l.capability_id).collect();
                let rows = if cap_ids.is_empty() {
                    Vec::new()
                } else {
                    business_capability::Entity::find()
                        .filter(business_capability::Column::Id.is_in(cap_ids))
                        .all(db)
                        .await
                        .map_err(db_err_to_graphql)?
                };
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("processId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(caps_by_process);

    // ── Application-architecture by-space / by-parent queries ─────────
    // Non-admin reads of the new P2/P3 entities go through these queries,
    // which enforce space visibility/membership before returning rows.

    // applicationComponentsBySpace
    let ac_by_space = Field::new(
        "applicationComponentsBySpace",
        TypeRef::named_nn_list_nn("ApplicationComponents"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = application_component::Entity::find()
                    .filter(application_component::Column::SpaceId.eq(space_id))
                    .filter(application_component::Column::DeletedAt.is_null())
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(ac_by_space);

    // applicationProcessesBySpace
    let ap_by_space = Field::new(
        "applicationProcessesBySpace",
        TypeRef::named_nn_list_nn("ApplicationProcesses"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = application_process::Entity::find()
                    .filter(application_process::Column::SpaceId.eq(space_id))
                    .filter(application_process::Column::DeletedAt.is_null())
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(ap_by_space);

    // applicationProcessStepsByProcess
    let aps_by_process = Field::new(
        "applicationProcessStepsByProcess",
        TypeRef::named_nn_list_nn("ApplicationProcessSteps"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let process_id = parse_uuid_arg(&ctx, "processId")?;
                let space_id = space_of_application_process(db, process_id).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = application_process_step::Entity::find()
                    .filter(application_process_step::Column::ProcessId.eq(process_id))
                    .filter(application_process_step::Column::DeletedAt.is_null())
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("processId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(aps_by_process);

    // ── v2.1 entity BySpace queries ───────────────────────────────────
    // organizationalUnitsBySpace
    let ou_by_space = Field::new("organizationalUnitsBySpace", TypeRef::named_nn_list_nn("OrganizationalUnits"), |ctx| {
        FieldFuture::new(async move {
            let db = ctx.data::<DatabaseConnection>()?;
            let space_id = parse_uuid_arg(&ctx, "spaceId")?;
            let (actor_id, actor_role) = caller_identity(&ctx);
            let service = space_service(db);
            service.ensure_can_read(space_id, actor_id, actor_role).await.map_err(domain_err_to_graphql)?;
            let rows = organizational_unit::Entity::find()
                .filter(organizational_unit::Column::SpaceId.eq(space_id))
                .filter(organizational_unit::Column::DeletedAt.is_null())
                .all(db).await.map_err(db_err_to_graphql)?;
            let values: Vec<FieldValue> = rows.into_iter().map(FieldValue::owned_any).collect();
            Ok(Some(FieldValue::list(values)))
        })
    })
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(ou_by_space);

    // businessRolesBySpace
    let br_by_space = Field::new("businessRolesBySpace", TypeRef::named_nn_list_nn("BusinessRoles"), |ctx| {
        FieldFuture::new(async move {
            let db = ctx.data::<DatabaseConnection>()?;
            let space_id = parse_uuid_arg(&ctx, "spaceId")?;
            let (actor_id, actor_role) = caller_identity(&ctx);
            let service = space_service(db);
            service.ensure_can_read(space_id, actor_id, actor_role).await.map_err(domain_err_to_graphql)?;
            let rows = business_role::Entity::find()
                .filter(business_role::Column::SpaceId.eq(space_id))
                .filter(business_role::Column::DeletedAt.is_null())
                .all(db).await.map_err(db_err_to_graphql)?;
            let values: Vec<FieldValue> = rows.into_iter().map(FieldValue::owned_any).collect();
            Ok(Some(FieldValue::list(values)))
        })
    })
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(br_by_space);

    // functionalModulesBySpace
    let fm_by_space = Field::new("functionalModulesBySpace", TypeRef::named_nn_list_nn("FunctionalModules"), |ctx| {
        FieldFuture::new(async move {
            let db = ctx.data::<DatabaseConnection>()?;
            let space_id = parse_uuid_arg(&ctx, "spaceId")?;
            let (actor_id, actor_role) = caller_identity(&ctx);
            let service = space_service(db);
            service.ensure_can_read(space_id, actor_id, actor_role).await.map_err(domain_err_to_graphql)?;
            let rows = functional_module::Entity::find()
                .filter(functional_module::Column::SpaceId.eq(space_id))
                .filter(functional_module::Column::DeletedAt.is_null())
                .all(db).await.map_err(db_err_to_graphql)?;
            let values: Vec<FieldValue> = rows.into_iter().map(FieldValue::owned_any).collect();
            Ok(Some(FieldValue::list(values)))
        })
    })
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(fm_by_space);

    // applicationInterfacesBySpace
    let ai_by_space = Field::new("applicationInterfacesBySpace", TypeRef::named_nn_list_nn("ApplicationInterfaces"), |ctx| {
        FieldFuture::new(async move {
            let db = ctx.data::<DatabaseConnection>()?;
            let space_id = parse_uuid_arg(&ctx, "spaceId")?;
            let (actor_id, actor_role) = caller_identity(&ctx);
            let service = space_service(db);
            service.ensure_can_read(space_id, actor_id, actor_role).await.map_err(domain_err_to_graphql)?;
            let rows = application_interface::Entity::find()
                .filter(application_interface::Column::SpaceId.eq(space_id))
                .filter(application_interface::Column::DeletedAt.is_null())
                .all(db).await.map_err(db_err_to_graphql)?;
            let values: Vec<FieldValue> = rows.into_iter().map(FieldValue::owned_any).collect();
            Ok(Some(FieldValue::list(values)))
        })
    })
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(ai_by_space);

    // capabilityRealizationsByCapability
    let cr_by_cap = Field::new(
        "capabilityRealizationsByCapability",
        TypeRef::named_nn_list_nn("CapabilityRealizations"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let cap_id = parse_uuid_arg(&ctx, "capabilityId")?;
                let space_id = space_of_capability(db, cap_id).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = capability_realization::Entity::find()
                    .filter(capability_realization::Column::CapabilityId.eq(cap_id))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("capabilityId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(cr_by_cap);

    // processReferencesByApplicationProcess
    let q = Field::new(
        "processReferencesByApplicationProcess",
        TypeRef::named_nn_list_nn("ProcessReferences"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let sid = parse_uuid_arg(&ctx, "applicationProcessId")?;
                let space_id = space_of_application_process(db, sid).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = process_reference::Entity::find()
                    .filter(process_reference::Column::ApplicationProcessId.eq(sid))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("applicationProcessId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(q);

    // processReferencesByBusinessProcess
    let q = Field::new(
        "processReferencesByBusinessProcess",
        TypeRef::named_nn_list_nn("ProcessReferences"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let sid = parse_uuid_arg(&ctx, "businessProcessId")?;
                let space_id = space_of_process(db, sid).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = process_reference::Entity::find()
                    .filter(process_reference::Column::BusinessProcessId.eq(sid))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("businessProcessId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(q);

    // capabilityRealizationsBySpace
    // Aggregation over all capabilities of a space; avoids per-capability
    // N+1 queries on the architecture overview page.
    let q = Field::new(
        "capabilityRealizationsBySpace",
        TypeRef::named_nn_list_nn("CapabilityRealizations"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let cap_ids: Vec<Uuid> = business_capability::Entity::find()
                    .filter(business_capability::Column::SpaceId.eq(space_id))
                    .filter(business_capability::Column::DeletedAt.is_null())
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?
                    .into_iter()
                    .map(|c| c.id)
                    .collect();
                let rows = if cap_ids.is_empty() {
                    Vec::new()
                } else {
                    capability_realization::Entity::find()
                        .filter(capability_realization::Column::CapabilityId.is_in(cap_ids))
                        .all(db)
                        .await
                        .map_err(db_err_to_graphql)?
                };
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(q);

    // processReferencesBySpace
    // Aggregation over all business/application processes of a space; avoids
    // per-process N+1 queries on the architecture overview page.
    let q = Field::new(
        "processReferencesBySpace",
        TypeRef::named_nn_list_nn("ProcessReferences"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let bp_ids: Vec<Uuid> = business_process::Entity::find()
                    .filter(business_process::Column::SpaceId.eq(space_id))
                    .filter(business_process::Column::DeletedAt.is_null())
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?
                    .into_iter()
                    .map(|p| p.id)
                    .collect();
                let ap_ids: Vec<Uuid> = application_process::Entity::find()
                    .filter(application_process::Column::SpaceId.eq(space_id))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?
                    .into_iter()
                    .map(|p| p.id)
                    .collect();
                let rows = if bp_ids.is_empty() && ap_ids.is_empty() {
                    Vec::new()
                } else {
                    process_reference::Entity::find()
                        .filter(
                            sea_orm::Condition::any()
                                .add(process_reference::Column::BusinessProcessId.is_in(bp_ids))
                                .add(process_reference::Column::ApplicationProcessId.is_in(ap_ids)),
                        )
                        .all(db)
                        .await
                        .map_err(db_err_to_graphql)?
                };
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(q);

    // assignmentsByOrganization
    let q = Field::new(
        "assignmentsByOrganization",
        TypeRef::named_nn_list_nn("Assignments"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let sid = parse_uuid_arg(&ctx, "organizationId")?;
                let space_id = space_of_organizational_unit(db, sid).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = assignment::Entity::find()
                    .filter(assignment::Column::OrganizationId.eq(sid))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("organizationId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(q);

    // assignmentsByBusinessRole
    let q = Field::new(
        "assignmentsByBusinessRole",
        TypeRef::named_nn_list_nn("Assignments"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let sid = parse_uuid_arg(&ctx, "businessRoleId")?;
                let space_id = space_of_business_role(db, sid).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = assignment::Entity::find()
                    .filter(assignment::Column::BusinessRoleId.eq(sid))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("businessRoleId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(q);

    // participationsByBusinessRole
    let q = Field::new(
        "participationsByBusinessRole",
        TypeRef::named_nn_list_nn("Participations"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let sid = parse_uuid_arg(&ctx, "businessRoleId")?;
                let space_id = space_of_business_role(db, sid).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = participation::Entity::find()
                    .filter(participation::Column::BusinessRoleId.eq(sid))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("businessRoleId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(q);

    // participationsByBusinessProcess
    let q = Field::new(
        "participationsByBusinessProcess",
        TypeRef::named_nn_list_nn("Participations"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let sid = parse_uuid_arg(&ctx, "businessProcessId")?;
                let space_id = space_of_process(db, sid).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = participation::Entity::find()
                    .filter(participation::Column::BusinessProcessId.eq(sid))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("businessProcessId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(q);

    // moduleContainmentsByFunctionalModule
    let q = Field::new(
        "moduleContainmentsByFunctionalModule",
        TypeRef::named_nn_list_nn("ModuleContainments"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let sid = parse_uuid_arg(&ctx, "functionalModuleId")?;
                let space_id = space_of_functional_module(db, sid).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = module_containment::Entity::find()
                    .filter(module_containment::Column::FunctionalModuleId.eq(sid))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("functionalModuleId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(q);

    // moduleContainmentsByApplicationComponent
    let q = Field::new(
        "moduleContainmentsByApplicationComponent",
        TypeRef::named_nn_list_nn("ModuleContainments"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let sid = parse_uuid_arg(&ctx, "applicationComponentId")?;
                let space_id = space_of_application_component(db, sid).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = module_containment::Entity::find()
                    .filter(module_containment::Column::ApplicationComponentId.eq(sid))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("applicationComponentId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(q);

    // interfaceExposuresByFunctionalModule
    let q = Field::new(
        "interfaceExposuresByFunctionalModule",
        TypeRef::named_nn_list_nn("InterfaceExposures"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let sid = parse_uuid_arg(&ctx, "functionalModuleId")?;
                let space_id = space_of_functional_module(db, sid).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = interface_exposure::Entity::find()
                    .filter(interface_exposure::Column::FunctionalModuleId.eq(sid))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("functionalModuleId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(q);

    // interfaceExposuresByApplicationInterface
    let q = Field::new(
        "interfaceExposuresByApplicationInterface",
        TypeRef::named_nn_list_nn("InterfaceExposures"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let sid = parse_uuid_arg(&ctx, "applicationInterfaceId")?;
                let space_id = space_of_application_interface(db, sid).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = interface_exposure::Entity::find()
                    .filter(interface_exposure::Column::ApplicationInterfaceId.eq(sid))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("applicationInterfaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(q);

    // orchestrationsByApplicationProcess
    let q = Field::new(
        "orchestrationsByApplicationProcess",
        TypeRef::named_nn_list_nn("Orchestrations"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let sid = parse_uuid_arg(&ctx, "applicationProcessId")?;
                let space_id = space_of_application_process(db, sid).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = orchestration::Entity::find()
                    .filter(orchestration::Column::ApplicationProcessId.eq(sid))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("applicationProcessId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(q);

    // orchestrationsByFunctionalModule
    let q = Field::new(
        "orchestrationsByFunctionalModule",
        TypeRef::named_nn_list_nn("Orchestrations"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let sid = parse_uuid_arg(&ctx, "functionalModuleId")?;
                let space_id = space_of_functional_module(db, sid).await?;
                let (actor_id, actor_role) = caller_identity(&ctx);
                let service = space_service(db);
                service
                    .ensure_can_read(space_id, actor_id, actor_role)
                    .await
                    .map_err(domain_err_to_graphql)?;
                let rows = orchestration::Entity::find()
                    .filter(orchestration::Column::FunctionalModuleId.eq(sid))
                    .all(db)
                    .await
                    .map_err(db_err_to_graphql)?;
                let values: Vec<FieldValue> =
                    rows.into_iter().map(FieldValue::owned_any).collect();
                Ok(Some(FieldValue::list(values)))
            })
        },
    )
    .argument(InputValue::new("functionalModuleId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(q);

}

// ============================================================================
// Build schema
// ============================================================================

pub async fn build_graphql_schema(db: &DatabaseConnection) -> anyhow::Result<GraphqlSchema> {
    let context: &'static BuilderContext = Box::leak(Box::new(BuilderContext {
        hooks: LifecycleHooks::new(GraphqlAuthGuard),
        types: TypesMapConfig {
            time_library: TimeLibrary::Chrono,
            timestamp_rfc3339: true,
            ..Default::default()
        },
        ..Default::default()
    }));

    let mut builder = Builder::new(context, db.clone());

    // ── User management: seaography CRUD (queries + mutations) ────────
    register_entity_with_mutations::<user::Entity, user::ActiveModel>(&mut builder);
    register_entity_with_mutations::<refresh_token::Entity, refresh_token::ActiveModel>(&mut builder);
    register_entity_with_mutations::<oauth_authorization_code::Entity, oauth_authorization_code::ActiveModel>(&mut builder);

    // ── Business architecture: queries only via seaography ────────────
    // Mutations for value_stream, business_capability, and business_process go
    // through custom domain mutations that enforce space-level ACL.
    // Sub-entities (process_step, value_stream_stage, capability_process,
    // stage_capability) likewise use queries-only registration here; their
    // mutations are registered below and enforce space-level ACL by resolving
    // the owning space from the parent entity before any write.
    register_entity::<business_capability::Entity>(&mut builder);  // queries only
    register_entity::<business_process::Entity>(&mut builder);     // queries only
    register_entity::<process_step::Entity>(&mut builder);         // queries only
    register_entity::<value_stream::Entity>(&mut builder);  // queries only
    register_entity::<value_stream_stage::Entity>(&mut builder);   // queries only
    register_entity::<capability_process::Entity>(&mut builder);   // queries only
    register_entity::<stage_capability::Entity>(&mut builder);     // queries only
    register_entity::<application_component::Entity>(&mut builder);  // queries only
    register_entity::<application_process::Entity>(&mut builder);   // queries only
    register_entity::<application_process_step::Entity>(&mut builder); // queries only
    register_entity::<capability_realization::Entity>(&mut builder); // queries only
    register_entity::<organizational_unit::Entity>(&mut builder);  // queries only
    register_entity::<business_role::Entity>(&mut builder);        // queries only
    register_entity::<functional_module::Entity>(&mut builder);    // queries only
    register_entity::<application_interface::Entity>(&mut builder); // queries only
    register_entity::<assignment::Entity>(&mut builder);           // queries only
    register_entity::<participation::Entity>(&mut builder);        // queries only
    register_entity::<module_containment::Entity>(&mut builder);   // queries only
    register_entity::<interface_exposure::Entity>(&mut builder);   // queries only
    register_entity::<process_reference::Entity>(&mut builder);    // queries only
    register_entity::<orchestration::Entity>(&mut builder);        // queries only

    // ── Spaces (reuses `organizations` table) + membership ─────────────
    // Queries are admin-only via the auto-generated query (see ADMIN_READ_ENTITIES);
    // non-admin / anonymous reads go through the custom space-scoped queries
    // registered below (`spaces`/`spaceById`/`*BySpace`). Writes go through
    // custom domain mutations registered below.
    // `space_invitations` is intentionally NOT registered (R7 dead-code cleanup):
    // the table exists but has no create/accept mutation and no consumers.
    register_entity::<space::Entity>(&mut builder);
    register_entity::<space_member::Entity>(&mut builder);

    // ── Custom domain mutations for ValueStream ───────────────────────
    register_value_stream_domain_mutations(&mut builder);

    // ── Custom domain mutations for BusinessCapability/Process ───────
    register_capability_domain_mutations(&mut builder);
    register_process_domain_mutations(&mut builder);

    // ── Custom domain mutations for sub-entities (space-level ACL) ────
    register_sub_entity_domain_mutations(&mut builder);

    // ── Custom domain mutations for application architecture (P2) ────
    register_application_component_domain_mutations(&mut builder);
    register_application_process_domain_mutations(&mut builder);
    register_application_process_step_domain_mutations(&mut builder);
    register_v21_entity_domain_mutations(&mut builder);
    register_realization_domain_mutations(&mut builder);

    // ── Custom domain mutations for Space + membership ────────────────
    register_space_domain_mutations(&mut builder);

    // ── Custom space-scoped queries (membership-enforced) ─────────────
    register_space_scoped_queries(&mut builder);

    // ── DataLoaders ───────────────────────────────────────────────────
    builder = builder
        .register_entity_dataloader_one_to_one(user::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(user::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(refresh_token::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(refresh_token::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(oauth_authorization_code::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(oauth_authorization_code::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(business_capability::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(business_capability::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(business_process::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(business_process::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(process_step::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(process_step::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(value_stream::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(value_stream::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(value_stream_stage::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(value_stream_stage::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(capability_process::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(capability_process::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(stage_capability::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(stage_capability::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(application_component::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(application_component::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(application_process::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(application_process::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(application_process_step::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(application_process_step::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(capability_realization::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(capability_realization::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(organizational_unit::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(organizational_unit::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(business_role::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(business_role::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(functional_module::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(functional_module::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(application_interface::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(application_interface::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(assignment::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(assignment::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(participation::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(participation::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(module_containment::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(module_containment::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(interface_exposure::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(interface_exposure::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(process_reference::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(process_reference::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(orchestration::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(orchestration::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(space::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(space::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(space_member::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(space_member::Entity, tokio::spawn);

    // ── Explicitly register enum types used in custom mutations ───────
    // seaography auto-registers enums for entity query fields, but custom
    // mutations reference them by string name and need them registered explicitly.
    builder.register_enumeration::<CapabilityLevel>();
    builder.register_enumeration::<MaturityLevel>();
    builder.register_enumeration::<BusinessValueRating>();
    builder.register_enumeration::<CostRating>();
    builder.register_enumeration::<CapabilityStatus>();
    builder.register_enumeration::<LifecycleStatus>();
    builder.register_enumeration::<AutomationLevel>();
    builder.register_enumeration::<ApplicationComponentType>();
    builder.register_enumeration::<ApplicationComponentStatus>();
    builder.register_enumeration::<ApplicationProcessTrigger>();
    builder.register_enumeration::<RaciRole>();
    builder.register_enumeration::<OrganizationalUnitType>();
    builder.register_enumeration::<FunctionalModuleStatus>();
    builder.register_enumeration::<ApplicationInterfaceProtocol>();
    builder.register_enumeration::<CapabilityRealizationTargetType>();

    // SpaceVisibility is used as a field type on the `Organizations` entity
    // (the `visibility` column). If the enum is not registered, seaography
    // silently skips the field, causing `Unknown field "visibility" on type
    // "Organizations"` at query time.
    builder.register_enumeration::<SpaceVisibility>();

    let schema = builder.schema_builder()
        .data(db.clone())
        .finish()?;

    Ok(schema)
}

#[cfg(test)]
mod tests {
    use super::*;
    use migration::MigratorTrait;

    /// Regression guard for the deploy-script failure caused by a custom
    /// query/mutation referencing a type name that seaography does not
    /// register (e.g. `BusinessCapabilityProcesses` vs the actual
    /// `CapabilityProcesses` derived from `table_name`). `finish()` returns
    /// `Err` for such mismatches, crashing the backend at startup.
    #[tokio::test]
    async fn build_graphql_schema_succeeds_against_migrated_sqlite() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        migration::Migrator::up(&db, None).await.unwrap();

        build_graphql_schema(&db)
            .await
            .expect("GraphQL schema must build; a referenced type name is not registered");
    }
}
