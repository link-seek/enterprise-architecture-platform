use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;

use super::handlers::BusinessArchitectureService;

pub fn business_architecture_routes() -> Router<Arc<BusinessArchitectureService>> {
    Router::new()
        .route(
            "/api/business-architecture/capabilities",
            post(super::handlers::create_capability)
                .get(super::handlers::list_capabilities),
        )
        .route(
            "/api/business-architecture/capabilities/{id}",
            get(super::handlers::get_capability)
                .put(super::handlers::update_capability)
                .delete(super::handlers::delete_capability),
        )
        .route(
            "/api/business-architecture/capabilities/{id}/processes",
            get(super::handlers::get_capability_processes)
                .post(super::handlers::link_capability_process),
        )
        .route(
            "/api/business-architecture/processes",
            post(super::handlers::create_process).get(super::handlers::list_processes),
        )
        .route(
            "/api/business-architecture/processes/{id}",
            get(super::handlers::get_process)
                .put(super::handlers::update_process)
                .delete(super::handlers::delete_process),
        )
        .route(
            "/api/business-architecture/processes/{id}/publish",
            post(super::handlers::publish_process_version),
        )
        .route(
            "/api/business-architecture/processes/{id}/versions",
            get(super::handlers::get_process_versions),
        )
        .route(
            "/api/business-architecture/processes/{id}/steps",
            get(super::handlers::get_process_steps).post(super::handlers::create_process_step),
        )
        .route(
            "/api/business-architecture/value-streams",
            post(super::handlers::create_value_stream)
                .get(super::handlers::list_value_streams),
        )
        .route(
            "/api/business-architecture/value-streams/{id}",
            get(super::handlers::get_value_stream)
                .put(super::handlers::update_value_stream)
                .delete(super::handlers::delete_value_stream),
        )
        .route(
            "/api/business-architecture/value-streams/{id}/stages",
            get(super::handlers::get_value_stream_stages)
                .post(super::handlers::create_value_stream_stage),
        )
        .route(
            "/api/business-architecture/value-streams/stages/{stage_id}/capabilities",
            post(super::handlers::link_stage_capability),
        )
        .route(
            "/api/business-architecture/analysis/gap",
            post(super::handlers::gap_analysis),
        )
        .route(
            "/api/business-architecture/analysis/redundancy",
            post(super::handlers::redundancy_analysis),
        )
}
