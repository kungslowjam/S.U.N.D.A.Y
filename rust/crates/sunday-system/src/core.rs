//! JarvisSystem — the fully wired system struct.
//!
//! Rust translation of `src/sunday/system/core.py`.

use crate::bundles::{AgentRuntime, Observability, Scheduling, SecurityContext};
use crate::protocols::OrchestratorDeps;
use sunday_core::{EventBus, JarvisConfig};
use sunday_engine::InferenceEngine;
use std::sync::Arc;

/// Fully wired system — the single source of truth for primitive composition.
pub struct JarvisSystem {
    pub config: JarvisConfig,
    pub bus: EventBus,
    pub engine: Arc<dyn InferenceEngine>,
    pub engine_key: String,
    pub model: String,
    pub agent: Option<Arc<dyn Send + Sync>>,
    pub agent_name: String,
    pub memory_backend: Option<Arc<dyn Send + Sync>>,
    pub channel_backend: Option<Arc<dyn Send + Sync>>,
    pub telemetry_store: Option<Arc<dyn Send + Sync>>,
    pub trace_store: Option<Arc<dyn Send + Sync>>,
    pub trace_collector: Option<Arc<dyn Send + Sync>>,
    pub gpu_monitor: Option<Arc<dyn Send + Sync>>,
    pub scheduler_store: Option<Arc<dyn Send + Sync>>,
    pub scheduler: Option<Arc<dyn Send + Sync>>,
    pub session_store: Option<Arc<dyn Send + Sync>>,
    pub capability_policy: Option<Arc<dyn Send + Sync>>,
    pub audit_logger: Option<Arc<dyn Send + Sync>>,
    pub boundary_guard: Option<Arc<dyn Send + Sync>>,
    pub skill_manager: Option<Arc<dyn Send + Sync>>,
}

impl JarvisSystem {
    /// Create a minimal JarvisSystem with only the required fields.
    pub fn new(
        config: JarvisConfig,
        bus: EventBus,
        engine: Arc<dyn InferenceEngine>,
        engine_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            config,
            bus,
            engine,
            engine_key: engine_key.into(),
            model: model.into(),
            agent: None,
            agent_name: String::new(),
            memory_backend: None,
            channel_backend: None,
            telemetry_store: None,
            trace_store: None,
            trace_collector: None,
            gpu_monitor: None,
            scheduler_store: None,
            scheduler: None,
            session_store: None,
            capability_policy: None,
            audit_logger: None,
            boundary_guard: None,
            skill_manager: None,
        }
    }

    /// Security policy, audit, and boundary enforcement.
    pub fn security(&self) -> SecurityContext {
        SecurityContext {
            capability_policy: self.capability_policy.clone(),
            audit_logger: self.audit_logger.clone(),
            boundary_guard: self.boundary_guard.clone(),
        }
    }

    /// Telemetry, traces, and hardware monitoring.
    pub fn observability(&self) -> Observability {
        Observability {
            telemetry_store: self.telemetry_store.clone(),
            trace_store: self.trace_store.clone(),
            trace_collector: self.trace_collector.clone(),
            gpu_monitor: self.gpu_monitor.clone(),
        }
    }

    /// Active agent and agent lifecycle managers.
    pub fn agents(&self) -> AgentRuntime {
        AgentRuntime {
            agent: self.agent.clone(),
            agent_name: self.agent_name.clone(),
            manager: None,
            scheduler: None,
            executor: None,
        }
    }

    /// Task scheduler and its persistent store.
    pub fn scheduling(&self) -> Scheduling {
        Scheduling {
            store: self.scheduler_store.clone(),
            runner: self.scheduler.clone(),
        }
    }

    /// Execute a query through the system.
    pub fn ask(&self, query: impl Into<String>) -> Result<serde_json::Value, sunday_core::SUNDAYError> {
        let orchestrator = crate::orchestrator::QueryOrchestrator::new(self);
        orchestrator.ask(&query.into())
    }
}

impl OrchestratorDeps for JarvisSystem {
    fn config(&self) -> &JarvisConfig {
        &self.config
    }

    fn bus(&self) -> &EventBus {
        &self.bus
    }

    fn engine(&self) -> &dyn InferenceEngine {
        &*self.engine
    }

