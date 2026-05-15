//! Session endpoints.

use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::json;

use crate::state::AppState;

pub async fn list_handler(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "sessions": [],
        "message": "Session listing not yet implemented in Rust server"
    }))
}

pub async fn get_handler(
    State(_state): State<AppState>,
    Path(_session_id): Path<String>,
) -> Json<serde_json::Value> {
    Json(json!({
        "status": "not_implemented",
    }))
}
