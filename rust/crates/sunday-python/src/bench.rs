//! PyO3 bindings for sunday-bench — benchmarking framework.

use pyo3::prelude::*;
use std::collections::HashMap;
use sunday_bench::Benchmark;

// ---------------------------------------------------------------------------
// BenchmarkResult
// ---------------------------------------------------------------------------

#[pyclass(name = "BenchmarkResult")]
#[derive(Clone)]
pub struct PyBenchmarkResult {
    #[pyo3(get)]
    pub benchmark_name: String,
    #[pyo3(get)]
    pub model: String,
    #[pyo3(get)]
    pub engine: String,
    #[pyo3(get)]
    pub metrics: HashMap<String, f64>,
    #[pyo3(get)]
    pub metadata: HashMap<String, String>,
    #[pyo3(get)]
    pub samples: usize,
    #[pyo3(get)]
    pub errors: usize,
    #[pyo3(get)]
    pub warmup_samples: usize,
    #[pyo3(get)]
    pub steady_state_samples: usize,
    #[pyo3(get)]
    pub steady_state_reached: bool,
    #[pyo3(get)]
    pub total_energy_joules: f64,
    #[pyo3(get)]
    pub energy_per_token_joules: f64,
    #[pyo3(get)]
    pub energy_method: String,
}

impl From<sunday_bench::BenchmarkResult> for PyBenchmarkResult {
    fn from(r: sunday_bench::BenchmarkResult) -> Self {
        let metadata: HashMap<String, String> = r
            .metadata
            .into_iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string())))
            .collect();
        Self {
            benchmark_name: r.benchmark_name,
            model: r.model,
            engine: r.engine,
            metrics: r.metrics,
            metadata,
            samples: r.samples,
            errors: r.errors,
            warmup_samples: r.warmup_samples,
            steady_state_samples: r.steady_state_samples,
            steady_state_reached: r.steady_state_reached,
            total_energy_joules: r.total_energy_joules,
            energy_per_token_joules: r.energy_per_token_joules,
            energy_method: r.energy_method,
        }
    }
}

#[pymethods]
impl PyBenchmarkResult {
    #[new]
    fn new(benchmark_name: String, model: String, engine: String) -> Self {
        Self {
            benchmark_name,
            model,
            engine,
            metrics: HashMap::new(),
            metadata: HashMap::new(),
            samples: 0,
            errors: 0,
            warmup_samples: 0,
            steady_state_samples: 0,
            steady_state_reached: false,
            total_energy_joules: 0.0,
            energy_per_token_joules: 0.0,
            energy_method: String::new(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "BenchmarkResult(name='{}', model='{}', engine='{}', samples={}, errors={})",
            self.benchmark_name, self.model, self.engine, self.samples, self.errors
        )
    }

    fn to_json(&self) -> PyResult<String> {
        let obj = serde_json::json!({
            "benchmark_name": self.benchmark_name,
            "model": self.model,
            "engine": self.engine,
            "metrics": self.metrics,
            "metadata": self.metadata,
            "samples": self.samples,
            "errors": self.errors,
            "warmup_samples": self.warmup_samples,
            "steady_state_samples": self.steady_state_samples,
            "steady_state_reached": self.steady_state_reached,
            "total_energy_joules": self.total_energy_joules,
            "energy_per_token_joules": self.energy_per_token_joules,
            "energy_method": self.energy_method,
        });
        Ok(obj.to_string())
    }
}

// ---------------------------------------------------------------------------
// BenchmarkSuite
// ---------------------------------------------------------------------------

#[pyclass(name = "BenchmarkSuite")]
pub struct PyBenchmarkSuite;

#[pymethods]
impl PyBenchmarkSuite {
    #[new]
    fn new() -> Self {
        Self
    }

    /// Run all built-in benchmarks and return results.
    #[pyo3(signature = (engine, model, num_samples=10, warmup_samples=0))]
    fn run_all(
        &self,
        engine: &super::engine::PyEngine,
        model: &str,
        num_samples: usize,
        warmup_samples: usize,
    ) -> PyResult<Vec<PyBenchmarkResult>> {
        sunday_bench::benchmarks::ensure_all_registered();

        let suite = sunday_bench::BenchmarkSuite::new(vec![
            Box::new(sunday_bench::LatencyBenchmark),
            Box::new(sunday_bench::ThroughputBenchmark),
            Box::new(sunday_bench::EnergyBenchmark),
        ]);

        let results = suite.run_all(&engine.inner, model, num_samples, warmup_samples);
        Ok(results.into_iter().map(PyBenchmarkResult::from).collect())
    }

