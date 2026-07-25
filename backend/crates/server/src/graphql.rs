use axum::response::IntoResponse;
use sea_orm::DatabaseConnection;
use seaography::{Builder, BuilderContext, GuardAction, LifecycleHooks, LifecycleHooksInterface, OperationType, RelatedEntityFilter, RelationBuilder, TimeLibrary, TypesMapConfig};
use uuid::Uuid;

use user_management::infrastructure::persistence::entities::{
    oauth_authorization_code, refresh_token, user,
};
use business_architecture::infrastructure::persistence::entities::{
    business_capability, business_process, capability_process, process_step, stage_capability,
    value_stream, value_stream_stage, space, space_member, space_invitation,
};
use business_architecture::application::value_stream_service::ValueStreamService;
use business_architecture::application::space_service::SpaceService;
use business_architecture::domain::value_stream::entity::ValueStream as DomainValueStream;
use business_architecture::domain::value_stream::repository::ValueStreamRepository;
use business_architecture::domain::space::entity::{Space as DomainSpace, SpaceMember as DomainSpaceMember};
use business_architecture::infrastructure::persistence::value_stream_repo::SeaOrmValueStreamRepo;
use business_architecture::infrastructure::persistence::space_repo::{SeaOrmSpaceRepo, SeaOrmMembershipRepo};
use shared_common::enums::ValueStreamImportance;
use shared_common::enums::SpaceRole;
use shared_common::enums::{
    BusinessValueRating, CapabilityLevel, CapabilityStatus, CostRating,
    LifecycleStatus, MaturityLevel,
};

pub type GraphqlSchema = async_graphql::dynamic::Schema;

// ============================================================================
// GraphQL Auth Guard (seaography LifecycleHooks)
// ============================================================================

/// Entities that support ownership (have `created_by` field).
const OWNED_ENTITIES: &[&str] = &[
    "business_capabilities",
    "business_processes",
    "value_streams",
];

/// User-management entities: only Admin can manage users.
const USER_ENTITIES: &[&str] = &[
    "users",
    "refresh_tokens",
    "oauth_authorization_codes",
];

/// Entities whose membership/identity data should not be exposed to anonymous
/// readers. Reading these requires an authenticated session; user records and
/// space membership/invitation records additionally require admin (cross-tenant
/// enumeration is prevented; space-scoped reads go through custom queries that
/// enforce membership).
const PRIVATE_READ_ENTITIES: &[&str] = &[
    "users",
    "refresh_tokens",
    "oauth_authorization_codes",
    "space_members",
    "space_invitations",
];

/// Entities that require admin to read even when authenticated (prevents
/// cross-tenant enumeration of membership and invitations via the
/// auto-generated query; space-scoped reads use custom queries instead).
const ADMIN_READ_ENTITIES: &[&str] = &[
    "space_members",
    "space_invitations",
];

/// Fields hidden from all users (including Admin) in queries.
const HIDDEN_FIELDS: &[(&str, &str)] = &[
    ("users", "password_hash"),
    ("space_invitations", "token_hash"),
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
                // Anonymous users may read spaces and business architecture entities
                // (case-showcase / public read). Membership and user entities require
                // an authenticated session; user records additionally require admin.
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

                    // Inject Claims into GraphQL context if JWT was valid
                    if let Some(claims) = has_jwt {
                        request = request.data(claims);
                    } else {
                        // Fallback: reject mutation requests without JWT
                        let body_str = String::from_utf8_lossy(&bytes);
                        if body_str.contains("mutation") {
                            return Ok((
                                axum::http::StatusCode::UNAUTHORIZED,
                                axum::Json(serde_json::json!({
                                    "errors": [{"message": "Authentication required for mutations. Provide a valid JWT via Authorization header."}]
                                })),
                            )
                                .into_response());
                        }
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
    value_stream::Model {
        id: vs.id,
        logical_id: vs.logical_id,
        business_version: vs.business_version.clone(),
        status: vs.status,
        name: vs.name.clone(),
        description: vs.description.clone(),
        triggering_event: vs.triggering_event.clone(),
        end_deliverable: vs.end_deliverable.clone(),
        owner_id: vs.owner_id,
        importance: vs.importance,
        stakeholders: vs.stakeholders.clone(),
        performance_metrics: vs.performance_metrics.clone(),
        created_by: vs.created_by,
        updated_by: vs.updated_by,
        created_at: vs.created_at,
        updated_at: vs.updated_at,
        deleted_at: vs.deleted_at,
        space_id: vs.space_id,
    }
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
    let service = SpaceService::new(
        SeaOrmSpaceRepo::new(db.clone()),
        SeaOrmMembershipRepo::new(db.clone()),
    );
    service
        .ensure_can_edit(space_id, claims.user_id, claims.user_role())
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))
}

