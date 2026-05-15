//! Shared statistics helpers for benchmark per-sample metrics.

use std::collections::HashMap;

/// Compute the p-th percentile via linear interpolation.
pub fn percentile(data: &[f64], p: f64) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let k = (sorted.len() - 1) as f64 * p;
    let f = k as usize;
    let c = f + 1;
    if c >= sorted.len() {
        return sorted[sorted.len() - 1];
    }
    sorted[f] + (k - f as f64) * (sorted[c] - sorted[f])
}

/// Compute mean/p50/p95/min/max/std for a list of per-sample values.
///
/// Returns dict with keys like `mean_{name}`, `p50_{name}`, etc.
/// Returns empty map if *values* is empty.
pub fn compute_stats(name: &str, values: &[f64]) -> HashMap<String, f64> {
    if values.is_empty() {
        return HashMap::new();
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let p50 = percentile(values, 0.50);
    let p95 = percentile(values, 0.95);
    let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let std = if values.len() > 1 {
        let variance = values.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
        variance.sqrt()
    } else {
        0.0
    };

    let mut result = HashMap::new();
    result.insert(format!("mean_{name}"), mean);
    result.insert(format!("p50_{name}"), p50);
    result.insert(format!("p95_{name}"), p95);
    result.insert(format!("min_{name}"), min);
    result.insert(format!("max_{name}"), max);
    result.insert(format!("std_{name}"), std);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile(&data, 0.0), 1.0);
        assert_eq!(percentile(&data, 0.5), 3.0);
        assert_eq!(percentile(&data, 1.0), 5.0);
    }

    #[test]
    fn test_percentile_interpolation() {
        let data = vec![1.0, 2.0, 3.0, 4.0];
        // p=0.5 -> k=1.5 -> 2.0 + 0.5*(3.0-2.0) = 2.5
        assert!((percentile(&data, 0.5) - 2.5).abs() < 1e-9);
    }

    #[test]
    fn test_compute_stats() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = compute_stats("latency", &values);
        assert_eq!(stats["mean_latency"], 3.0);
        assert_eq!(stats["p50_latency"], 3.0);
        assert_eq!(stats["min_latency"], 1.0);
        assert_eq!(stats["max_latency"], 5.0);
        assert!(stats["std_latency"] > 0.0);
    }

    #[test]
    fn test_compute_stats_empty() {
        let stats = compute_stats("latency", &[]);
        assert!(stats.is_empty());
    }

    #[test]
    fn test_compute_stats_single() {
        let stats = compute_stats("latency", &[42.0]);
        assert_eq!(stats["mean_latency"], 42.0);
        assert_eq!(stats["std_latency"], 0.0);
    }
}
