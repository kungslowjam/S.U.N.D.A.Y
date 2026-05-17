//! Application state shared across all request handlers.

use std::sync::Arc;
use sunday_core::{events::EventBus, JarvisConfig};
use sunday_engine::traits::InferenceEngine;

use tokio::sync::RwLock;

use sunday_tools::executor::ToolExecutor;
use sunday_agents::AgentRuntime;

use crate::memory_manager::MemoryManager;

/// Shared state for all Axum handlers.
#[derive(Clone)]
pub struct AppState {
    /// Active inference engine (wrapped in RwLock for hot-swapping).
    pub engine: Arc<RwLock<Arc<dyn InferenceEngine>>>,
    /// Loaded configuration.
    #[allow(dead_code)]
    pub config: Arc<JarvisConfig>,
    /// Event bus for telemetry and agent events.
    pub bus: Arc<EventBus>,
    /// Active model identifier.
    pub model: Arc<RwLock<String>>,
    /// Central tool executor for built-in tools.
    pub tools: Arc<ToolExecutor>,
    /// Persistent memory (Knowledge Graph + Episodic).
    pub memory: Option<Arc<MemoryManager>>,
    /// High-performance Agent Runtime
    pub agent_runtime: Arc<AgentRuntime>,
}

impl AppState {
    pub fn new(
        engine: Arc<dyn InferenceEngine>,
        config: JarvisConfig,
        bus: Arc<EventBus>,
        model: String,
        tools: ToolExecutor,
        memory: Option<Arc<MemoryManager>>,
    ) -> Self {
        Self {
            engine: Arc::new(RwLock::new(engine)),
            config: Arc::new(config),
            bus: bus.clone(),
            model: Arc::new(RwLock::new(model)),
            tools: Arc::new(tools),
            memory,
            agent_runtime: Arc::new(AgentRuntime::new(bus)),
        }
    }
}
