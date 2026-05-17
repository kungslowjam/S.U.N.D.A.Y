//! PyO3 bindings for sunday-harness.

use crate::RUNTIME;
use pyo3::prelude::*;
use std::path::PathBuf;
use std::time::Duration;

// ---------------------------------------------------------------------------
// AssertionType
// ---------------------------------------------------------------------------

#[pyclass(name = "AssertionType")]
#[derive(Clone, Copy)]
pub struct PyAssertionType {
    pub inner: sunday_harness::AssertionType,
}

#[pymethods]
#[allow(non_snake_case)]
impl PyAssertionType {
    #[classattr]
    fn TEXT_CONTAINS() -> Self {
        Self {
            inner: sunday_harness::AssertionType::TextContains,
        }
    }
    #[classattr]
    fn TEXT_REGEX() -> Self {
        Self {
            inner: sunday_harness::AssertionType::TextRegex,
        }
    }
    #[classattr]
    fn JSON_SCHEMA() -> Self {
        Self {
            inner: sunday_harness::AssertionType::JsonSchema,
        }
    }
    #[classattr]
    fn DOM_SELECTOR() -> Self {
        Self {
            inner: sunday_harness::AssertionType::DomSelector,
        }
    }
    #[classattr]
    fn STATUS_CODE() -> Self {
        Self {
            inner: sunday_harness::AssertionType::StatusCode,
        }
    }
    #[classattr]
    fn LATENCY_THRESHOLD() -> Self {
        Self {
            inner: sunday_harness::AssertionType::LatencyThreshold,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }
}

// ---------------------------------------------------------------------------
// Assertion
// ---------------------------------------------------------------------------

#[pyclass(name = "Assertion")]
#[derive(Clone)]
pub struct PyAssertion {
    pub inner: sunday_harness::Assertion,
}

#[pymethods]
impl PyAssertion {
    #[new]
    #[pyo3(signature = (assertion_type, expected, description="", required=true))]
    fn new(
        assertion_type: &PyAssertionType,
        expected: &str,
        description: &str,
        required: bool,
    ) -> PyResult<Self> {
        let val: serde_json::Value = serde_json::from_str(expected)
            .unwrap_or_else(|_| serde_json::Value::String(expected.to_string()));
        let mut inner = sunday_harness::Assertion::new(assertion_type.inner, val);
        inner.description = description.to_string();
        inner.required = required;
        Ok(Self { inner })
    }

    #[getter]
    fn assertion_type(&self) -> PyAssertionType {
        PyAssertionType {
            inner: self.inner.assertion_type,
        }
    }

    #[getter]
    fn description(&self) -> String {
        self.inner.description.clone()
    }

    #[getter]
    fn required(&self) -> bool {
        self.inner.required
    }

    #[getter]
    fn expected(&self) -> String {
        self.inner.expected.to_string()
    }
}

// ---------------------------------------------------------------------------
// AssertionResult
// ---------------------------------------------------------------------------

#[pyclass(name = "AssertionResult")]
#[derive(Clone)]
pub struct PyAssertionResult {
    pub inner: sunday_harness::AssertionResult,
}

#[pymethods]
impl PyAssertionResult {
    #[getter]
    fn passed(&self) -> bool {
        self.inner.passed
    }
    #[getter]
    fn message(&self) -> String {
        self.inner.message.clone()
    }
    #[getter]
    fn actual(&self) -> Option<String> {
        self.inner.actual.as_ref().map(|v| v.to_string())
    }
    #[getter]
    fn assertion(&self) -> PyAssertion {
        PyAssertion {
            inner: self.inner.assertion.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// HarnessConfig
// ---------------------------------------------------------------------------

#[pyclass(name = "HarnessConfig")]
#[derive(Clone)]
pub struct PyHarnessConfig {
    pub inner: sunday_harness::HarnessConfig,
}

#[pymethods]
impl PyHarnessConfig {
    #[new]
    #[pyo3(signature = (
        max_retries=3,
        retry_base_delay=2.0,
        retry_max_delay=30.0,
        retry_backoff_multiplier=2.0,
        max_turns=10,
        visual_audit=false,
        parallel_tools=false,
        screenshot_on_pass=true,
        screenshot_on_fail=true,
        latency_baseline_path=None,
        visual_baseline_path=None,
        process_kill_cmd=None
    ))]
    fn new(
        max_retries: u32,
        retry_base_delay: f64,
        retry_max_delay: f64,
        retry_backoff_multiplier: f64,
        max_turns: u32,
        visual_audit: bool,
        parallel_tools: bool,
        screenshot_on_pass: bool,
        screenshot_on_fail: bool,
        latency_baseline_path: Option<&str>,
        visual_baseline_path: Option<&str>,
        process_kill_cmd: Option<&str>,
    ) -> Self {
        Self {
            inner: sunday_harness::HarnessConfig {
                max_retries,
                retry_base_delay: Duration::from_secs_f64(retry_base_delay),
                retry_max_delay: Duration::from_secs_f64(retry_max_delay),
                retry_backoff_multiplier,
                max_turns,
                visual_audit,
                parallel_tools,
                screenshot_on_pass,
                screenshot_on_fail,
                latency_baseline_path: latency_baseline_path.map(PathBuf::from),
                visual_baseline_path: visual_baseline_path.map(PathBuf::from),
                process_kill_cmd: process_kill_cmd.map(String::from),
            },
        }
    }

