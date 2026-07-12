use axum::response::IntoResponse;
use sea_orm::DatabaseConnection;
use seaography::{Builder, BuilderContext, GuardAction, LifecycleHooks, LifecycleHooksInterface, OperationType, RelatedEntityFilter, RelationBuilder, TimeLibrary, TypesMapConfig};
use uuid::Uuid;

use user_management::infrastructure::persistence::entities::{
    oauth_authorization_code, refresh_token, user,
};
use business_architecture::infrastructure::persistence::entities::{
    business_capability, business_process, capability_process, process_step, stage_capability,
    value_stream, value_stream_stage,
};
use business_architecture::application::value_stream_service::ValueStreamService;
use business_architecture::domain::value_stream::entity::ValueStream as DomainValueStream;
use business_architecture::infrastructure::persistence::value_stream_repo::SeaOrmValueStreamRepo;
use shared_common::enums::ValueStreamImportance;

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

/// Fields hidden from all users (including Admin) in queries.
const HIDDEN_FIELDS: &[(&str, &str)] = &[
    ("users", "password_hash"),
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
            OperationType::Read => GuardAction::Allow,

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
    }
}

// ============================================================================
// Custom ValueStream Domain Mutations
// ============================================================================

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
                let db = ctx.data::<DatabaseConnection>()?;

                let name = ctx.args.try_get("name")?.string()?.to_owned();
                let description = ctx.args.try_get("description")?.string()?.to_owned();
                let business_version = ctx.args.try_get("businessVersion")?.string()?.to_owned();
                let importance = parse_importance(ctx.args.try_get("importance")?.enum_name()?)?;

                let repo = SeaOrmValueStreamRepo::new(db.clone());
                let service = ValueStreamService::new(repo);
                let vs = service
                    .create(name, description, business_version, importance)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;

                let model = domain_vs_to_model(&vs);
                Ok(Some(FieldValue::owned_any(model)))
            })
        },
    )
    .argument(InputValue::new("name", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("description", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("businessVersion", TypeRef::named_nn(TypeRef::STRING)))
    .argument(InputValue::new("importance", TypeRef::named_nn(TypeRef::STRING)));

    builder.mutations.push(create_field);

    // ── valueStreamUpdate ──────────────────────────────────────────────
    let update_field = Field::new(
        "valueStreamUpdate",
        TypeRef::named_nn("ValueStreams"),
        |ctx| {
            FieldFuture::new(async move {
                let db = ctx.data::<DatabaseConnection>()?;

                let id_str = ctx.args.try_get("id")?.string()?;
                let id = Uuid::parse_str(id_str)
                    .map_err(|e| async_graphql::Error::new(format!("Invalid UUID: {e}")))?;

                let name = ctx.args.get("name").and_then(|v| v.string().ok()).map(|s| s.to_owned());
                let description = ctx.args.get("description").and_then(|v| v.string().ok()).map(|s| s.to_owned());
                let importance = match ctx.args.get("importance") {
                    Some(v) if !v.is_null() => Some(parse_importance(v.enum_name()?)?),
                    _ => None,
                };

                let repo = SeaOrmValueStreamRepo::new(db.clone());
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
                let db = ctx.data::<DatabaseConnection>()?;

                let id_str = ctx.args.try_get("id")?.string()?;
                let id = Uuid::parse_str(id_str)
                    .map_err(|e| async_graphql::Error::new(format!("Invalid UUID: {e}")))?;

                let repo = SeaOrmValueStreamRepo::new(db.clone());
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
                let db = ctx.data::<DatabaseConnection>()?;

                let current_id_str = ctx.args.try_get("currentId")?.string()?;
                let current_id = Uuid::parse_str(current_id_str)
                    .map_err(|e| async_graphql::Error::new(format!("Invalid UUID: {e}")))?;

                let new_version = ctx.args.try_get("newVersion")?.string()?.to_owned();
                let new_name = ctx.args.get("newName").and_then(|v| v.string().ok()).map(|s| s.to_owned());
                let new_description = ctx.args.get("newDescription").and_then(|v| v.string().ok()).map(|s| s.to_owned());

                let repo = SeaOrmValueStreamRepo::new(db.clone());
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
    // Mutations for value_stream go through the domain model (DDD)
    register_entity_with_mutations::<business_capability::Entity, business_capability::ActiveModel>(&mut builder);
    register_entity_with_mutations::<business_process::Entity, business_process::ActiveModel>(&mut builder);
    register_entity_with_mutations::<process_step::Entity, process_step::ActiveModel>(&mut builder);
    register_entity::<value_stream::Entity>(&mut builder);  // queries only
    register_entity_with_mutations::<value_stream_stage::Entity, value_stream_stage::ActiveModel>(&mut builder);
    register_entity_with_mutations::<capability_process::Entity, capability_process::ActiveModel>(&mut builder);
    register_entity_with_mutations::<stage_capability::Entity, stage_capability::ActiveModel>(&mut builder);

    // ── Custom domain mutations for ValueStream ───────────────────────
    register_value_stream_domain_mutations(&mut builder);

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
        .register_entity_dataloader_one_to_many(stage_capability::Entity, tokio::spawn);

    let schema = builder.schema_builder()
        .data(db.clone())
        .finish()?;

    Ok(schema)
}
