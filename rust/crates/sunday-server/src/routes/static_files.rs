//! Static file serving and SPA catch-all.

use axum::{
    extract::State,
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
};
use std::path::PathBuf;

use crate::state::AppState;

/// Serve static files or fallback to index.html for SPA routing.
pub async fn handler(uri: Uri, State(_state): State<AppState>) -> Response {
    let static_dir = PathBuf::from("frontend/dist");
    let path = uri.path().trim_start_matches('/');

    // Try to serve the actual file
    let file_path = static_dir.join(path);
    if file_path.is_file() {
        return serve_file(&file_path).await;
    }

    // Fallback to index.html for SPA routes
    let index_path = static_dir.join("index.html");
    if index_path.is_file() {
        return serve_file(&index_path).await;
    }

    (StatusCode::NOT_FOUND, "Not found").into_response()
}

async fn serve_file(path: &PathBuf) -> Response {
    match tokio::fs::read(path).await {
        Ok(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let mut headers = axum::http::HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_str(mime.as_ref()).unwrap(),
            );
            // No-cache for assets
            if path.to_string_lossy().contains("assets/") {
                headers.insert(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("no-cache, no-store, must-revalidate"),
                );
            }
            (headers, content).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}
