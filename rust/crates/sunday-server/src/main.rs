//! SUNDAY HTTP API Server — Axum-based replacement for Python FastAPI.

use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod memory_manager;
mod middleware;
mod routes;
mod state;

use app::create_app;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sunday_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration from default path or environment override
    let config = sunday_core::config::load_config(None).unwrap_or_default();

    let bind_host = std::env::var("SUNDAY_HOST")
        .unwrap_or_else(|_| config.server.host.clone());
    let bind_port = std::env::var("SUNDAY_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(config.server.port as u16);

    let addr: SocketAddr = format!("{}:{}", bind_host, bind_port)
        .parse()
        .expect("Invalid bind address");

    tracing::info!("🚀 SUNDAY server starting on http://{}", addr);
    tracing::info!("   Engine: {} | Model: {}", config.engine.default, config.intelligence.default_model);

    let app = create_app(config).await;

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
