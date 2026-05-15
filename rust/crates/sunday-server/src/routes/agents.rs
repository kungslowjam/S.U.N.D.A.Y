//! Agent management endpoints.

use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::json;

use crate::state::AppState;

pub async fn list_handler(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "agents": [],
        "message": "Agent listing not yet implemented in Rust server"
    }))
}

pub async fn create_handler(
    State(_state): State<AppState>,
    Json(_body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    Json(json!({
        "status": "not_implemented",
        "message": "Agent creation not yet implemented in Rust server"
    }))
}

pub async fn delete_handler(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> Json<serde_json::Value> {
    Json(json!({
        "status": "not_implemented",
    }))
}

pub async fn message_handler(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    Json(json!({
        "status": "not_implemented",
        "message": "Agent messaging not yet implemented in Rust server"
    }))
}
