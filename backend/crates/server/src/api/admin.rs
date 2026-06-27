use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use domain::models::SourceKind;
use serde_json::{json, Value};

use crate::app::AppState;
use crate::error::ApiError;

pub async fn collect(
    State(s): State<AppState>,
    Path(source): Path<String>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let provided = headers
        .get("X-Admin-Token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if provided != s.admin_token {
        return Err(ApiError::Unauthorized);
    }

    let kind = match source.as_str() {
        "github" => SourceKind::Github,
        "hackernews" => SourceKind::Hackernews,
        other => return Err(ApiError::BadRequest(format!("unknown source: {other}"))),
    };

    // fire-and-forget：后台执行采集，立即返回 202
    let collector = s.collector.clone();
    let label = kind.as_str().to_string();
    tokio::spawn(async move {
        match collector.run_once(kind).await {
            Ok(n) => tracing::info!(source = %label, persisted = n, "manual collect done"),
            Err(e) => tracing::warn!(source = %label, error = %e, "manual collect failed"),
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "status": "accepted", "source": source })),
    ))
}
