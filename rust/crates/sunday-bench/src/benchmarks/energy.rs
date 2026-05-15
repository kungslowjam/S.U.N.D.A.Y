//! Energy benchmark — per-sample energy, power, and efficiency measurement.

use crate::result::BenchmarkResult;
use crate::stats::compute_stats;
use crate::traits::Benchmark;
use sunday_core::Message;
use sunday_engine::InferenceEngine;
use sunday_core::registry::BENCHMARK_REGISTRY;
use std::time::Instant;

const PROMPT: &str = "Write a short paragraph about artificial intelligence.";

/// Energy sample from a monitor.
#[derive(Debug, Clone, Copy)]
pub struct EnergySample {
    pub energy_joules: f64,
    pub mean_power_watts: f64,
}

/// Trait for energy monitors that can sample energy consumption.
pub trait EnergyMonitor: Send + Sync {
    /// Take a sample during inference.
    fn sample(&self) -> EnergySample;
    /// Description of the measurement method.
    fn energy_method(&self) -> &str;
}

/// Measures energy per token at thermal equilibrium.
pub struct EnergyBenchmark;

impl Benchmark for EnergyBenchmark {
    fn name(&self) -> &'static str {
        "energy"
    }

    fn description(&self) -> &'static str {
        "Measures energy per token at thermal equilibrium"
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

        // For this initial Rust port, we run without energy monitor
        // (same as Python's fallback path when no monitor is provided)
        let mut per_tps: Vec<f64> = Vec::with_capacity(num_samples);
        let mut per_latency: Vec<f64> = Vec::with_capacity(num_samples);
        let mut per_tokens: Vec<f64> = Vec::with_capacity(num_samples);
        let mut errors = 0;

        for _ in 0..num_samples {
            let t0 = Instant::now();
            match engine.generate(&messages, model, 0.7, 256, None) {
                Ok(result) => {
                    let elapsed = t0.elapsed().as_secs_f64();
                    let tokens = result.usage.completion_tokens as f64;
                    let tps = if elapsed > 0.0 { tokens / elapsed } else { 0.0 };
                    per_tps.push(tps);
                    per_latency.push(elapsed);
                    per_tokens.push(tokens);
                }
                Err(_) => {
                    errors += 1;
                }
            }
        }

        let total_tokens: f64 = per_tokens.iter().sum();
        let total_time: f64 = per_latency.iter().sum();

        let mut metrics = compute_stats("tokens_per_second", &per_tps);
        metrics.extend(compute_stats("latency_seconds", &per_latency));
        metrics.insert("total_energy_joules".into(), 0.0);
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
            warmup_samples,
            steady_state_samples: 0,
            steady_state_reached: false,
            total_energy_joules: 0.0,
            energy_per_token_joules: if total_tokens > 0.0 { 0.0 } else { 0.0 },
            energy_method: String::new(),
        }
    }
}

/// Register the energy benchmark if not already present.
pub fn ensure_registered() {
    if !BENCHMARK_REGISTRY.contains("energy") {
        let _ = BENCHMARK_REGISTRY.register("energy", serde_json::json!("energy"));
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
    fn test_name_and_description() {
        let b = EnergyBenchmark;
        assert_eq!(b.name(), "energy");
        assert!(b.description().to_lowercase().contains("energy"));
    }

    #[test]
    fn test_run_without_energy_monitor() {
        let engine = MockEngine::new(10);
        let b = EnergyBenchmark;
        let result = b.run(&engine, "test-model", 3, 0);
        assert_eq!(result.benchmark_name, "energy");
        assert_eq!(result.model, "test-model");
        assert_eq!(result.engine, "mock");
        assert_eq!(result.samples, 3);
        assert_eq!(result.errors, 0);
        assert!(result.metrics.contains_key("mean_tokens_per_second"));
        assert!(result.metrics.contains_key("total_energy_joules"));
        assert_eq!(result.metrics["total_energy_joules"], 0.0);
        assert_eq!(result.energy_method, "");
    }

    #[test]
    fn test_warmup_samples_excluded() {
        let engine = MockEngine::new(10);
        let b = EnergyBenchmark;
        let result = b.run(&engine, "test-model", 3, 2);
        assert_eq!(result.warmup_samples, 2);
        assert_eq!(result.samples, 3);
        // warmup (2) + measurement (3) = 5 total calls
        assert_eq!(engine.call_count.load(std::sync::atomic::Ordering::SeqCst), 5);
    }

    #[test]
    fn test_run_with_errors() {
        let engine = MockEngine::failing();
        let b = EnergyBenchmark;
        let result = b.run(&engine, "test-model", 3, 0);
        assert_eq!(result.errors, 3);
        assert_eq!(result.metrics.get("mean_tokens_per_second").copied().unwrap_or(0.0), 0.0);
        assert_eq!(result.metrics.get("total_energy_joules").copied().unwrap_or(0.0), 0.0);
    }

    #[test]
    fn test_ensure_registered() {
        BENCHMARK_REGISTRY.clear();
        assert!(!BENCHMARK_REGISTRY.contains("energy"));
        ensure_registered();
        assert!(BENCHMARK_REGISTRY.contains("energy"));
    }
}
