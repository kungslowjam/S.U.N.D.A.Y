//! PyO3 bindings for sunday-system — system composition layer.

use pyo3::prelude::*;

// ---------------------------------------------------------------------------
// Bundle types
// ---------------------------------------------------------------------------

#[pyclass(name = "SecurityContext")]
#[derive(Clone, Debug)]
pub struct PySecurityContext {
    #[pyo3(get)]
    pub has_capability_policy: bool,
    #[pyo3(get)]
    pub has_audit_logger: bool,
    #[pyo3(get)]
    pub has_boundary_guard: bool,
}

#[pymethods]
impl PySecurityContext {
    #[new]
    fn new() -> Self {
        Self {
            has_capability_policy: false,
            has_audit_logger: false,
            has_boundary_guard: false,
        }
    }

    fn __repr__(&self) -> String {
        format!("SecurityContext(cap={}, audit={}, guard={})",
            self.has_capability_policy, self.has_audit_logger, self.has_boundary_guard)
    }
}

#[pyclass(name = "Observability")]
#[derive(Clone, Debug)]
pub struct PyObservability {
    #[pyo3(get)]
    pub has_telemetry_store: bool,
    #[pyo3(get)]
    pub has_trace_store: bool,
    #[pyo3(get)]
    pub has_trace_collector: bool,
    #[pyo3(get)]
    pub has_gpu_monitor: bool,
}

#[pymethods]
impl PyObservability {
    #[new]
    fn new() -> Self {
        Self {
            has_telemetry_store: false,
            has_trace_store: false,
            has_trace_collector: false,
            has_gpu_monitor: false,
        }
    }

    fn __repr__(&self) -> String {
        format!("Observability(telemetry={}, trace={}, collector={}, gpu={})",
            self.has_telemetry_store, self.has_trace_store, self.has_trace_collector, self.has_gpu_monitor)
    }
}

#[pyclass(name = "AgentRuntime")]
#[derive(Clone, Debug)]
pub struct PyAgentRuntime {
    #[pyo3(get)]
    pub has_agent: bool,
    #[pyo3(get)]
    pub agent_name: String,
    #[pyo3(get)]
    pub has_manager: bool,
    #[pyo3(get)]
    pub has_scheduler: bool,
    #[pyo3(get)]
    pub has_executor: bool,
}

#[pymethods]
impl PyAgentRuntime {
    #[new]
    fn new() -> Self {
        Self {
            has_agent: false,
            agent_name: String::new(),
            has_manager: false,
            has_scheduler: false,
            has_executor: false,
        }
    }

    fn __repr__(&self) -> String {
        format!("AgentRuntime(agent={}, name='{}')", self.has_agent, self.agent_name)
    }
}

#[pyclass(name = "Scheduling")]
#[derive(Clone, Debug)]
pub struct PyScheduling {
    #[pyo3(get)]
    pub has_store: bool,
    #[pyo3(get)]
    pub has_runner: bool,
}

#[pymethods]
impl PyScheduling {
    #[new]
    fn new() -> Self {
        Self {
            has_store: false,
            has_runner: false,
        }
    }

    fn __repr__(&self) -> String {
        format!("Scheduling(store={}, runner={})", self.has_store, self.has_runner)
    }
}

// ---------------------------------------------------------------------------
// JarvisSystem
// ---------------------------------------------------------------------------

#[pyclass(name = "JarvisSystem")]
pub struct PyJarvisSystem;

#[pymethods]
impl PyJarvisSystem {
    #[new]
    fn new() -> Self {
        Self
    }

    /// Create a system from config (skeleton).
    #[staticmethod]
    fn from_config(config: &super::core::PyConfig) -> PyResult<Self> {
        // For the skeleton, we just return a placeholder
        // Full implementation would use SystemBuilder
        Ok(Self)
    }

    fn security(&self) -> PySecurityContext {
        PySecurityContext::new()
    }

    fn observability(&self) -> PyObservability {
        PyObservability::new()
    }

    fn agents(&self) -> PyAgentRuntime {
        PyAgentRuntime::new()
    }

    fn scheduling(&self) -> PyScheduling {
        PyScheduling::new()
    }

    fn __repr__(&self) -> String {
        "JarvisSystem()".into()
    }
}

// ---------------------------------------------------------------------------
// SystemBuilder
// ---------------------------------------------------------------------------

#[pyclass(name = "SystemBuilder")]
pub struct PySystemBuilder;

#[pymethods]
impl PySystemBuilder {
    #[new]
    fn new() -> Self {
        Self
    }

    fn engine(&self, key: &str) -> PyResult<Self> {
        Ok(Self)
    }

    fn model(&self, name: &str) -> PyResult<Self> {
        Ok(Self)
    }

    fn agent(&self, name: &str) -> PyResult<Self> {
        Ok(Self)
    }

    fn telemetry(&self, enabled: bool) -> PyResult<Self> {
        Ok(Self)
    }

    fn traces(&self, enabled: bool) -> PyResult<Self> {
        Ok(Self)
    }

    fn build(&self) -> PyResult<PyJarvisSystem> {
        Ok(PyJarvisSystem)
    }

    fn __repr__(&self) -> String {
        "SystemBuilder()".into()
    }
}

// ---------------------------------------------------------------------------
// QueryOrchestrator
// ---------------------------------------------------------------------------

#[pyclass(name = "QueryOrchestrator")]
pub struct PyQueryOrchestrator;

#[pymethods]
impl PyQueryOrchestrator {
    #[new]
    fn new() -> Self {
        Self
    }

    /// Detect agent intent from a query string.
    fn detect_agent_intent(&self, query: &str) -> Option<String> {
        if query.to_lowercase().contains("morning digest") {
            Some("morning_digest".into())
        } else if query.to_lowercase().contains("deep research") {
            Some("deep_research".into())
        } else {
            None
        }
    }

    fn __repr__(&self) -> String {
        "QueryOrchestrator()".into()
    }
}
