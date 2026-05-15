//! App factory — creates the Axum router with all routes and middleware.

use crate::middleware::{cors_layer, security_headers_layer};
use crate::routes::{
    agents, chat, health, memory, models, sessions, skills, static_files, telemetry, traces, ws,
};
use crate::state::AppState;
use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use sunday_core::{events::EventBus, JarvisConfig};
use sunday_engine::ollama::OllamaEngine;
use tower_http::trace::TraceLayer;

/// Create the full Axum application router.
pub async fn create_app() -> Router {
    // TODO: Load real config and discover engines
    let config = JarvisConfig::default();
    let bus = Arc::new(EventBus::new(true));
    let engine = Arc::new(OllamaEngine::new("http://127.0.0.1:11434", 30.0));
    let model = "qwen3.5:latest".to_string();

    let state = AppState::new(engine, config, bus, model);

    Router::new()
        // Health
        .route("/health", get(health::handler))
        // OpenAI-compatible chat
        .route("/v1/chat/completions", post(chat::completions_handler))
        // Models
        .route("/v1/models", get(models::list_handler))
        .route("/v1/models/pull", post(models::pull_handler))
        .route("/v1/models/{model_name}", delete(models::delete_handler))
        // Agents
        .route("/v1/agents", get(agents::list_handler).post(agents::create_handler))
        .route("/v1/agents/{id}", delete(agents::delete_handler))
        .route("/v1/agents/{id}/message", post(agents::message_handler))
        // Memory
        .route("/v1/memory/store", post(memory::store_handler))
        .route("/v1/memory/search", post(memory::search_handler))
        .route("/v1/memory/stats", get(memory::stats_handler))
        // Telemetry
        .route("/v1/telemetry/stats", get(telemetry::stats_handler))
        .route("/v1/telemetry/energy", get(telemetry::energy_handler))
        // Traces
        .route("/v1/traces", get(traces::list_handler))
        .route("/v1/traces/{trace_id}", get(traces::get_handler))
        // Skills
        .route("/v1/skills", get(skills::list_handler))
        // Sessions
        .route("/v1/sessions", get(sessions::list_handler))
        .route("/v1/sessions/{session_id}", get(sessions::get_handler))
        // WebSocket
        .route("/v1/chat/stream", get(ws::chat_stream_handler))
        .route("/v1/agents/events", get(ws::agent_events_handler))
        // Static files + SPA catch-all
        .fallback(static_files::handler)
        // Middleware
        .layer(cors_layer())
        .layer(security_headers_layer())
        .layer(axum::middleware::from_fn(crate::middleware::auth_middleware))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
