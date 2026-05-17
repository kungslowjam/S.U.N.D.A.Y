//! Health check endpoint.

use axum::extract::State;
use axum::Json;
use serde_json::json;

use crate::state::AppState;

pub async fn handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let engine = state.engine.read().await;
    let engine_healthy = engine.health();
    let model = state.model.read().await;

    Json(json!({
        "status": if engine_healthy { "healthy" } else { "degraded" },
        "engine": engine.engine_id(),
        "model": *model,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}
