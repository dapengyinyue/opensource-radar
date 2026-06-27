use axum::extract::{Path, State};
use axum::Json;

use crate::app::AppState;
use crate::error::ApiError;

pub async fn list(
    State(s): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<storage::repo::snapshot::Snapshot>>, ApiError> {
    let rows = storage::repo::snapshot::list_snapshots(&s.pool, id, 200).await?;
    Ok(Json(rows))
}
