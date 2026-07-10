use axum::{routing::get, routing::post, Router};

use crate::state::AppState;

pub fn ai_routes() -> Router<AppState> {
    Router::new()
        .route("/suggest", post(crate::ai::handlers::suggest_handler))
        .route("/stream", get(crate::ai::handlers::stream_handler))
}
