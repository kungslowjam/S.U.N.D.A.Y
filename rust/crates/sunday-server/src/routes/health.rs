//! Health check endpoint.

use axum::extract::State;
use axum::Json;
use serde_json::json;

use crate::state::AppState;

pub async fn handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let engine_healthy = state.engine.health();

    Json(json!({
        "status": if engine_healthy { "healthy" } else { "degraded" },
        "engine": state.engine.engine_id(),
        "model": state.model,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}
