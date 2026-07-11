use axum::extract::State;
use axum::Json;

use crate::app::AppState;
use crate::error::ApiError;
use storage::repo::facets::{self, Facet, SourceStatus};

pub async fn languages(State(s): State<AppState>) -> Result<Json<Vec<Facet>>, ApiError> {
    Ok(Json(facets::languages(&s.pool, 50).await?))
}

pub async fn topics(State(s): State<AppState>) -> Result<Json<Vec<Facet>>, ApiError> {
    Ok(Json(facets::topics(&s.pool, 50).await?))
}

pub async fn sources_status(
    State(s): State<AppState>,
) -> Result<Json<Vec<SourceStatus>>, ApiError> {
    Ok(Json(facets::sources_status(&s.pool).await?))
}
