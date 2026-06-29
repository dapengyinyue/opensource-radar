use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use sqlx::PgPool;

use crate::api;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub collector: Arc<collector::scheduler::Collector>,
    pub admin_token: String,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(api::health::health))
        .route("/api/v1/projects", get(api::projects::list))
        .route("/api/v1/projects/:id", get(api::projects::detail))
        .route("/api/v1/projects/:id/snapshots", get(api::snapshots::list))
        .route("/api/v1/projects/:id/sources", get(api::sources::detail))
        .route("/api/v1/languages", get(api::facets::languages))
        .route("/api/v1/topics", get(api::facets::topics))
        .route("/api/v1/sources/status", get(api::facets::sources_status))
        .route("/api/v1/admin/collect/:source", post(api::admin::collect))
        .with_state(state)
}
