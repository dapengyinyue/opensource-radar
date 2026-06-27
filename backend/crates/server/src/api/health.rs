use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::app::AppState;

pub async fn health(State(state): State<AppState>) -> Json<Value> {
    let db_ok = storage::pool::ping(&state.pool).await.is_ok();
    Json(json!({
        "status": if db_ok { "ok" } else { "degraded" },
        "db": if db_ok { "ok" } else { "down" },
    }))
}
