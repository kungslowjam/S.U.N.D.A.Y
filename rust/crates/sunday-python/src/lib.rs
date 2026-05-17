//! PyO3 bridge — exposes ~50 Rust classes to Python via `sunday_rust`.
#![allow(clippy::redundant_closure, unused_variables)]

use once_cell::sync::Lazy;
use pyo3::prelude::*;

// Shared tokio runtime for async-to-sync bridge (agents, future async APIs).
pub(crate) static RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Runtime::new().expect("Failed to create tokio runtime")
});

pub mod a2a;
pub mod agents;
pub mod bench;
pub mod core;
pub mod engine;
pub mod harness;
pub mod learning;
pub mod mcp;
pub mod recipes;
pub mod scheduler;
pub mod security;
pub mod sessions;
pub mod skills;
pub mod storage;
pub mod sandbox;
pub mod system;
pub mod telemetry;
pub mod templates;
pub mod tools;
pub mod traces;
pub mod workflow;

// Module-level functions

#[pyfunction]
#[pyo3(signature = (path=None))]
fn load_config(path: Option<&str>) -> PyResult<core::PyConfig> {
    let p = path.map(std::path::Path::new);
    let config = sunday_core::load_config(p)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    Ok(core::PyConfig { inner: config })
}

#[pyfunction]
fn detect_hardware() -> PyResult<String> {
    let hw = sunday_core::hardware::detect_hardware();
    Ok(serde_json::to_string(&hw).unwrap_or_default())
}

#[pyfunction]
fn check_ssrf(url: &str) -> Option<String> {
    sunday_security::check_ssrf(url)
}

#[pyfunction]
fn is_sensitive_file(path: &str) -> bool {
    sunday_security::is_sensitive_file(std::path::Path::new(path))
}

#[pyfunction]
fn register_builtin_models() {
    sunday_core::model_catalog::register_builtin_models();
}

#[pyfunction]
fn classify_query(query: &str) -> &'static str {
    sunday_learning::classify_query(query)
}

