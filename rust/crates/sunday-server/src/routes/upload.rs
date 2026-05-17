//! File upload and ingestion endpoints.

use axum::{
    extract::{multipart::Multipart, State},
    response::{IntoResponse, Response},
    Json,
};

use crate::state::AppState;

/// Upload and ingest files into memory / knowledge base.
pub async fn ingest_handler(
    State(_state): State<AppState>,
    mut multipart: Multipart,
) -> Response {
    tracing::info!("File upload request");

    let mut files_received = 0usize;

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        if let Some(name) = field.name() {
            tracing::info!("Received field: {}", name);
            files_received += 1;
        }
        // TODO: Save to temp, chunk, and ingest into memory backend
    }

    let body = serde_json::json!({
        "files_received": files_received,
        "status": "accepted"
    });
    Json(body).into_response()
}
