//! Bundle dataclasses that group cohesive subsystems of JarvisSystem.
//!
//! Rust translation of `src/sunday/system/bundles.py`.

use std::sync::Arc;

// ---------------------------------------------------------------------------
// SecurityContext
// ---------------------------------------------------------------------------

/// Security policy, audit, and boundary enforcement.
pub struct SecurityContext {
    pub capability_policy: Option<Arc<dyn Send + Sync>>,
    pub audit_logger: Option<Arc<dyn Send + Sync>>,
    pub boundary_guard: Option<Arc<dyn Send + Sync>>,
}

impl std::fmt::Debug for SecurityContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecurityContext")
            .field("capability_policy", &self.capability_policy.is_some())
            .field("audit_logger", &self.audit_logger.is_some())
            .field("boundary_guard", &self.boundary_guard.is_some())
            .finish()
    }
}

impl Clone for SecurityContext {
    fn clone(&self) -> Self {
        Self {
            capability_policy: self.capability_policy.clone(),
            audit_logger: self.audit_logger.clone(),
            boundary_guard: self.boundary_guard.clone(),
        }
    }
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self {
            capability_policy: None,
            audit_logger: None,
            boundary_guard: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Observability
// ---------------------------------------------------------------------------

/// Telemetry, traces, and hardware monitoring.
pub struct Observability {
    pub telemetry_store: Option<Arc<dyn Send + Sync>>,
    pub trace_store: Option<Arc<dyn Send + Sync>>,
    pub trace_collector: Option<Arc<dyn Send + Sync>>,
    pub gpu_monitor: Option<Arc<dyn Send + Sync>>,
}

impl std::fmt::Debug for Observability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Observability")
            .field("telemetry_store", &self.telemetry_store.is_some())
            .field("trace_store", &self.trace_store.is_some())
            .field("trace_collector", &self.trace_collector.is_some())
            .field("gpu_monitor", &self.gpu_monitor.is_some())
            .finish()
    }
}

impl Clone for Observability {
    fn clone(&self) -> Self {
        Self {
            telemetry_store: self.telemetry_store.clone(),
            trace_store: self.trace_store.clone(),
            trace_collector: self.trace_collector.clone(),
            gpu_monitor: self.gpu_monitor.clone(),
        }
    }
}

impl Default for Observability {
    fn default() -> Self {
        Self {
            telemetry_store: None,
            trace_store: None,
            trace_collector: None,
            gpu_monitor: None,
        }
    }
}

// ---------------------------------------------------------------------------
// AgentRuntime
// ---------------------------------------------------------------------------

/// Active agent and agent lifecycle managers.
pub struct AgentRuntime {
    pub agent: Option<Arc<dyn Send + Sync>>,
    pub agent_name: String,
    pub manager: Option<Arc<dyn Send + Sync>>,
    pub scheduler: Option<Arc<dyn Send + Sync>>,
    pub executor: Option<Arc<dyn Send + Sync>>,
}

impl std::fmt::Debug for AgentRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentRuntime")
            .field("agent", &self.agent.is_some())
            .field("agent_name", &self.agent_name)
            .field("manager", &self.manager.is_some())
            .field("scheduler", &self.scheduler.is_some())
            .field("executor", &self.executor.is_some())
            .finish()
    }
}

impl Clone for AgentRuntime {
    fn clone(&self) -> Self {
        Self {
            agent: self.agent.clone(),
            agent_name: self.agent_name.clone(),
            manager: self.manager.clone(),
            scheduler: self.scheduler.clone(),
            executor: self.executor.clone(),
        }
    }
}

impl Default for AgentRuntime {
    fn default() -> Self {
        Self {
            agent: None,
            agent_name: String::new(),
            manager: None,
            scheduler: None,
            executor: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Scheduling
// ---------------------------------------------------------------------------

/// Task scheduler and its persistent store.
pub struct Scheduling {
    pub store: Option<Arc<dyn Send + Sync>>,
    pub runner: Option<Arc<dyn Send + Sync>>,
}

impl std::fmt::Debug for Scheduling {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scheduling")
            .field("store", &self.store.is_some())
            .field("runner", &self.runner.is_some())
            .finish()
    }
}

impl Clone for Scheduling {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            runner: self.runner.clone(),
        }
    }
}

impl Default for Scheduling {
    fn default() -> Self {
        Self {
            store: None,
            runner: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_context_default() {
        let ctx = SecurityContext::default();
        assert!(ctx.capability_policy.is_none());
        assert!(ctx.audit_logger.is_none());
        assert!(ctx.boundary_guard.is_none());
    }

    #[test]
    fn test_observability_default() {
        let obs = Observability::default();
        assert!(obs.telemetry_store.is_none());
        assert!(obs.trace_store.is_none());
        assert!(obs.trace_collector.is_none());
        assert!(obs.gpu_monitor.is_none());
    }

    #[test]
    fn test_agent_runtime_default() {
        let rt = AgentRuntime::default();
        assert!(rt.agent.is_none());
        assert_eq!(rt.agent_name, "");
        assert!(rt.manager.is_none());
        assert!(rt.scheduler.is_none());
        assert!(rt.executor.is_none());
    }

    #[test]
    fn test_scheduling_default() {
        let sch = Scheduling::default();
        assert!(sch.store.is_none());
        assert!(sch.runner.is_none());
    }
}