/// Register custom ValueStream mutations that go through the domain model.
/// These replace seaography's auto-generated CRUD mutations for value_stream.
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

                ensure_space_edit_access(&ctx, db, space_id).await?;

                let repo = SeaOrmValueStreamRepo::new(db.clone());
                let service = ValueStreamService::new(repo);
                let vs = service
                    .create(space_id, name, description, business_version, importance)
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
    .argument(InputValue::new("importance", TypeRef::named_nn(TypeRef::STRING)));

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

                let repo = SeaOrmValueStreamRepo::new(db.clone());
                // Enforce space-level ACL: look up the target's space before mutating.
                let existing = repo
                    .find_by_id(id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Value stream not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;

                let service = ValueStreamService::new(repo);
                let vs = service
                    .update(id, name, description, importance)
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
    .argument(InputValue::new("importance", TypeRef::named(TypeRef::STRING)));

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
                // Enforce space-level ACL before archiving.
                let existing = repo
                    .find_by_id(id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Value stream not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;

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
                // Enforce space-level ACL before creating a new version.
                let existing = repo
                    .find_by_id(current_id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    .ok_or_else(|| async_graphql::Error::new("Value stream not found."))?;
                ensure_space_edit_access(&ctx, db, existing.space_id).await?;

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
}

// ============================================================================
// Custom Space Domain Mutations
// ============================================================================

fn domain_space_to_model(s: &DomainSpace) -> space::Model {
    space::Model {
        id: s.id,
        name: s.name.clone(),
        description: s.description.clone(),
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

                let service = SpaceService::new(
                    SeaOrmSpaceRepo::new(db.clone()),
                    SeaOrmMembershipRepo::new(db.clone()),
                );
                let space_obj = service
                    .create_space(claims.user_id, claims.user_role(), name, description)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(domain_space_to_model(&space_obj))))
            })
        },
    )
    .argument(InputValue::new("name", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(create);

    // ── spaceUpdate ────────────────────────────────────────────────────
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

                let service = SpaceService::new(
                    SeaOrmSpaceRepo::new(db.clone()),
                    SeaOrmMembershipRepo::new(db.clone()),
                );
                let space_obj = service
                    .update_space(space_id, claims.user_id, claims.user_role(), name, description)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                Ok(Some(FieldValue::owned_any(domain_space_to_model(&space_obj))))
            })
        },
    )
    .argument(InputValue::new("id", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("name", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named(TypeRef::STRING)));
    builder.mutations.push(update);

    // ── spaceArchive ───────────────────────────────────────────────────
    let archive = Field::new(
        "spaceArchive",
        TypeRef::named_nn(TypeRef::BOOLEAN),
        |ctx| {
            FieldFuture::new(async move {
                let claims = require_claims(&ctx)?;
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "id")?;
                let service = SpaceService::new(
                    SeaOrmSpaceRepo::new(db.clone()),
                    SeaOrmMembershipRepo::new(db.clone()),
                );
                service
                    .archive_space(space_id, claims.user_id, claims.user_role())
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
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

                let service = SpaceService::new(
                    SeaOrmSpaceRepo::new(db.clone()),
                    SeaOrmMembershipRepo::new(db.clone()),
                );
                let member = service
                    .add_member(space_id, claims.user_id, claims.user_role(), user_id, role)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
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
                let service = SpaceService::new(
                    SeaOrmSpaceRepo::new(db.clone()),
                    SeaOrmMembershipRepo::new(db.clone()),
                );
                service
                    .remove_member(space_id, claims.user_id, claims.user_role(), user_id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
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

                ensure_space_edit_access(&ctx, db, space_id).await?;

                let now = chrono::Utc::now();
                let am = business_capability::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    logical_id: Set(Uuid::now_v7()),
                    business_version: Set("v1.0".to_owned()),
                    status: Set(LifecycleStatus::Active),
                    capability_status: Set(CapabilityStatus::Active),
                    name: Set(name),
                    description: Set(description),
                    level: Set(level),
                    maturity: Set(maturity),
                    business_value: Set(business_value),
                    cost: Set(CostRating::Low),
                    owner_id: NotSet,
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
    .argument(InputValue::new("level", TypeRef::named_nn("CapabilityLevel")))
    .argument(InputValue::new("maturity", TypeRef::named_nn("MaturityLevel")))
    .argument(InputValue::new("businessValue", TypeRef::named_nn("BusinessValueRating")));
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
    .argument(InputValue::new("level", TypeRef::named("CapabilityLevel")))
    .argument(InputValue::new("maturity", TypeRef::named("MaturityLevel")))
    .argument(InputValue::new("businessValue", TypeRef::named("BusinessValueRating")));
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
                let sla = ctx.args.get("sla").and_then(|v| v.string().ok()).map(|s| s.to_owned());
                let cycle_time: Option<i64> = ctx.args.get("cycleTime").and_then(|v| v.i64().ok());
                let cost_per_transaction: Option<f64> = ctx.args.get("costPerTransaction").and_then(|v| v.f64().ok());

                ensure_space_edit_access(&ctx, db, space_id).await?;

                let now = chrono::Utc::now();
                let am = business_process::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    logical_id: Set(Uuid::now_v7()),
                    business_version: Set("v1.0".to_owned()),
                    status: Set(LifecycleStatus::Active),
                    name: Set(name),
                    description: Set(description),
                    sla: Set(sla),
                    cost_per_transaction: Set(cost_per_transaction),
                    cycle_time: Set(cycle_time),
                    owner_id: NotSet,
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
    .argument(InputValue::new("sla", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("cycleTime", TypeRef::named(TypeRef::INT)))
    .argument(InputValue::new("costPerTransaction", TypeRef::named(TypeRef::FLOAT)));
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

                let mut am: business_process::ActiveModel = existing.into();
                if let Some(v) = ctx.args.get("name").and_then(|v| v.string().ok()) {
                    am.name = Set(v.to_owned());
                }
                if let Some(v) = ctx.args.get("description").and_then(|v| v.string().ok()) {
                    am.description = Set(v.to_owned());
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
    .argument(InputValue::new("sla", TypeRef::named(TypeRef::STRING)))
    .argument(InputValue::new("cycleTime", TypeRef::named(TypeRef::INT)))
    .argument(InputValue::new("costPerTransaction", TypeRef::named(TypeRef::FLOAT)));
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

                let now = chrono::Utc::now();
                let am = value_stream_stage::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    name: Set(name),
                    sequence_order: Set(sequence_order),
                    input: Set(input),
                    output: Set(output),
                    value_stream_id: Set(value_stream_id),
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
    .argument(InputValue::new("output", TypeRef::named(TypeRef::STRING)));
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
                let space_id = space_of_value_stream(db, existing.value_stream_id).await?;
                ensure_space_edit_access(&ctx, db, space_id).await?;

                let mut am: value_stream_stage::ActiveModel = existing.into();
                if let Some(v) = ctx.args.get("name").and_then(|v| v.string().ok()) {
                    am.name = Set(v.to_owned());
                }
                if let Some(v) = ctx.args.get("sequenceOrder").and_then(|v| v.i64().ok()).map(|v| v as i32) {
                    am.sequence_order = Set(v);
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
    .argument(InputValue::new("output", TypeRef::named(TypeRef::STRING)));
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
        TypeRef::named_nn("BusinessCapabilityProcesses"),
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
        TypeRef::named_nn("ValueStreamStageCapabilities"),
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

                let am = stage_capability::ActiveModel {
                    stage_id: Set(stage_id),
                    capability_id: Set(capability_id),
                };
                let model = am
                    .insert(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
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
    let service = SpaceService::new(
        SeaOrmSpaceRepo::new(db.clone()),
        SeaOrmMembershipRepo::new(db.clone()),
    );
    let membership = service
        .my_membership(space_id, claims.user_id)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
    if membership.is_none() {
        return Err(async_graphql::Error::new("Not a member of this space."));
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

fn register_space_scoped_queries(builder: &mut Builder) {
    use async_graphql::dynamic::{Field, FieldFuture, FieldValue, InputValue, Object, TypeRef};
    use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};

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
                let service = SpaceService::new(
                    SeaOrmSpaceRepo::new(db.clone()),
                    SeaOrmMembershipRepo::new(db.clone()),
                );
                let membership = service
                    .my_membership(space_id, claims.user_id)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
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

    // ── spaceInvitationsBySpace ──────────────────────────────────────
    let invitations_query = Field::new(
        "spaceInvitationsBySpace",
        TypeRef::named_nn_list_nn("SpaceInvitations"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;
                let space_id = parse_uuid_arg(&ctx, "spaceId")?;
                ensure_space_read_access(&ctx, db, space_id).await?;

                let models = space_invitation::Entity::find()
                    .filter(space_invitation::Column::SpaceId.eq(space_id))
                    .all(db)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                let list: Vec<FieldValue> = models
                    .into_iter()
                    .map(FieldValue::owned_any)
                    .collect();
                Ok(Some(FieldValue::list(list)))
            })
        },
    )
    .argument(InputValue::new("spaceId", TypeRef::named_nn(TypeRef::STRING)));
    builder.queries.push(invitations_query);

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

    // ── Spaces (reuses `organizations` table) + membership/invitations ──
    // Queries are public (anonymous case-showcase); writes go through custom
    // domain mutations registered below.
    register_entity::<space::Entity>(&mut builder);
    register_entity::<space_member::Entity>(&mut builder);
    register_entity::<space_invitation::Entity>(&mut builder);

    // ── Custom domain mutations for ValueStream ───────────────────────
    register_value_stream_domain_mutations(&mut builder);

    // ── Custom domain mutations for BusinessCapability/Process ───────
    register_capability_domain_mutations(&mut builder);
    register_process_domain_mutations(&mut builder);

    // ── Custom domain mutations for sub-entities (space-level ACL) ────
    register_sub_entity_domain_mutations(&mut builder);

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
        .register_entity_dataloader_one_to_one(space::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(space::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(space_member::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(space_member::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_one(space_invitation::Entity, tokio::spawn)
        .register_entity_dataloader_one_to_many(space_invitation::Entity, tokio::spawn);

    let schema = builder.schema_builder()
        .data(db.clone())
        .finish()?;

    Ok(schema)
}
