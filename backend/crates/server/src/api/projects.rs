use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::app::AppState;
use crate::error::ApiError;
use storage::repo::project::{self, ProjectFilter, Sort};

#[derive(Deserialize)]
pub struct ListParams {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    pub language: Option<String>,
    pub topic: Option<String>,
    pub source: Option<String>,
    pub q: Option<String>,
    #[serde(default)]
    pub sort: String,
    #[serde(default)]
    pub since: String,
}

fn default_page() -> i64 {
    1
}
fn default_per_page() -> i64 {
    20
}

impl ListParams {
    fn to_filter(&self) -> ProjectFilter {
        let since = match self.since.as_str() {
            "7d" => Some(chrono::Utc::now() - chrono::Duration::days(7)),
            "30d" => Some(chrono::Utc::now() - chrono::Duration::days(30)),
            _ => None,
        };
        ProjectFilter {
            language: self.language.clone(),
            topic: self.topic.clone(),
            source: self.source.clone(),
            q: self.q.clone(),
            sort: Sort::parse(&self.sort),
            since,
            first_seen_since: None,
            page: self.page,
            per_page: self.per_page,
        }
    }
}

#[derive(Serialize)]
pub struct ListResponse {
    pub data: Vec<project::Project>,
    pub page: i64,
    pub per_page: i64,
    pub total: i64,
}

pub async fn list(
    State(s): State<AppState>,
    Query(p): Query<ListParams>,
) -> Result<Json<ListResponse>, ApiError> {
    let f = p.to_filter();
    let page = f.page;
    let per_page = f.per_page;
    let data = project::list(&s.pool, &f).await?;
    let total = project::count(&s.pool, &f).await?;
    Ok(Json(ListResponse {
        data,
        page,
        per_page,
        total,
    }))
}

pub async fn detail(
    State(s): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<project::Project>, ApiError> {
    let p = project::get(&s.pool, id).await?.ok_or(ApiError::NotFound)?;
    Ok(Json(p))
}
