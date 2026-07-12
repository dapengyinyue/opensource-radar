use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use serde_json::{json, Value};

use crate::app::AppState;
use crate::error::ApiError;

/// 手动触发一次日报推送（admin 端点，验证用）。
pub async fn trigger(
    State(s): State<AppState>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let provided = headers
        .get("X-Admin-Token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if provided != s.admin_token {
        return Err(ApiError::Unauthorized);
    }

    let sched = s
        .digest_scheduler
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("推送未配置（SERVERCHAN_SENDKEY 为空）".into()))?;

    let report = sched.run_once().await?;
    match report {
        Some(r) => Ok((
            StatusCode::OK,
            Json(json!({
                "status": "sent",
                "count": r.count,
                "title": r.title,
                "preview": r.markdown,
            })),
        )),
        None => Ok((
            StatusCode::OK,
            Json(json!({ "status": "empty", "count": 0, "message": "近 24h 无上升项目" })),
        )),
    }
}
