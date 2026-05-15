//! Model management endpoints.

use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::json;

use crate::state::AppState;

/// List available models.
pub async fn list_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    match state.engine.list_models() {
        Ok(models) => Json(json!({ "models": models })),
        Err(e) => {
            let empty: Vec<String> = vec![];
            Json(json!({ "error": e.to_string(), "models": empty }))
        }
    }
}

/// Pull a model (placeholder — would delegate to engine).
pub async fn pull_handler(State(_state): State<AppState>, Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let model_name = body.get("model").and_then(|m| m.as_str()).unwrap_or("");
    Json(json!({
        "status": "queued",
        "model": model_name,
        "message": "Model pull is not yet implemented in Rust server"
    }))
}

/// Delete a model (placeholder).
pub async fn delete_handler(
    State(_state): State<AppState>,
    Path(model_name): Path<String>,
) -> Json<serde_json::Value> {
    Json(json!({
        "status": "not_implemented",
        "model": model_name,
    }))
}
