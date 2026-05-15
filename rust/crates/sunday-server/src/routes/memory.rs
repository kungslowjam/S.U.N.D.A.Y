//! Memory endpoints.

use axum::{
    extract::State,
    Json,
};
use serde_json::json;

use crate::state::AppState;

pub async fn store_handler(
    State(_state): State<AppState>,
    Json(_body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    Json(json!({
        "status": "not_implemented",
        "message": "Memory store not yet implemented in Rust server"
    }))
}

pub async fn search_handler(
    State(_state): State<AppState>,
    Json(_body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    Json(json!({
        "status": "not_implemented",
        "message": "Memory search not yet implemented in Rust server"
    }))
}

pub async fn stats_handler(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "not_implemented",
    }))
}
