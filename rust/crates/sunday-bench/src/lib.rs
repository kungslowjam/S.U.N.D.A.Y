//! SUNDAY Benchmarking Framework — inference engine performance measurement.
//!
//! Rust translation of `src/sunday/bench/`.
//! Provides latency, throughput, and energy benchmarks with statistical aggregation.

pub mod benchmarks;
pub mod result;
pub mod stats;
pub mod suite;
pub mod traits;

pub use benchmarks::{ensure_all_registered, LatencyBenchmark, ThroughputBenchmark, EnergyBenchmark};
pub use result::BenchmarkResult;
pub use stats::compute_stats;
pub use suite::BenchmarkSuite;
pub use traits::Benchmark;
