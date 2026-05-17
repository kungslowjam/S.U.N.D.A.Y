//! Morning digest and daily briefing endpoints.

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct DigestRequest {
    pub sources: Option<Vec<String>>,
    pub format: Option<String>,
}

/// Generate a morning digest.
pub async fn generate_handler(
    State(_state): State<AppState>,
    Json(req): Json<DigestRequest>,
) -> Response {
    tracing::info!("Digest request: sources={:?}, format={:?}", req.sources, req.format);
    // TODO: Integrate with sunday-agents morning digest agent
    let body = serde_json::json!({
        "digest": "",
        "status": "not_implemented"
    });
    (axum::http::StatusCode::NOT_IMPLEMENTED, Json(body)).into_response()
}

/// Get the last generated digest.
pub async fn latest_handler(State(_state): State<AppState>) -> Response {
    // TODO: Query digest store
    let body = serde_json::json!({
        "digest": null,
        "status": "not_implemented"
    });
    (axum::http::StatusCode::NOT_IMPLEMENTED, Json(body)).into_response()
}
