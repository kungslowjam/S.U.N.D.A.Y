//! Application state shared across all request handlers.

use std::sync::Arc;
use sunday_core::{events::EventBus, JarvisConfig};
use sunday_engine::traits::InferenceEngine;

/// Shared state for all Axum handlers.
#[derive(Clone)]
pub struct AppState {
    /// Active inference engine (wrapped MultiEngine).
    pub engine: Arc<dyn InferenceEngine>,
    /// Loaded configuration.
    pub config: Arc<JarvisConfig>,
    /// Event bus for telemetry and agent events.
    pub bus: Arc<EventBus>,
    /// Default model name.
    pub model: String,
}

impl AppState {
    pub fn new(
        engine: Arc<dyn InferenceEngine>,
        config: JarvisConfig,
        bus: Arc<EventBus>,
        model: String,
    ) -> Self {
        Self {
            engine,
            config: Arc::new(config),
            bus,
            model,
        }
    }
}