    #[getter]
    fn max_retries(&self) -> u32 {
        self.inner.max_retries
    }
    #[getter]
    fn retry_base_delay(&self) -> f64 {
        self.inner.retry_base_delay.as_secs_f64()
    }
    #[getter]
    fn retry_max_delay(&self) -> f64 {
        self.inner.retry_max_delay.as_secs_f64()
    }
    #[getter]
    fn retry_backoff_multiplier(&self) -> f64 {
        self.inner.retry_backoff_multiplier
    }
    #[getter]
    fn max_turns(&self) -> u32 {
        self.inner.max_turns
    }
    #[getter]
    fn visual_audit(&self) -> bool {
        self.inner.visual_audit
    }
    #[getter]
    fn parallel_tools(&self) -> bool {
        self.inner.parallel_tools
    }
    #[getter]
    fn screenshot_on_pass(&self) -> bool {
        self.inner.screenshot_on_pass
    }
    #[getter]
    fn screenshot_on_fail(&self) -> bool {
        self.inner.screenshot_on_fail
    }
}

// ---------------------------------------------------------------------------
// TestResult
// ---------------------------------------------------------------------------

#[pyclass(name = "TestResult")]
#[derive(Clone)]
pub struct PyTestResult {
    pub inner: sunday_harness::TestResult,
}

#[pymethods]
impl PyTestResult {
    #[getter]
    fn tool_id(&self) -> String {
        self.inner.tool_id.clone()
    }
    #[getter]
    fn prompt(&self) -> String {
        self.inner.prompt.clone()
    }
    #[getter]
    fn success(&self) -> bool {
        self.inner.success
    }
    #[getter]
    fn output(&self) -> String {
        self.inner.output.clone()
    }
    #[getter]
    fn error(&self) -> Option<String> {
        self.inner.error.clone()
    }
    #[getter]
    fn latency(&self) -> f64 {
        self.inner.latency.as_secs_f64()
    }
    #[getter]
    fn retry_count(&self) -> u32 {
        self.inner.retry_count
    }
    #[getter]
    fn visual_evidence(&self) -> Option<String> {
        self.inner.visual_evidence.as_ref().map(|p| p.to_string_lossy().to_string())
    }
    #[getter]
    fn assertion_results(&self) -> Vec<PyAssertionResult> {
        self.inner.assertion_results.iter().map(|r| PyAssertionResult { inner: r.clone() }).collect()
    }
}

// ---------------------------------------------------------------------------
// PerformanceTracker
// ---------------------------------------------------------------------------

#[pyclass(name = "PerformanceTracker")]
pub struct PyPerformanceTracker {
    inner: sunday_harness::PerformanceTracker,
}

#[pymethods]
impl PyPerformanceTracker {
    #[new]
    fn new(path: &str) -> Self {
        Self {
            inner: sunday_harness::PerformanceTracker::new(path),
        }
    }

    fn record(&mut self, tool_id: &str, latency_secs: f64) {
        self.inner.record(tool_id, Duration::from_secs_f64(latency_secs));
    }

    fn check_regression(&self, tool_id: &str, latency_secs: f64) -> (bool, Option<f64>, f64) {
        self.inner.check_regression(tool_id, Duration::from_secs_f64(latency_secs))
    }

    fn update_baseline(&mut self, tool_id: &str, latency_secs: f64) {
        self.inner.update_baseline(tool_id, Duration::from_secs_f64(latency_secs));
    }

    fn get_baseline_latency_p50(&self, tool_id: &str) -> Option<f64> {
        self.inner.get_baseline(tool_id).map(|b| b.latency_p50)
    }
}

// ---------------------------------------------------------------------------
// VisualRegressionChecker
// ---------------------------------------------------------------------------

#[pyclass(name = "VisualRegressionChecker")]
pub struct PyVisualRegressionChecker {
    inner: sunday_harness::VisualRegressionChecker,
}

#[pymethods]
impl PyVisualRegressionChecker {
    #[new]
    fn new(baseline_dir: &str, output_dir: &str) -> Self {
        Self {
            inner: sunday_harness::VisualRegressionChecker::new(baseline_dir, output_dir),
        }
    }

