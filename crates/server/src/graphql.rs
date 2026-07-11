use axum::response::IntoResponse;
use sea_orm::DatabaseConnection;
use seaography::{Builder, BuilderContext, GuardAction, LifecycleHooks, LifecycleHooksInterface, OperationType, RelatedEntityFilter, RelationBuilder};

use user_management::infrastructure::persistence::entities::{
    oauth_authorization_code, refresh_token, user,
};
use business_architecture::infrastructure::persistence::entities::{
    business_capability, business_process, capability_process, process_step, stage_capability,
    value_stream, value_stream_stage,
};

pub type GraphqlSchema = async_graphql::dynamic::Schema;

// ============================================================================
// GraphQL Auth Guard (seaography LifecycleHooks)
// ============================================================================

/// GraphQL auth guard: queries are public, mutations require JWT Claims in context.
/// Claims are injected by GraphQLService from the Authorization header.
pub struct GraphqlAuthGuard;

impl LifecycleHooksInterface for GraphqlAuthGuard {
    fn entity_guard(
        &self,
        ctx: &async_graphql::dynamic::ResolverContext,
        entity: &str,
        action: OperationType,
    ) -> GuardAction {
        let has_claims = ctx.data_opt::<crate::middleware::Claims>().is_some();
        tracing::warn!(
            "entity_guard: entity={}, action={:?}, has_claims={}",
            entity, action, has_claims
        );
        match action {
            OperationType::Read => GuardAction::Allow,
            OperationType::Create | OperationType::Update | OperationType::Delete => {
                if has_claims {
                    GuardAction::Allow
                } else {
                    GuardAction::Block(Some(
                        "Authentication required for mutations.".to_string(),
                    ))
                }
            }
        }
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
                    // This enables seaography LifecycleHooks entity_guard (if it works)
                    // AND also serves as a fallback auth check:
                    // If the request contains a mutation but no JWT, reject it.
                    if let Some(claims) = has_jwt {
                        request = request.data(claims);
                    } else {
                        // Check if request contains mutations (simple heuristic: look for "mutation" keyword)
                        // If so, reject without JWT
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

pub async fn build_graphql_schema(db: &DatabaseConnection) -> anyhow::Result<GraphqlSchema> {
    let context: &'static BuilderContext = Box::leak(Box::new(BuilderContext {
        hooks: LifecycleHooks::new(GraphqlAuthGuard),
        ..Default::default()
    }));

    let mut builder = Builder::new(context, db.clone());

    register_entity_with_mutations::<user::Entity, user::ActiveModel>(&mut builder);
    register_entity_with_mutations::<refresh_token::Entity, refresh_token::ActiveModel>(&mut builder);
    register_entity_with_mutations::<oauth_authorization_code::Entity, oauth_authorization_code::ActiveModel>(&mut builder);

    register_entity_with_mutations::<business_capability::Entity, business_capability::ActiveModel>(&mut builder);
    register_entity_with_mutations::<business_process::Entity, business_process::ActiveModel>(&mut builder);
    register_entity_with_mutations::<process_step::Entity, process_step::ActiveModel>(&mut builder);
    register_entity_with_mutations::<value_stream::Entity, value_stream::ActiveModel>(&mut builder);
    register_entity_with_mutations::<value_stream_stage::Entity, value_stream_stage::ActiveModel>(&mut builder);
    register_entity_with_mutations::<capability_process::Entity, capability_process::ActiveModel>(&mut builder);
    register_entity_with_mutations::<stage_capability::Entity, stage_capability::ActiveModel>(&mut builder);

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
