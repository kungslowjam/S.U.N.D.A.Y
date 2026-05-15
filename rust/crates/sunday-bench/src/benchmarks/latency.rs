//! Latency benchmark — measures per-call inference latency.

use crate::result::BenchmarkResult;
use crate::stats::compute_stats;
use crate::traits::Benchmark;
use sunday_core::Message;
use sunday_engine::InferenceEngine;
use sunday_core::registry::BENCHMARK_REGISTRY;
use std::time::Instant;

const CANNED_PROMPTS: &[&str] = &[
    "Hello",
    "What is 2+2?",
    "Explain gravity in one sentence",
];

/// Measures per-call inference latency with short prompts.
pub struct LatencyBenchmark;

impl Benchmark for LatencyBenchmark {
    fn name(&self) -> &'static str {
        "latency"
    }

    fn description(&self) -> &'static str {
        "Measures per-call inference latency with short prompts"
    }

    fn run(
        &self,
        engine: &dyn InferenceEngine,
        model: &str,
        num_samples: usize,
        warmup_samples: usize,
    ) -> BenchmarkResult {
        // Warmup
        for i in 0..warmup_samples {
            let prompt = CANNED_PROMPTS[i % CANNED_PROMPTS.len()];
            let messages = vec![Message::user(prompt)];
            let _ = engine.generate(&messages, model, 0.7, 256, None);
        }

        let mut latencies: Vec<f64> = Vec::with_capacity(num_samples);
        let mut errors = 0;

        for i in 0..num_samples {
            let prompt = CANNED_PROMPTS[i % CANNED_PROMPTS.len()];
            let messages = vec![Message::user(prompt)];
            let t0 = Instant::now();
            match engine.generate(&messages, model, 0.7, 256, None) {
                Ok(_) => {
                    latencies.push(t0.elapsed().as_secs_f64());
                }
                Err(_) => {
                    errors += 1;
                }
            }
        }

        let mut result = BenchmarkResult::new(self.name(), model, engine.engine_id());
        result.samples = num_samples;
        result.errors = errors;

        if !latencies.is_empty() {
            result.metrics = compute_stats("latency", &latencies);
        }

        result
    }
}

/// Register the latency benchmark if not already present.
pub fn ensure_registered() {
    if !BENCHMARK_REGISTRY.contains("latency") {
        let _ = BENCHMARK_REGISTRY.register("latency", serde_json::json!("latency"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sunday_core::{GenerateResult, Message, Usage};

    struct MockEngine {
        fail_after: Option<usize>,
        call_count: std::sync::atomic::AtomicUsize,
    }

    impl MockEngine {
        fn new() -> Self {
            Self {
                fail_after: None,
                call_count: std::sync::atomic::AtomicUsize::new(0),
            }
        }

        fn failing() -> Self {
            Self {
                fail_after: Some(0),
                call_count: std::sync::atomic::AtomicUsize::new(0),
            }
        }
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
            let count = self.call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if let Some(fail_after) = self.fail_after {
                if count >= fail_after {
                    return Err(sunday_core::SUNDAYError::Engine(sunday_core::EngineError::Generation("fail".into())));
                }
            }
            Ok(GenerateResult {
                content: "Hello".into(),
                usage: Usage {
                    prompt_tokens: 5,
                    completion_tokens: 3,
                    total_tokens: 8,
                },
                ..Default::default()
            })
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
    fn test_name() {
        let b = LatencyBenchmark;
        assert_eq!(b.name(), "latency");
    }

    #[test]
    fn test_description() {
        let b = LatencyBenchmark;
        assert!(b.description().to_lowercase().contains("latency"));
    }

    #[test]
    fn test_run_with_mock_engine() {
        let engine = MockEngine::new();
        let b = LatencyBenchmark;
        let result = b.run(&engine, "test-model", 3, 0);
        assert_eq!(result.benchmark_name, "latency");
        assert_eq!(result.model, "test-model");
        assert_eq!(result.engine, "mock");
        assert_eq!(result.samples, 3);
        assert_eq!(result.errors, 0);
    }

    #[test]
    fn test_metrics_keys() {
        let engine = MockEngine::new();
        let b = LatencyBenchmark;
        let result = b.run(&engine, "test-model", 3, 0);
        let expected_keys: std::collections::HashSet<String> = [
            "mean_latency", "p50_latency", "p95_latency",
            "min_latency", "max_latency", "std_latency",
        ].iter().map(|s| s.to_string()).collect();
        let actual_keys: std::collections::HashSet<String> = result.metrics.keys().cloned().collect();
        assert_eq!(actual_keys, expected_keys);
    }

    #[test]
    fn test_sample_count() {
        let engine = MockEngine::new();
        let b = LatencyBenchmark;
        b.run(&engine, "test-model", 5, 0);
        assert_eq!(engine.call_count.load(std::sync::atomic::Ordering::SeqCst), 5);
    }

    #[test]
    fn test_run_with_errors() {
        let engine = MockEngine::failing();
        let b = LatencyBenchmark;
        let result = b.run(&engine, "test-model", 3, 0);
        assert_eq!(result.errors, 3);
        assert!(result.metrics.is_empty());
    }

    #[test]
    fn test_ensure_registered() {
        BENCHMARK_REGISTRY.clear();
        assert!(!BENCHMARK_REGISTRY.contains("latency"));
        ensure_registered();
        assert!(BENCHMARK_REGISTRY.contains("latency"));
    }
}
