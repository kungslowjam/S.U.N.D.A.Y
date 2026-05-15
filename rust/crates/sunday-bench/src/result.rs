//! Benchmark result type — universal container for any benchmark run.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result from running a single benchmark.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkResult {
    pub benchmark_name: String,
    pub model: String,
    pub engine: String,
    #[serde(default)]
    pub metrics: HashMap<String, f64>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub samples: usize,
    #[serde(default)]
    pub errors: usize,
    #[serde(default)]
    pub warmup_samples: usize,
    #[serde(default)]
    pub steady_state_samples: usize,
    #[serde(default)]
    pub steady_state_reached: bool,
    #[serde(default)]
    pub total_energy_joules: f64,
    #[serde(default)]
    pub energy_per_token_joules: f64,
    #[serde(default)]
    pub energy_method: String,
}

impl BenchmarkResult {
    /// Create a minimal result with required fields.
    pub fn new(benchmark_name: impl Into<String>, model: impl Into<String>, engine: impl Into<String>) -> Self {
        Self {
            benchmark_name: benchmark_name.into(),
            model: model.into(),
            engine: engine.into(),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let r = BenchmarkResult::new("test", "m1", "e1");
        assert_eq!(r.benchmark_name, "test");
        assert_eq!(r.model, "m1");
        assert_eq!(r.engine, "e1");
        assert!(r.metrics.is_empty());
        assert!(r.metadata.is_empty());
        assert_eq!(r.samples, 0);
        assert_eq!(r.errors, 0);
    }

    #[test]
    fn test_full() {
        let mut metrics = HashMap::new();
        metrics.insert("mean_latency".into(), 0.5);

        let r = BenchmarkResult {
            benchmark_name: "latency".into(),
            model: "gpt-4".into(),
            engine: "vllm".into(),
            metrics,
            metadata: {
                let mut m = HashMap::new();
                m.insert("note".into(), serde_json::json!("test"));
                m
            },
            samples: 10,
            errors: 1,
            ..BenchmarkResult::new("latency", "gpt-4", "vllm")
        };
        assert_eq!(r.metrics["mean_latency"], 0.5);
        assert_eq!(r.samples, 10);
        assert_eq!(r.errors, 1);
    }
}