    fn engine_key(&self) -> &str {
        &self.engine_key
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn agent_name(&self) -> &str {
        &self.agent_name
    }

    fn memory_backend(&self) -> Option<&Arc<dyn Send + Sync>> {
        self.memory_backend.as_ref()
    }

    fn capability_policy(&self) -> Option<&Arc<dyn Send + Sync>> {
        self.capability_policy.as_ref()
    }

    fn session_store(&self) -> Option<&Arc<dyn Send + Sync>> {
        self.session_store.as_ref()
    }

    fn trace_store(&self) -> Option<&Arc<dyn Send + Sync>> {
        self.trace_store.as_ref()
    }

    fn trace_collector(&self) -> Option<&Arc<dyn Send + Sync>> {
        self.trace_collector.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sunday_core::{GenerateResult, Message, Usage};

    struct MockEngine;

    #[async_trait::async_trait]
    impl InferenceEngine for MockEngine {
        fn engine_id(&self) -> &str {
            "mock"
        }

        fn generate(
            &self,
            _messages: &[Message],
            _model: &str,
            _temperature: f64,
            _max_tokens: i64,
            _extra: Option<&serde_json::Value>,
        ) -> Result<GenerateResult, sunday_core::SUNDAYError> {
            Ok(GenerateResult {
                content: "Hello from mock".into(),
                usage: Usage::default(),
                ..Default::default()
            })
        }

        async fn stream(
            &self,
            _messages: &[Message],
            _model: &str,
            _temperature: f64,
            _max_tokens: i64,
            _extra: Option<&serde_json::Value>,
        ) -> Result<sunday_engine::TokenStream, sunday_core::SUNDAYError> {
            unimplemented!("mock")
        }

        fn list_models(&self) -> Result<Vec<String>, sunday_core::SUNDAYError> {
            Ok(vec![])
        }

        fn health(&self) -> bool {
            true
        }
    }

    #[test]
    fn test_jarvis_system_new() {
        let config = JarvisConfig::default();
        let bus = EventBus::new(true);
        let engine = Arc::new(MockEngine);
        let system = JarvisSystem::new(config, bus, engine, "mock", "test-model");

        assert_eq!(system.engine_key, "mock");
        assert_eq!(system.model, "test-model");
        assert!(system.agent.is_none());
        assert_eq!(system.agent_name, "");
    }

    #[test]
    fn test_security_bundle() {
        let config = JarvisConfig::default();
        let bus = EventBus::new(true);
        let engine = Arc::new(MockEngine);
        let system = JarvisSystem::new(config, bus, engine, "mock", "test-model");

        let security = system.security();
        assert!(security.capability_policy.is_none());
        assert!(security.audit_logger.is_none());
        assert!(security.boundary_guard.is_none());
    }

    #[test]
    fn test_observability_bundle() {
        let config = JarvisConfig::default();
        let bus = EventBus::new(true);
        let engine = Arc::new(MockEngine);
        let system = JarvisSystem::new(config, bus, engine, "mock", "test-model");

        let obs = system.observability();
        assert!(obs.telemetry_store.is_none());
        assert!(obs.gpu_monitor.is_none());
    }

    #[test]
    fn test_agents_bundle() {
        let config = JarvisConfig::default();
        let bus = EventBus::new(true);
        let engine = Arc::new(MockEngine);
        let system = JarvisSystem::new(config, bus, engine, "mock", "test-model");

        let agents = system.agents();
        assert!(agents.agent.is_none());
        assert_eq!(agents.agent_name, "");
    }

    #[test]
    fn test_scheduling_bundle() {
        let config = JarvisConfig::default();
        let bus = EventBus::new(true);
        let engine = Arc::new(MockEngine);
        let system = JarvisSystem::new(config, bus, engine, "mock", "test-model");

        let scheduling = system.scheduling();
        assert!(scheduling.store.is_none());
        assert!(scheduling.runner.is_none());
    }

    #[test]
    fn test_orchestrator_deps_trait() {
        let config = JarvisConfig::default();
        let bus = EventBus::new(true);
        let engine = Arc::new(MockEngine);
        let system = JarvisSystem::new(config, bus, engine, "mock", "test-model");

        // Verify the trait is implemented
        let deps: &dyn OrchestratorDeps = &system;
        assert_eq!(deps.engine_key(), "mock");
        assert_eq!(deps.model(), "test-model");
        assert_eq!(deps.agent_name(), "");
    }
}