    /// Serialize a list of results to JSONL.
    #[staticmethod]
    fn to_jsonl(results: Vec<PyBenchmarkResult>) -> PyResult<String> {
        let core_results: Vec<sunday_bench::BenchmarkResult> = results
            .into_iter()
            .map(|r| sunday_bench::BenchmarkResult {
                benchmark_name: r.benchmark_name,
                model: r.model,
                engine: r.engine,
                metrics: r.metrics,
                metadata: r
                    .metadata
                    .into_iter()
                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                    .collect(),
                samples: r.samples,
                errors: r.errors,
                warmup_samples: r.warmup_samples,
                steady_state_samples: r.steady_state_samples,
                steady_state_reached: r.steady_state_reached,
                total_energy_joules: r.total_energy_joules,
                energy_per_token_joules: r.energy_per_token_joules,
                energy_method: r.energy_method,
            })
            .collect();
        Ok(sunday_bench::BenchmarkSuite::to_jsonl(&core_results))
    }

    /// Create a summary dict from results (returns JSON string).
    #[staticmethod]
    fn summary(results: Vec<PyBenchmarkResult>) -> PyResult<String> {
        let core_results: Vec<sunday_bench::BenchmarkResult> = results
            .into_iter()
            .map(|r| sunday_bench::BenchmarkResult {
                benchmark_name: r.benchmark_name,
                model: r.model,
                engine: r.engine,
                metrics: r.metrics,
                metadata: r
                    .metadata
                    .into_iter()
                    .map(|(k, v)| (k, serde_json::Value::String(v)))
                    .collect(),
                samples: r.samples,
                errors: r.errors,
                warmup_samples: r.warmup_samples,
                steady_state_samples: r.steady_state_samples,
                steady_state_reached: r.steady_state_reached,
                total_energy_joules: r.total_energy_joules,
                energy_per_token_joules: r.energy_per_token_joules,
                energy_method: r.energy_method,
            })
            .collect();
        let summary = sunday_bench::BenchmarkSuite::summary(&core_results);
        Ok(serde_json::to_string(&summary).unwrap_or_default())
    }
}

// ---------------------------------------------------------------------------
// Individual benchmarks
// ---------------------------------------------------------------------------

#[pyclass(name = "LatencyBenchmark")]
pub struct PyLatencyBenchmark;

#[pymethods]
impl PyLatencyBenchmark {
    #[new]
    fn new() -> Self {
        Self
    }

    #[pyo3(signature = (engine, model, num_samples=10, warmup_samples=0))]
    fn run(
        &self,
        engine: &super::engine::PyEngine,
        model: &str,
        num_samples: usize,
        warmup_samples: usize,
    ) -> PyResult<PyBenchmarkResult> {
        let bench = sunday_bench::LatencyBenchmark;
        let result = bench.run(&engine.inner, model, num_samples, warmup_samples);
        Ok(PyBenchmarkResult::from(result))
    }
}

#[pyclass(name = "ThroughputBenchmark")]
pub struct PyThroughputBenchmark;

#[pymethods]
impl PyThroughputBenchmark {
    #[new]
    fn new() -> Self {
        Self
    }

    #[pyo3(signature = (engine, model, num_samples=10, warmup_samples=0))]
    fn run(
        &self,
        engine: &super::engine::PyEngine,
        model: &str,
        num_samples: usize,
        warmup_samples: usize,
    ) -> PyResult<PyBenchmarkResult> {
        let bench = sunday_bench::ThroughputBenchmark;
        let result = bench.run(&engine.inner, model, num_samples, warmup_samples);
        Ok(PyBenchmarkResult::from(result))
    }
}

#[pyclass(name = "EnergyBenchmark")]
pub struct PyEnergyBenchmark;

#[pymethods]
impl PyEnergyBenchmark {
    #[new]
    fn new() -> Self {
        Self
    }

    #[pyo3(signature = (engine, model, num_samples=10, warmup_samples=5))]
    fn run(
        &self,
        engine: &super::engine::PyEngine,
        model: &str,
        num_samples: usize,
        warmup_samples: usize,
    ) -> PyResult<PyBenchmarkResult> {
        let bench = sunday_bench::EnergyBenchmark;
        let result = bench.run(&engine.inner, model, num_samples, warmup_samples);
        Ok(PyBenchmarkResult::from(result))
    }
}

// ---------------------------------------------------------------------------
// Module-level functions
// ---------------------------------------------------------------------------

#[pyfunction]
pub fn bench_ensure_registered() {
    sunday_bench::benchmarks::ensure_all_registered();
}

#[pyfunction]
pub fn bench_compute_stats(name: &str, values: Vec<f64>) -> HashMap<String, f64> {
    sunday_bench::compute_stats(name, &values)
}
