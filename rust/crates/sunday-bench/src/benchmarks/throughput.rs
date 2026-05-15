//! Throughput benchmark — measures tokens per second.

use crate::result::BenchmarkResult;
use crate::stats::compute_stats;
use crate::traits::Benchmark;
use sunday_core::Message;
use sunday_engine::InferenceEngine;
use sunday_core::registry::BENCHMARK_REGISTRY;
use std::time::Instant;

const PROMPT: &str = "Write a short paragraph about artificial intelligence.";

/// Measures inference throughput in tokens per second.
pub struct ThroughputBenchmark;

impl Benchmark for ThroughputBenchmark {
    fn name(&self) -> &'static str {
        "throughput"
    }

    fn description(&self) -> &'static str {
        "Measures inference throughput in tokens per second"
    }

    fn run(
        &self,
        engine: &dyn InferenceEngine,
        model: &str,
        num_samples: usize,
        warmup_samples: usize,
    ) -> BenchmarkResult {
        let messages = vec![Message::user(PROMPT)];

        // Warmup
        for _ in 0..warmup_samples {
            let _ = engine.generate(&messages, model, 0.7, 256, None);
        }

        let mut per_sample_tps: Vec<f64> = Vec::with_capacity(num_samples);
        let mut per_sample_tokens: Vec<f64> = Vec::with_capacity(num_samples);
        let mut per_sample_latency: Vec<f64> = Vec::with_capacity(num_samples);
        let mut errors = 0;

        for _ in 0..num_samples {
            let t0 = Instant::now();
            match engine.generate(&messages, model, 0.7, 256, None) {
                Ok(result) => {
                    let elapsed = t0.elapsed().as_secs_f64();
                    let tokens = result.usage.completion_tokens as f64;
                    let tps = if elapsed > 0.0 { tokens / elapsed } else { 0.0 };
                    per_sample_tps.push(tps);
                    per_sample_tokens.push(tokens);
                    per_sample_latency.push(elapsed);
                }
                Err(_) => {
                    errors += 1;
                }
            }
        }

        let total_tokens: f64 = per_sample_tokens.iter().sum();
        let total_time: f64 = per_sample_latency.iter().sum();

        let mut metrics = compute_stats("tokens_per_second", &per_sample_tps);
        metrics.extend(compute_stats("latency_seconds", &per_sample_latency));
        metrics.insert("total_tokens".into(), total_tokens);
        metrics.insert("total_time_seconds".into(), total_time);

        let mut metadata = std::collections::HashMap::new();
        if engine.engine_id() == "apple_fm" {
            metadata.insert(
                "token_estimation".into(),
                serde_json::json!("~4 chars/token (Apple FM SDK does not expose counts)"),
            );
        }

        BenchmarkResult {
            benchmark_name: self.name().into(),
            model: model.into(),
            engine: engine.engine_id().into(),
            metrics,
            metadata,
            samples: num_samples,
            errors,
            ..BenchmarkResult::new(self.name(), model, engine.engine_id())
        }
    }
}

/// Register the throughput benchmark if not already present.
pub fn ensure_registered() {
    if !BENCHMARK_REGISTRY.contains("throughput") {
        let _ = BENCHMARK_REGISTRY.register("throughput", serde_json::json!("throughput"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sunday_core::{GenerateResult, Message, Usage};

    struct MockEngine {
        completion_tokens: i64,
        fail: bool,
        call_count: std::sync::atomic::AtomicUsize,
    }

    impl MockEngine {
        fn new(completion_tokens: i64) -> Self {
            Self {
                completion_tokens,
                fail: false,
                call_count: std::sync::atomic::AtomicUsize::new(0),
            }
        }

        fn failing() -> Self {
            Self {
                completion_tokens: 0,
                fail: true,
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
            self.call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if self.fail {
                return Err(sunday_core::SUNDAYError::Engine(sunday_core::EngineError::Generation("fail".into())));
            }
            Ok(GenerateResult {
                content: "Hello world".into(),
                usage: Usage {
                    prompt_tokens: 5,
                    completion_tokens: self.completion_tokens,
                    total_tokens: 5 + self.completion_tokens,
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
    fn test_run_with_mock() {
        let engine = MockEngine::new(10);
        let b = ThroughputBenchmark;
        let result = b.run(&engine, "test-model", 3, 0);
        assert_eq!(result.benchmark_name, "throughput");
        assert_eq!(result.model, "test-model");
        assert_eq!(result.engine, "mock");
        assert_eq!(result.samples, 3);
    }

    #[test]
    fn test_metrics_keys() {
        let engine = MockEngine::new(10);
        let b = ThroughputBenchmark;
        let result = b.run(&engine, "test-model", 3, 0);
        assert!(result.metrics.contains_key("mean_tokens_per_second"));
        assert!(result.metrics.contains_key("total_tokens"));
        assert!(result.metrics.contains_key("total_time_seconds"));
    }

    #[test]
    fn test_tokens_per_second_calc() {
        let engine = MockEngine::new(10);
        let b = ThroughputBenchmark;
        let result = b.run(&engine, "test-model", 5, 0);
        // 5 samples * 10 tokens each = 50 total tokens
        assert_eq!(result.metrics["total_tokens"], 50.0);
        assert!(result.metrics["mean_tokens_per_second"] > 0.0);
    }

    #[test]
    fn test_sample_count() {
        let engine = MockEngine::new(10);
        let b = ThroughputBenchmark;
        b.run(&engine, "test-model", 7, 0);
        assert_eq!(engine.call_count.load(std::sync::atomic::Ordering::SeqCst), 7);
    }

    #[test]
    fn test_zero_latency_handling() {
        let engine = MockEngine::failing();
        let b = ThroughputBenchmark;
        let result = b.run(&engine, "test-model", 3, 0);
        assert_eq!(result.errors, 3);
        assert_eq!(result.metrics.get("mean_tokens_per_second").copied().unwrap_or(0.0), 0.0);
    }

    #[test]
    fn test_ensure_registered() {
        BENCHMARK_REGISTRY.clear();
        assert!(!BENCHMARK_REGISTRY.contains("throughput"));
        ensure_registered();
        assert!(BENCHMARK_REGISTRY.contains("throughput"));
    }
}
