//! Application state for SUNDAY Desktop.

use std::sync::Arc;
use sunday_core::events::EventBus;
use sunday_engine::Engine;
use tokio::sync::RwLock;

/// Shared state for the Tauri application.
pub struct AppState {
    /// Active inference engine for the orchestrator (Hot-swappable Arc).
    pub engine: Arc<RwLock<Arc<Engine>>>,
    /// Global event bus for telemetry and mission events.
    pub bus: Arc<EventBus>,
    /// Currently active model name.
    pub model: Arc<RwLock<String>>,
}

impl AppState {
    pub fn new(engine: Engine, bus: Arc<EventBus>, model: String) -> Self {
        Self {
            engine: Arc::new(RwLock::new(Arc::new(engine))),
            bus,
            model: Arc::new(RwLock::new(model)),
        }
    }
}
