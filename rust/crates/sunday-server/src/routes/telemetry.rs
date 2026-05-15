//! Telemetry endpoints.

use axum::{
    extract::State,
    Json,
};
use serde_json::json;

use crate::state::AppState;

pub async fn stats_handler(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "not_implemented",
        "message": "Telemetry stats not yet implemented in Rust server"
    }))
}

pub async fn energy_handler(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "not_implemented",
        "message": "Energy telemetry not yet implemented in Rust server"
    }))
}
