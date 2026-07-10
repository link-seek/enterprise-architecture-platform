use sea_orm::DatabaseConnection;
use seaography::{Builder, BuilderContext, RelatedEntityFilter, RelationBuilder};

use user_management::infrastructure::persistence::entities::{
    oauth_authorization_code, refresh_token, user,
};
use business_architecture::infrastructure::persistence::entities::{
    business_capability, business_process, capability_process, process_step, stage_capability,
    value_stream, value_stream_stage,
};

pub type GraphqlSchema = async_graphql::dynamic::Schema;

// Global storage for admin endpoint (set once at startup)
use std::sync::OnceLock;
static JWT_SECRET: OnceLock<String> = OnceLock::new();
static ADMIN_SCHEMA: OnceLock<GraphqlSchema> = OnceLock::new();

pub fn set_jwt_secret(secret: String) {
    let _ = JWT_SECRET.set(secret);
}

pub fn set_admin_schema(schema: GraphqlSchema) {
    let _ = ADMIN_SCHEMA.set(schema);
}

/// Fallback handler for /gql/* paths that axum routing can't match
/// Dispatches to playground (GET /gql/playground) or admin (POST /gql/admin)
pub async fn gql_fallback_handler(req: axum::extract::Request) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let path = req.uri().path();
    let method = req.method();

    match (method, path) {
        (&axum::http::Method::GET, "/gql/playground") => {
            let config = async_graphql::http::GraphQLPlaygroundConfig::new("/graphql");
            axum::response::Html(async_graphql::http::playground_source(config)).into_response()
        }
        (&axum::http::Method::POST, "/gql/admin") => {
            // Extract headers and body from request
            let (parts, body) = req.into_parts();
            let headers = parts.headers;
            let body_bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
                Ok(bytes) => bytes,
                Err(_) => return (StatusCode::BAD_REQUEST, axum::Json(serde_json::json!({"error": "body_too_large"}))).into_response(),
            };
            let body_str = match std::str::from_utf8(&body_bytes) {
                Ok(s) => s.to_string(),
                Err(_) => return (StatusCode::BAD_REQUEST, axum::Json(serde_json::json!({"error": "invalid_utf8"}))).into_response(),
            };
            graphql_admin_handler_inner(headers, body_str).await
        }
        _ => (StatusCode::NOT_FOUND, axum::Json(serde_json::json!({"error": "not_found"}))).into_response(),
    }
}

async fn graphql_admin_handler_inner(
    headers: axum::http::HeaderMap,
    body: String,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use jsonwebtoken::{decode, DecodingKey, Validation};

    let jwt_secret = match JWT_SECRET.get() {
        Some(s) => s,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(serde_json::json!({"error": "server_not_ready"}))).into_response(),
    };

    let schema = match ADMIN_SCHEMA.get() {
        Some(s) => s,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(serde_json::json!({"error": "server_not_ready"}))).into_response(),
    };

    // Validate JWT
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    let _claims = match auth_header {
        Some(token) => {
            let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
            validation.validate_exp = true;
            match decode::<crate::middleware::Claims>(token, &DecodingKey::from_secret(jwt_secret.as_bytes()), &validation) {
                Ok(data) => data.claims,
                Err(_) => return (StatusCode::UNAUTHORIZED, axum::Json(serde_json::json!({"error": "invalid_token"}))).into_response(),
            }
        }
        None => return (StatusCode::UNAUTHORIZED, axum::Json(serde_json::json!({"error": "missing_authorization"}))).into_response(),
    };

    // Execute GraphQL request
    let request: async_graphql::Request = match serde_json::from_str(&body) {
        Ok(req) => req,
        Err(e) => return (StatusCode::BAD_REQUEST, axum::Json(serde_json::json!({"error": format!("invalid request: {e}")}))).into_response(),
    };

    let response = schema.execute(request).await;
    axum::Json(response).into_response()
}

/// GraphQL admin handler: validates JWT, then executes GraphQL request
/// This protects mutations while /graphql remains public for queries
pub async fn graphql_admin_handler(
    headers: axum::http::HeaderMap,
    body: String,
) -> axum::response::Response {
    graphql_admin_handler_inner(headers, body).await
}

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
    let context: &'static BuilderContext = Box::leak(Box::new(BuilderContext::default()));

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