    fn compute_ssim(&self, img1_path: &str, img2_path: &str) -> PyResult<f64> {
        self.inner
            .compute_ssim(PathBuf::from(img1_path).as_path(), PathBuf::from(img2_path).as_path())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    fn compare_against_baseline(&self, name: &str, screenshot_path: &str) -> PyResult<(f64, bool)> {
        self.inner
            .compare_against_baseline(name, PathBuf::from(screenshot_path).as_path())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    fn update_baseline(&self, name: &str, screenshot_path: &str) -> PyResult<()> {
        self.inner
            .update_baseline(name, PathBuf::from(screenshot_path).as_path())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    fn screenshot_path(&self, label: &str) -> String {
        self.inner.screenshot_path(label).to_string_lossy().to_string()
    }

    fn pixel_diff_ratio(&self, img1_path: &str, img2_path: &str) -> PyResult<f64> {
        self.inner
            .pixel_diff_ratio(PathBuf::from(img1_path).as_path(), PathBuf::from(img2_path).as_path())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// BootOrchestrator
// ---------------------------------------------------------------------------

#[pyclass(name = "BootOrchestrator")]
pub struct PyBootOrchestrator {
    inner: sunday_harness::BootOrchestrator,
}

#[pymethods]
impl PyBootOrchestrator {
    #[new]
    fn new() -> Self {
        Self {
            inner: sunday_harness::BootOrchestrator::new(),
        }
    }

    #[pyo3(signature = (llama_port, backend_port, frontend_port, model_path=None))]
    fn cold_start(
        &mut self,
        llama_port: u16,
        backend_port: u16,
        frontend_port: u16,
        model_path: Option<&str>,
    ) -> PyResult<()> {
        let path = model_path.map(PathBuf::from);
        RUNTIME
            .block_on(self.inner.cold_start(llama_port, backend_port, frontend_port, path))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SkillHarness (orchestration + report generation)
// ---------------------------------------------------------------------------

#[pyclass(name = "SkillHarness")]
pub struct PySkillHarness;

#[pymethods]
impl PySkillHarness {
    #[new]
    fn new(_config: &PyHarnessConfig) -> Self {
        Self
    }

    /// Evaluate a list of assertions against output text.
    fn evaluate_assertions(
        &self,
        assertions: Vec<PyAssertion>,
        output: &str,
        latency_secs: f64,
    ) -> Vec<PyAssertionResult> {
        let raw: Vec<sunday_harness::Assertion> =
            assertions.into_iter().map(|a| a.inner).collect();
        sunday_harness::AssertionEngine::evaluate_all(
            &raw,
            output,
            Duration::from_secs_f64(latency_secs),
        )
        .into_iter()
        .map(|r| PyAssertionResult { inner: r })
        .collect()
    }

    /// Check if all required assertions passed.
    fn all_required_passed(&self, results: Vec<PyAssertionResult>) -> bool {
        let raw: Vec<sunday_harness::AssertionResult> =
            results.into_iter().map(|r| r.inner).collect();
        sunday_harness::AssertionEngine::all_required_passed(&raw)
    }

    /// Generate a markdown report from test results.
    fn generate_report(&self, results: Vec<PyTestResult>) -> String {
        let raw: Vec<sunday_harness::TestResult> =
            results.into_iter().map(|r| r.inner).collect();
        sunday_harness::SkillHarness::generate_report(&raw)
    }
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

/// Evaluate assertions without instantiating SkillHarness.
#[pyfunction]
pub fn evaluate_assertions(
    assertions: Vec<PyAssertion>,
    output: &str,
    latency_secs: f64,
) -> Vec<PyAssertionResult> {
    let raw: Vec<sunday_harness::Assertion> = assertions.into_iter().map(|a| a.inner).collect();
    sunday_harness::AssertionEngine::evaluate_all(
        &raw,
        output,
        Duration::from_secs_f64(latency_secs),
    )
    .into_iter()
    .map(|r| PyAssertionResult { inner: r })
    .collect()
}

/// Check if all required assertions passed.
#[pyfunction]
pub fn all_required_passed(results: Vec<PyAssertionResult>) -> bool {
    let raw: Vec<sunday_harness::AssertionResult> =
        results.into_iter().map(|r| r.inner).collect();
    sunday_harness::AssertionEngine::all_required_passed(&raw)
}
