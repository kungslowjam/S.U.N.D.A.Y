//! SUNDAY HTTP API Server — Axum-based replacement for Python FastAPI.

use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod middleware;
mod routes;
mod state;

use app::create_app;
use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sunday_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let bind_host = std::env::var("SUNDAY_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let bind_port = std::env::var("SUNDAY_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000u16);

    let addr: SocketAddr = format!("{}:{}", bind_host, bind_port)
        .parse()
        .expect("Invalid bind address");

    tracing::info!("🚀 SUNDAY server starting on http://{}", addr);

    let app = create_app().await;

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
