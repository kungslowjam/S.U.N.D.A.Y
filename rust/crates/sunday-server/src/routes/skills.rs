//! Skills endpoints.

use axum::{
    extract::State,
    Json,
};
use serde_json::json;

use crate::state::AppState;

#[allow(dead_code)]
pub async fn list_handler(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "skills": [],
        "message": "Skills listing not yet implemented in Rust server"
    }))
}
