//! Self-healing integration — emits failure events to the EventBus and persists traces.

use crate::runner::TestResult;
use sunday_core::events::{emit_event, EventType};
use std::path::PathBuf;

/// Triggers self-healing actions on test failures.
pub struct HealingEngine;

impl HealingEngine {
    pub fn new() -> Self {
        Self
    }

    /// Heal a failed test result.
    /// 1. Persist failure trace
    /// 2. Emit failure event to EventBus
    /// 3. Trigger distillation for critical failures
    pub fn heal(&self, result: &TestResult) {
        if result.success {
            return;
        }

        tracing::info!("Healing triggered for '{}'", result.tool_id);

        // Phase 1: Persist failure trace
        if let Err(e) = self.persist_failure_trace(result) {
            tracing::error!("Failed to persist trace: {}", e);
        }

        // Phase 2: Emit failure event
        let event_data = serde_json::json!({
            "tool_id": result.tool_id,
            "prompt": result.prompt,
            "error": result.error,
            "latency_secs": result.latency.as_secs_f64(),
            "retry_count": result.retry_count,
            "assertion_failures": result.assertion_results.iter()
                .filter(|a| !a.passed && a.assertion.required)
                .map(|a| &a.assertion.description)
                .collect::<Vec<_>>(),
        });
        emit_event(EventType::TraceComplete, event_data);

        // Phase 3: Trigger distillation for critical failures
        if result.retry_count >= 2 {
            tracing::warn!(
                "Critical failure for '{}' — triggering distillation",
                result.tool_id
            );
            self.trigger_distillation(result);
        }
    }

    fn persist_failure_trace(&self, result: &TestResult) -> Result<(), Box<dyn std::error::Error>> {
        let trace_dir = PathBuf::from("harness-stress-test/traces");
        std::fs::create_dir_all(&trace_dir)?;

        let trace_id = uuid::Uuid::new_v4().to_string();
        let trace_path = trace_dir.join(format!("{}.json", trace_id));

        let trace = serde_json::json!({
            "trace_id": trace_id,
            "tool_id": result.tool_id,
            "prompt": result.prompt,
            "outcome": "failure",
            "error": result.error,
            "latency_secs": result.latency.as_secs_f64(),
            "retry_count": result.retry_count,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "assertion_results": result.assertion_results.iter().map(|a| {
                serde_json::json!({
                    "type": format!("{:?}", a.assertion.assertion_type),
                    "passed": a.passed,
                    "message": a.message,
                })
            }).collect::<Vec<_>>(),
        });

        std::fs::write(&trace_path, serde_json::to_string_pretty(&trace)?)?;
        tracing::info!("Persisted failure trace: {:?}", trace_path);
        Ok(())
    }

    fn trigger_distillation(&self, result: &TestResult) {
        // In a full implementation, this would call into the learning system
        // to trigger policy distillation or skill evolution.
        // For now, we emit a dedicated event that the learning system can subscribe to.
        emit_event(
            EventType::SharedMemoryUpdate,
            serde_json::json!({
                "action": "trigger_distillation",
                "tool_id": result.tool_id,
                "reason": "critical_test_failure",
                "retry_count": result.retry_count,
            }),
        );
    }
}