#[pymodule]
fn sunday_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // --- Core types ---
    m.add_class::<core::PyMessage>()?;
    m.add_class::<core::PyToolResult>()?;
    m.add_class::<core::PyToolCall>()?;
    m.add_class::<core::PyConfig>()?;
    m.add_class::<core::PyEventBus>()?;
    m.add_class::<core::PyModelSpec>()?;
    m.add_class::<core::PyRoutingContext>()?;
    m.add_class::<core::PyAgentContext>()?;
    m.add_class::<core::PyAgentResult>()?;
    m.add_class::<core::PyTokenizer>()?;

    // --- Engines ---
    m.add_class::<engine::PyEngine>()?;
    m.add_class::<engine::PyOllamaEngine>()?;

    // --- Agents ---
    m.add_class::<agents::PySimpleAgent>()?;
    m.add_class::<agents::PyOrchestratorAgent>()?;
    m.add_class::<agents::PyNativeReActAgent>()?;
    m.add_class::<agents::PyNativeOpenHandsAgent>()?;
    m.add_class::<agents::PyMonitorOperativeAgent>()?;
    m.add_class::<agents::PyLoopGuard>()?;

    // --- Agent parsing utilities ---
    m.add_function(wrap_pyfunction!(agents::parse_structured_response, m)?)?;
    m.add_function(wrap_pyfunction!(agents::extract_tool_call, m)?)?;
    m.add_function(wrap_pyfunction!(agents::strip_think_tags, m)?)?;
    m.add_function(wrap_pyfunction!(agents::to_slack_fmt, m)?)?;
    m.add_function(wrap_pyfunction!(agents::clean_browser_text, m)?)?;
    m.add_function(wrap_pyfunction!(agents::compress_tool_outputs, m)?)?;
    m.add_function(wrap_pyfunction!(agents::apply_window, m)?)?;

    // --- Skill auto-creation & user modeling ---
    m.add_class::<agents::PySkillAutoCreator>()?;
    m.add_function(wrap_pyfunction!(agents::analyze_conversation_for_skill, m)?)?;
    m.add_function(wrap_pyfunction!(agents::generate_skill_manifest, m)?)?;
    m.add_class::<agents::PyUserModelStore>()?;

    // --- Tools ---
    m.add_class::<tools::PyToolExecutor>()?;
    m.add_class::<tools::PyCalculatorTool>()?;
    m.add_class::<tools::PyThinkTool>()?;
    m.add_class::<tools::PyFileReadTool>()?;
    m.add_class::<tools::PyFileWriteTool>()?;
    m.add_class::<tools::PyListDirectoryTool>()?;
    m.add_class::<tools::PyShellExecTool>()?;
    m.add_class::<tools::PyHttpRequestTool>()?;
    m.add_class::<tools::PyGitStatusTool>()?;
    m.add_class::<tools::PyGitDiffTool>()?;
    m.add_class::<tools::PyGitLogTool>()?;
    m.add_class::<tools::PyApplyPatchTool>()?;
    m.add_class::<tools::PySemanticScholarSearchTool>()?;
    m.add_class::<tools::PyArxivSearchTool>()?;
    m.add_class::<tools::PyOpenAlexSearchTool>()?;
    m.add_class::<tools::PyAXTreeProcessor>()?;
    m.add_class::<tools::PyNativeBrowser>()?;
    m.add_class::<tools::PyNativeMiner>()?;

    // --- Storage / Memory ---
    m.add_class::<storage::PySQLiteMemory>()?;
    m.add_class::<storage::PyBM25Memory>()?;
    m.add_class::<storage::PyFAISSMemory>()?;
    m.add_class::<storage::PyColBERTMemory>()?;
    m.add_class::<storage::PyHybridMemory>()?;
    m.add_class::<storage::PyKnowledgeGraphMemory>()?;
    m.add_class::<storage::PyMdChunk>()?;
    m.add_function(wrap_pyfunction!(storage::chunk_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(storage::dedupe_chunks, m)?)?;

    // --- Security ---
    m.add_class::<security::PySecretScanner>()?;
    m.add_class::<security::PyPIIScanner>()?;
    m.add_class::<security::PyGuardrailsEngine>()?;
    m.add_class::<security::PyAuditLogger>()?;
    m.add_class::<security::PyCapabilityPolicy>()?;
    m.add_class::<security::PyInjectionScanner>()?;
    m.add_class::<security::PyRateLimiter>()?;
    m.add_class::<security::PyTaintSet>()?;
    m.add_class::<sandbox::PyNativeSandbox>()?;

    // --- Telemetry ---
    m.add_class::<telemetry::PyTelemetryStore>()?;
    m.add_class::<telemetry::PyTelemetryAggregator>()?;
    m.add_class::<telemetry::PyInstrumentedEngine>()?;
    // --- Telemetry (new session/phase/ITL/FLOPs classes) ---
    m.add_class::<telemetry::PyTelemetrySample>()?;
    m.add_class::<telemetry::PyTelemetrySessionCore>()?;
    m.add_class::<telemetry::PyItlStats>()?;
    m.add_class::<telemetry::PyFlopsEstimator>()?;
    m.add_class::<telemetry::PyPhaseMetrics>()?;

    // --- Traces ---
    m.add_class::<traces::PyTraceStore>()?;
    m.add_class::<traces::PyTraceCollector>()?;
    m.add_class::<traces::PyTraceAnalyzer>()?;

    // --- Learning ---
    m.add_class::<learning::PyHeuristicRouter>()?;
    m.add_class::<learning::PyBanditRouterPolicy>()?;
    m.add_class::<learning::PyGRPORouterPolicy>()?;
    m.add_class::<learning::PyOptimizationStore>()?;
    m.add_class::<learning::PyLLMOptimizer>()?;
    m.add_class::<learning::PySFTRouterPolicy>()?;
    m.add_class::<learning::PyHeuristicRewardFunction>()?;
    m.add_class::<learning::PySkillDiscovery>()?;
    m.add_class::<learning::PySkillEvolutionEngine>()?;
    m.add_class::<learning::PyTraceDrivenPolicy>()?;
    m.add_class::<learning::PyAgentAdvisorPolicy>()?;
    m.add_class::<learning::PyICLUpdaterPolicy>()?;
    m.add_class::<learning::PyTrainingDataMiner>()?;
    m.add_class::<learning::PyAgentConfigEvolver>()?;
    m.add_class::<learning::PyMultiObjectiveReward>()?;
    m.add_class::<learning::PyLearningOrchestrator>()?;

    // --- MCP ---
    m.add_class::<mcp::PyMcpServer>()?;

    // --- Sessions ---
    m.add_class::<sessions::PySessionStore>()?;

    // --- Workflow ---
    m.add_class::<workflow::PyWorkflowGraph>()?;
    m.add_class::<workflow::PyWorkflowEngine>()?;

    // --- Skills ---
    m.add_class::<skills::PySkillManifest>()?;

    // --- Recipes ---
    m.add_class::<recipes::PyRecipe>()?;

    // --- Templates ---
    m.add_class::<templates::PyAgentTemplate>()?;

    // --- A2A ---
    m.add_class::<a2a::PyAgentCard>()?;
    m.add_class::<a2a::PyA2ATaskStore>()?;

    // --- Scheduler ---
    m.add_class::<scheduler::PySchedulerStore>()?;

    // --- Bench ---
    m.add_class::<bench::PyBenchmarkResult>()?;
    m.add_class::<bench::PyBenchmarkSuite>()?;
    m.add_class::<bench::PyLatencyBenchmark>()?;
    m.add_class::<bench::PyThroughputBenchmark>()?;
    m.add_class::<bench::PyEnergyBenchmark>()?;

    // --- Harness ---
    m.add_class::<harness::PyHarnessConfig>()?;
    m.add_class::<harness::PyAssertion>()?;
    m.add_class::<harness::PyAssertionType>()?;
    m.add_class::<harness::PyAssertionResult>()?;
    m.add_class::<harness::PyTestResult>()?;
    m.add_class::<harness::PyPerformanceTracker>()?;
    m.add_class::<harness::PyVisualRegressionChecker>()?;
    m.add_class::<harness::PyBootOrchestrator>()?;
    m.add_class::<harness::PySkillHarness>()?;
    m.add_function(wrap_pyfunction!(harness::evaluate_assertions, m)?)?;
    m.add_function(wrap_pyfunction!(harness::all_required_passed, m)?)?;

    // --- System ---
    m.add_class::<system::PyJarvisSystem>()?;
    m.add_class::<system::PySystemBuilder>()?;
    m.add_class::<system::PyQueryOrchestrator>()?;
    m.add_class::<system::PySecurityContext>()?;
    m.add_class::<system::PyObservability>()?;
    m.add_class::<system::PyAgentRuntime>()?;
    m.add_class::<system::PyScheduling>()?;

    // --- Mining ---
    m.add_class::<sunday_mining::NativeMiner>()?;

    // --- Module-level functions ---
    m.add_function(wrap_pyfunction!(load_config, m)?)?;
    m.add_function(wrap_pyfunction!(detect_hardware, m)?)?;
    m.add_function(wrap_pyfunction!(check_ssrf, m)?)?;
    m.add_function(wrap_pyfunction!(is_sensitive_file, m)?)?;
    m.add_function(wrap_pyfunction!(register_builtin_models, m)?)?;
    m.add_function(wrap_pyfunction!(classify_query, m)?)?;
    m.add_function(wrap_pyfunction!(skills::load_skill, m)?)?;
    m.add_function(wrap_pyfunction!(skills::parse_skill_markdown, m)?)?;
    m.add_function(wrap_pyfunction!(recipes::load_recipe, m)?)?;
    m.add_function(wrap_pyfunction!(templates::load_template, m)?)?;
    m.add_function(wrap_pyfunction!(a2a::parse_a2a_request, m)?)?;
    m.add_function(wrap_pyfunction!(scheduler::parse_cron_next, m)?)?;
    m.add_function(wrap_pyfunction!(bench::bench_ensure_registered, m)?)?;
    m.add_function(wrap_pyfunction!(bench::bench_compute_stats, m)?)?;

    Ok(())
}
