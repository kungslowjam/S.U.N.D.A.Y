//! Data connector sync and status endpoints.

use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct SyncRequest {
    #[allow(dead_code)]
    pub connector: String,
    pub force: Option<bool>,
}

/// List all available connectors and their status.
pub async fn list_handler(State(_state): State<AppState>) -> Response {
    // TODO: Query connector registry
    let body = serde_json::json!({
        "connectors": [],
        "status": "not_implemented"
    });
    (axum::http::StatusCode::NOT_IMPLEMENTED, Json(body)).into_response()
}

/// Trigger a sync for a specific connector.
pub async fn sync_handler(
    State(_state): State<AppState>,
    Path(connector): Path<String>,
    Json(req): Json<SyncRequest>,
) -> Response {
    tracing::info!("Connector sync: {} force={:?}", connector, req.force);
    // TODO: Trigger connector ingestion pipeline
    let body = serde_json::json!({
        "connector": connector,
        "status": "queued"
    });
    Json(body).into_response()
}

/// Get status of a specific connector.
pub async fn status_handler(
    State(_state): State<AppState>,
    Path(connector): Path<String>,
) -> Response {
    tracing::info!("Connector status: {}", connector);
    // TODO: Query connector state
    let body = serde_json::json!({
        "connector": connector,
        "status": "unknown"
    });
    Json(body).into_response()
}
