//! Concrete benchmark implementations.

pub mod energy;
pub mod latency;
pub mod throughput;

pub use energy::EnergyBenchmark;
pub use latency::LatencyBenchmark;
pub use throughput::ThroughputBenchmark;

/// Register all benchmarks if not already present.
pub fn ensure_all_registered() {
    latency::ensure_registered();
    throughput::ensure_registered();
    energy::ensure_registered();
}

#[cfg(test)]
mod tests {
    use super::*;
    use sunday_core::registry::BENCHMARK_REGISTRY;

    #[test]
    fn test_registration() {
        // Clear registry first to ensure clean state
        BENCHMARK_REGISTRY.clear();
        assert!(!BENCHMARK_REGISTRY.contains("latency"));
        assert!(!BENCHMARK_REGISTRY.contains("throughput"));
        assert!(!BENCHMARK_REGISTRY.contains("energy"));

        ensure_all_registered();

        assert!(BENCHMARK_REGISTRY.contains("latency"));
        assert!(BENCHMARK_REGISTRY.contains("throughput"));
        assert!(BENCHMARK_REGISTRY.contains("energy"));
    }
}
