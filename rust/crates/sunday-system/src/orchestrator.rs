//! Executes user queries through the engine or through an agent.
//!
//! Rust translation of `src/sunday/system/orchestrator.py`.

use crate::protocols::OrchestratorDeps;
use sunday_core::Message;
use serde_json;

/// Query orchestrator — executes queries via engine or agent.
pub struct QueryOrchestrator<'a> {
    system: &'a dyn OrchestratorDeps,
}

impl<'a> QueryOrchestrator<'a> {
    /// Create a new orchestrator bound to the given system.
    pub fn new(system: &'a dyn OrchestratorDeps) -> Self {
        Self { system }
    }

    /// Execute a query and return a result dict.
    pub fn ask(&self, query: &str) -> Result<serde_json::Value, sunday_core::SUNDAYError> {
        let s = self.system;
        let temperature = s.config().intelligence.temperature;
        let max_tokens = s.config().intelligence.max_tokens;

        let messages = vec![Message::user(query)];

        // If an agent is configured, we would delegate to it here.
        // For the skeleton, we call the engine directly.
        let agent_name = s.agent_name();
        if !agent_name.is_empty() && agent_name != "none" {
            // Agent path — simplified for skeleton
            // Full implementation would look up agent in registry,
            // build tools, construct AgentContext, etc.
            return self.run_agent(query, &messages, agent_name, temperature, max_tokens);
        }

        // Direct engine path
        let result = s.engine().generate(
            &messages,
            s.model(),
            temperature,
            max_tokens,
            None,
        )?;

        Ok(serde_json::json!({
            "content": result.content,
            "usage": result.usage,
            "model": result.model,
            "finish_reason": result.finish_reason,
        }))
    }

    /// Run query through an agent.
    fn run_agent(
        &self,
        _query: &str,
        messages: &[Message],
        agent_name: &str,
        temperature: f64,
        max_tokens: i64,
    ) -> Result<serde_json::Value, sunday_core::SUNDAYError> {
        // Skeleton: just call the engine directly with agent context
        // Full implementation would:
        // 1. Look up agent class in AgentRegistry
        // 2. Build agent-specific tools
        // 3. Construct AgentContext
        // 4. Run agent loop
        let s = self.system;
        let result = s.engine().generate(
            messages,
            s.model(),
            temperature,
            max_tokens,
            None,
        )?;

        Ok(serde_json::json!({
            "content": result.content,
            "usage": result.usage,
            "model": result.model,
            "agent": agent_name,
            "finish_reason": result.finish_reason,
        }))
    }

    /// Detect agent intent from query string.
    ///
    /// Returns the agent name if intent is detected, None otherwise.
    pub fn detect_agent_intent(&self, query: &str) -> Option<&'static str> {
        let lower = query.to_lowercase();
        if lower.contains("morning digest") || lower.contains("daily summary") {
            Some("morning_digest")
        } else if lower.contains("deep research") {
            Some("deep_research")
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sunday_core::{EventBus, GenerateResult, JarvisConfig, Message, Usage};
    use sunday_engine::InferenceEngine;
    use std::sync::Arc;

    struct MockEngine {
        response: String,
    }

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
                content: self.response.clone(),
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

    struct FakeSystem {
        config: JarvisConfig,
        bus: EventBus,
        engine: Arc<MockEngine>,
        agent_name: String,
    }

    impl OrchestratorDeps for FakeSystem {
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
            "mock"
        }

        fn model(&self) -> &str {
            "test-model"
        }

        fn agent_name(&self) -> &str {
            &self.agent_name
        }

        fn memory_backend(&self) -> Option<&Arc<dyn Send + Sync>> {
            None
        }

        fn capability_policy(&self) -> Option<&Arc<dyn Send + Sync>> {
            None
        }

        fn session_store(&self) -> Option<&Arc<dyn Send + Sync>> {
            None
        }

        fn trace_store(&self) -> Option<&Arc<dyn Send + Sync>> {
            None
        }

        fn trace_collector(&self) -> Option<&Arc<dyn Send + Sync>> {
            None
        }
    }

    #[test]
    fn test_orchestrator_ask_direct_engine() {
        let system = FakeSystem {
            config: JarvisConfig::default(),
            bus: EventBus::new(true),
            engine: Arc::new(MockEngine { response: "Hello!".into() }),
            agent_name: "".into(),
        };

        let orchestrator = QueryOrchestrator::new(&system);
        let result = orchestrator.ask("Say hello").unwrap();

        assert_eq!(result["content"], "Hello!");
    }

    #[test]
    fn test_orchestrator_ask_with_agent() {
        let system = FakeSystem {
            config: JarvisConfig::default(),
            bus: EventBus::new(true),
            engine: Arc::new(MockEngine { response: "Agent response".into() }),
            agent_name: "simple".into(),
        };

        let orchestrator = QueryOrchestrator::new(&system);
        let result = orchestrator.ask("Do something").unwrap();

        assert_eq!(result["content"], "Agent response");
        assert_eq!(result["agent"], "simple");
    }

    #[test]
    fn test_detect_agent_intent_morning_digest() {
        let system = FakeSystem {
            config: JarvisConfig::default(),
            bus: EventBus::new(true),
            engine: Arc::new(MockEngine { response: "".into() }),
            agent_name: "".into(),
        };
        let orchestrator = QueryOrchestrator::new(&system);

        assert_eq!(
            orchestrator.detect_agent_intent("Give me my morning digest"),
            Some("morning_digest")
        );
        assert_eq!(
            orchestrator.detect_agent_intent("Daily summary please"),
            Some("morning_digest")
        );
    }

    #[test]
    fn test_detect_agent_intent_deep_research() {
        let system = FakeSystem {
            config: JarvisConfig::default(),
            bus: EventBus::new(true),
            engine: Arc::new(MockEngine { response: "".into() }),
            agent_name: "".into(),
        };
        let orchestrator = QueryOrchestrator::new(&system);

        assert_eq!(
            orchestrator.detect_agent_intent("Do deep research on AI"),
            Some("deep_research")
        );
    }

    #[test]
    fn test_detect_agent_intent_none() {
        let system = FakeSystem {
            config: JarvisConfig::default(),
            bus: EventBus::new(true),
            engine: Arc::new(MockEngine { response: "".into() }),
            agent_name: "".into(),
        };
        let orchestrator = QueryOrchestrator::new(&system);

        assert_eq!(
            orchestrator.detect_agent_intent("What is 2+2?"),
            None
        );
    }
}
