//! Axum middleware — CORS, auth, security headers.

use axum::{
    extract::Request,
    http::{header, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tower_http::{
    cors::CorsLayer,
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
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::ACCEPT_LANGUAGE,
            header::CONTENT_LANGUAGE,
        ])
}

/// Security headers middleware — adds common security headers to all responses.
pub async fn security_headers_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "x-frame-options",
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        "x-xss-protection",
        HeaderValue::from_static("1; mode=block"),
    );
    headers.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    response
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
        let api_key = std::env::var("OPENSUNDAY_API_KEY")
            .ok()
            .filter(|s| !s.is_empty());

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
