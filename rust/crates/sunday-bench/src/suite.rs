//! Benchmark suite — run a collection of benchmarks and aggregate results.

use crate::result::BenchmarkResult;
use crate::traits::Benchmark;
use serde_json;
use std::collections::HashMap;
use sunday_engine::InferenceEngine;

/// Run a collection of benchmarks and aggregate results.
pub struct BenchmarkSuite {
    benchmarks: Vec<Box<dyn Benchmark>>,
}

impl BenchmarkSuite {
    /// Create a new suite with the given benchmarks.
    pub fn new(benchmarks: Vec<Box<dyn Benchmark>>) -> Self {
        Self { benchmarks }
    }

    /// Create an empty suite.
    pub fn empty() -> Self {
        Self { benchmarks: Vec::new() }
    }

    /// Add a benchmark to the suite.
    pub fn add(&mut self, benchmark: Box<dyn Benchmark>) {
        self.benchmarks.push(benchmark);
    }

    /// Run all benchmarks and return a list of results.
    pub fn run_all(
        &self,
        engine: &dyn InferenceEngine,
        model: &str,
        num_samples: usize,
        warmup_samples: usize,
    ) -> Vec<BenchmarkResult> {
        self.benchmarks
            .iter()
            .map(|bench| bench.run(engine, model, num_samples, warmup_samples))
            .collect()
    }

    /// Serialize results to JSONL format (one JSON object per line).
    pub fn to_jsonl(results: &[BenchmarkResult]) -> String {
        results
            .iter()
            .map(|r| {
                let obj = serde_json::json!({
                    "benchmark_name": r.benchmark_name,
                    "model": r.model,
                    "engine": r.engine,
                    "metrics": r.metrics,
                    "metadata": r.metadata,
                    "samples": r.samples,
                    "errors": r.errors,
                });
                obj.to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Create a summary dict from benchmark results.
    pub fn summary(results: &[BenchmarkResult]) -> HashMap<String, serde_json::Value> {
        let benchmarks: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.benchmark_name,
                    "model": r.model,
                    "engine": r.engine,
                    "metrics": r.metrics,
                    "samples": r.samples,
                    "errors": r.errors,
                })
            })
            .collect();

        let mut summary = HashMap::new();
        summary.insert("benchmark_count".into(), serde_json::json!(results.len()));
        summary.insert("benchmarks".into(), serde_json::json!(benchmarks));
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::BenchmarkResult;
    use sunday_core::{GenerateResult, Message};
    use sunday_engine::InferenceEngine;

    struct DummyBench {
        name: &'static str,
    }

    impl Benchmark for DummyBench {
        fn name(&self) -> &'static str {
            self.name
        }

        fn description(&self) -> &'static str {
            "dummy benchmark"
        }

        fn run(
            &self,
            _engine: &dyn InferenceEngine,
            model: &str,
            num_samples: usize,
            _warmup_samples: usize,
        ) -> BenchmarkResult {
            let mut result = BenchmarkResult::new(self.name, model, "mock");
            result.samples = num_samples;
            result.metrics.insert("value".into(), 1.0);
            result
        }
    }

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
            Ok(GenerateResult::default())
        }

        #[allow(unused)]
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
    fn test_run_all() {
        let suite = BenchmarkSuite::new(vec![
            Box::new(DummyBench { name: "a" }),
            Box::new(DummyBench { name: "b" }),
        ]);
        let engine = MockEngine;
        let results = suite.run_all(&engine, "m1", 5, 0);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].benchmark_name, "a");
        assert_eq!(results[1].benchmark_name, "b");
    }

    #[test]
    fn test_run_all_empty() {
        let suite = BenchmarkSuite::empty();
        let engine = MockEngine;
        let results = suite.run_all(&engine, "m1", 10, 0);
        assert!(results.is_empty());
    }

    #[test]
    fn test_to_jsonl() {
        let results = vec![
            BenchmarkResult::new("test", "m1", "mock"),
        ];
        let jsonl = BenchmarkSuite::to_jsonl(&results);
        let lines: Vec<&str> = jsonl.trim().split('\n').collect();
        assert_eq!(lines.len(), 1);
        let obj: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(obj["benchmark_name"], "test");
    }

    #[test]
    fn test_to_jsonl_multiple() {
        let results = vec![
            BenchmarkResult::new("a", "m1", "mock"),
            BenchmarkResult::new("b", "m1", "mock"),
        ];
        let jsonl = BenchmarkSuite::to_jsonl(&results);
        for line in jsonl.trim().split('\n') {
            let obj: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(obj.get("benchmark_name").is_some());
        }
    }

    #[test]
    fn test_summary_format() {
        let suite = BenchmarkSuite::new(vec![Box::new(DummyBench { name: "test" })]);
        let engine = MockEngine;
        let results = suite.run_all(&engine, "m1", 10, 0);
        let summary = BenchmarkSuite::summary(&results);
        assert!(summary.contains_key("benchmark_count"));
        assert!(summary.contains_key("benchmarks"));
    }

    #[test]
    fn test_summary_count() {
        let suite = BenchmarkSuite::new(vec![
            Box::new(DummyBench { name: "a" }),
            Box::new(DummyBench { name: "b" }),
        ]);
        let engine = MockEngine;
        let results = suite.run_all(&engine, "m1", 10, 0);
        let summary = BenchmarkSuite::summary(&results);
        assert_eq!(summary["benchmark_count"], 2);
        let benchmarks = summary["benchmarks"].as_array().unwrap();
        assert_eq!(benchmarks.len(), 2);
    }
}
