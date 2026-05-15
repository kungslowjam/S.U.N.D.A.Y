//! Config-driven fluent builder that wires up a JarvisSystem.
//!
//! Rust translation of `src/sunday/system/builder.py`.
//! This is a **simplified skeleton** — full subsystem wiring requires
//! porting many additional crates first.

use crate::core::JarvisSystem;
use sunday_core::{EventBus, JarvisConfig};
use sunday_engine::InferenceEngine;
use std::sync::Arc;

/// Config-driven fluent builder for JarvisSystem.
///
/// # Example
/// ```
/// use sunday_system::SystemBuilder;
///
/// let system = SystemBuilder::new()
///     .engine("ollama")
///     .model("llama3")
///     .build();
/// ```
pub struct SystemBuilder {
    config: JarvisConfig,
    engine_key: Option<String>,
    model: Option<String>,
    agent_name: Option<String>,
    telemetry: Option<bool>,
    traces: Option<bool>,
    sandbox: Option<bool>,
    scheduler: Option<bool>,
    workflow: Option<bool>,
    sessions: Option<bool>,
    speech: Option<bool>,
    bus: Option<EventBus>,
}

impl Default for SystemBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemBuilder {
    /// Create a new builder with default config.
    pub fn new() -> Self {
        Self {
            config: JarvisConfig::default(),
            engine_key: None,
            model: None,
            agent_name: None,
            telemetry: None,
            traces: None,
            sandbox: None,
            scheduler: None,
            workflow: None,
            sessions: None,
            speech: None,
            bus: None,
        }
    }

    /// Create a new builder with the given config.
    pub fn with_config(config: JarvisConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Override the inference engine key.
    pub fn engine(mut self, key: impl Into<String>) -> Self {
        self.engine_key = Some(key.into());
        self
    }

    /// Override the model name.
    pub fn model(mut self, name: impl Into<String>) -> Self {
        self.model = Some(name.into());
        self
    }

    /// Override the default agent.
    pub fn agent(mut self, name: impl Into<String>) -> Self {
        self.agent_name = Some(name.into());
        self
    }

    /// Toggle telemetry.
    pub fn telemetry(mut self, enabled: bool) -> Self {
        self.telemetry = Some(enabled);
        self
    }

    /// Toggle traces.
    pub fn traces(mut self, enabled: bool) -> Self {
        self.traces = Some(enabled);
        self
    }

    /// Toggle sandbox.
    pub fn sandbox(mut self, enabled: bool) -> Self {
        self.sandbox = Some(enabled);
        self
    }

    /// Toggle scheduler.
    pub fn scheduler(mut self, enabled: bool) -> Self {
        self.scheduler = Some(enabled);
        self
    }

    /// Toggle workflow engine.
    pub fn workflow(mut self, enabled: bool) -> Self {
        self.workflow = Some(enabled);
        self
    }

    /// Toggle session store.
    pub fn sessions(mut self, enabled: bool) -> Self {
        self.sessions = Some(enabled);
        self
    }

    /// Toggle speech backend.
    pub fn speech(mut self, enabled: bool) -> Self {
        self.speech = Some(enabled);
        self
    }

    /// Inject an existing event bus.
    pub fn event_bus(mut self, bus: EventBus) -> Self {
        self.bus = Some(bus);
        self
    }

    /// Build the JarvisSystem.
    ///
    /// **Note:** This is a simplified build that creates the system shell.
    /// Full subsystem wiring (tools, memory, channels, security, etc.)
    /// requires additional crates to be ported first.
    pub fn build(self) -> Result<JarvisSystem, sunday_core::SUNDAYError> {
        let bus = self.bus.unwrap_or_else(|| EventBus::new(true));

        let engine_key = self.engine_key.unwrap_or_else(|| {
            self.config.engine.default.clone()
        });

        let model = self.model.unwrap_or_else(|| {
            self.config.intelligence.default_model.clone()
        });

        let agent_name = self.agent_name.unwrap_or_else(|| {
            self.config.agent.default_agent.clone()
        });

        // Resolve real engine
        let engine: Arc<dyn InferenceEngine> = match engine_key.as_str() {
            "ollama" => Arc::new(sunday_engine::OllamaEngine::with_defaults()),
            "llamacpp" => Arc::new(sunday_engine::LlamaCppEngine::with_defaults()),
            _ => Arc::new(PlaceholderEngine {
                engine_id: engine_key.clone(),
            }),
        };

        let mut system = JarvisSystem::new(self.config, bus, engine, &engine_key, &model);
        system.agent_name = agent_name;

        Ok(system)
    }
}

// Placeholder engine for skeleton build.
struct PlaceholderEngine {
    engine_id: String,
}

#[async_trait::async_trait]
impl InferenceEngine for PlaceholderEngine {
    fn engine_id(&self) -> &str {
        &self.engine_id
    }

    fn generate(
        &self,
        _messages: &[sunday_core::Message],
        _model: &str,
        _temperature: f64,
        _max_tokens: i64,
        _extra: Option<&serde_json::Value>,
    ) -> Result<sunday_core::GenerateResult, sunday_core::SUNDAYError> {
        Err(sunday_core::SUNDAYError::Engine(
            sunday_core::EngineError::Connection(
                "PlaceholderEngine cannot generate".into()
            )
        ))
    }

    async fn stream(
        &self,
        _messages: &[sunday_core::Message],
        _model: &str,
        _temperature: f64,
        _max_tokens: i64,
        _extra: Option<&serde_json::Value>,
    ) -> Result<sunday_engine::TokenStream, sunday_core::SUNDAYError> {
        Err(sunday_core::SUNDAYError::Engine(
            sunday_core::EngineError::Streaming(
                "PlaceholderEngine cannot stream".into()
            )
        ))
    }

    fn list_models(&self) -> Result<Vec<String>, sunday_core::SUNDAYError> {
        Ok(vec![])
    }

    fn health(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_fluent_api() {
        let system = SystemBuilder::new()
            .engine("ollama")
            .model("llama3")
            .agent("orchestrator")
            .telemetry(true)
            .traces(true)
            .build()
            .unwrap();

        assert_eq!(system.engine_key, "ollama");
        assert_eq!(system.model, "llama3");
        assert_eq!(system.agent_name, "orchestrator");
    }

    #[test]
    fn test_builder_defaults() {
        let system = SystemBuilder::new().build().unwrap();

        // Should use defaults from JarvisConfig (may be empty strings)
        // Just verify the system was built successfully
        assert_eq!(system.engine_key, system.config.engine.default);
        assert_eq!(system.model, system.config.intelligence.default_model);
    }

    #[test]
    fn test_builder_with_config() {
        let mut config = JarvisConfig::default();
        config.intelligence.default_model = "custom-model".into();

        let system = SystemBuilder::with_config(config)
            .engine("vllm")
            .build()
            .unwrap();

        assert_eq!(system.engine_key, "vllm");
        assert_eq!(system.model, "custom-model");
    }

    #[test]
    fn test_builder_event_bus() {
        let bus = EventBus::new(256);
        let system = SystemBuilder::new()
            .event_bus(bus)
            .build()
            .unwrap();

        // System should have the injected bus
        // Verify the bus was injected by checking we can subscribe
        let _receiver = system.bus.subscribe();
    }
}
