//! Axum middleware — CORS, auth, security headers.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use tower_http::{
    cors::{Any, CorsLayer},
};

/// CORS layer allowing frontend origins.
pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin([
            "http://localhost:5173".parse().unwrap(),
            "http://127.0.0.1:5173".parse().unwrap(),
            "tauri://localhost".parse().unwrap(),
            "http://tauri.localhost".parse().unwrap(),
            "https://tauri.localhost".parse().unwrap(),
        ])
        .allow_credentials(true)
        .allow_methods(Any)
        .allow_headers(Any)
}

/// Security headers middleware — simple tower layer.
pub fn security_headers_layer() -> tower::layer::util::Identity {
    // For now, return identity layer. Full security headers can be added via a custom middleware.
    tower::layer::util::Identity::new()
}

/// Auth middleware — validates Bearer token on protected routes.
pub async fn auth_middleware(request: Request, next: Next) -> Response {
    let path = request.uri().path();

    // Exempt health and webhooks
    if path == "/health" || path.starts_with("/webhooks/") {
        return next.run(request).await;
    }

    // Only enforce on API routes
    if path.starts_with("/v1/") || path.starts_with("/api/") {
        let api_key = std::env::var("OPENSUNDAY_API_KEY").ok();

        // If no key configured, skip auth (but warn on non-loopback)
        if api_key.is_none() {
            return next.run(request).await;
        }

        let auth_header = request
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok());

        match auth_header {
            Some(header) if header.starts_with("Bearer ") => {
                let token = &header[7..];
                if Some(token.to_string()) == api_key {
                    return next.run(request).await;
                }
            }
            _ => {}
        }

        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    next.run(request).await
}
