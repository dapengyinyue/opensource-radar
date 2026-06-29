use axum::extract::{Path, State};
use axum::Json;

use crate::app::AppState;
use crate::error::ApiError;

/// 项目来源明细：GitHub repo + HN 故事列表（含讨论外链）。
pub async fn detail(
    State(s): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<storage::repo::raw::ProjectSources>, ApiError> {
    // 项目不存在则 404
    if storage::repo::project::get(&s.pool, id).await?.is_none() {
        return Err(ApiError::NotFound);
    }
    let sources = storage::repo::raw::list_project_sources(&s.pool, id).await?;
    Ok(Json(sources))
}
