//! Benchmark trait — ABC for all benchmark implementations.

use crate::result::BenchmarkResult;
use sunday_engine::InferenceEngine;

/// Base trait for all benchmark implementations.
///
/// Implementations should be registered via `sunday_core::BENCHMARK_REGISTRY`
/// to become discoverable at runtime.
pub trait Benchmark: Send + Sync {
    /// Short identifier for this benchmark.
    fn name(&self) -> &'static str;

    /// Human-readable description of what this benchmark measures.
    fn description(&self) -> &'static str;

    /// Execute the benchmark and return results.
    ///
    /// # Arguments
    /// * `engine` — The inference engine backend to benchmark
    /// * `model` — Model identifier to use
    /// * `num_samples` — Number of measurement samples
    /// * `warmup_samples` — Number of warmup iterations before measurement
    fn run(
        &self,
        engine: &dyn InferenceEngine,
        model: &str,
        num_samples: usize,
        warmup_samples: usize,
    ) -> BenchmarkResult;
}
