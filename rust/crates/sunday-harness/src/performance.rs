//! Performance baseline tracking — EMA-smoothed latency baselines with regression detection.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Baseline data for a single tool/test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineEntry {
    pub latency_p50: f64,
    pub latency_p95: f64,
    pub count: u64,
    pub last_updated: f64,
}

/// Tracks performance baselines and detects regressions.
pub struct PerformanceTracker {
    baselines: HashMap<String, BaselineEntry>,
    path: PathBuf,
    ema_alpha: f64,
    regression_threshold: f64,
}

impl PerformanceTracker {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        let baselines = if path.exists() {
            Self::load(&path).unwrap_or_default()
        } else {
            HashMap::new()
        };
        Self {
            baselines,
            path,
            ema_alpha: 0.3,
            regression_threshold: 1.5,
        }
    }

    /// Record a latency measurement for a tool.
    pub fn record(&mut self, tool_id: &str, latency: Duration) {
        let secs = latency.as_secs_f64();
        let now = chrono::Utc::now().timestamp() as f64;

        let entry = self.baselines.entry(tool_id.to_string()).or_insert_with(|| BaselineEntry {
            latency_p50: secs,
            latency_p95: secs,
            count: 0,
            last_updated: now,
        });

        // EMA update for p50 (median approximation)
        entry.latency_p50 = Self::ema_update(entry.latency_p50, secs, self.ema_alpha);
        // p95: more aggressive EMA on higher values
        if secs > entry.latency_p95 {
            entry.latency_p95 = Self::ema_update(entry.latency_p95, secs, self.ema_alpha * 1.5);
        }
        entry.count += 1;
        entry.last_updated = now;

        self.save().ok();
    }

    /// Check if latency regresses against baseline.
    /// Returns (regressed, baseline_p50, ratio).
    pub fn check_regression(
        &self,
        tool_id: &str,
        latency: Duration,
    ) -> (bool, Option<f64>, f64) {
        let secs = latency.as_secs_f64();
        match self.baselines.get(tool_id) {
            Some(entry) => {
                let ratio = secs / entry.latency_p50.max(0.001);
                let regressed = ratio > self.regression_threshold;
                (regressed, Some(entry.latency_p50), ratio)
            }
            None => (false, None, 1.0),
        }
    }

    /// Force update baseline for a tool to current measurement.
    pub fn update_baseline(&mut self, tool_id: &str, latency: Duration) {
        let secs = latency.as_secs_f64();
        let now = chrono::Utc::now().timestamp() as f64;
        self.baselines.insert(
            tool_id.to_string(),
            BaselineEntry {
                latency_p50: secs,
                latency_p95: secs,
                count: 1,
                last_updated: now,
            },
        );
        self.save().ok();
    }

    /// Get baseline for a tool.
    pub fn get_baseline(&self, tool_id: &str) -> Option<&BaselineEntry> {
        self.baselines.get(tool_id)
    }

    fn load(path: &Path) -> Result<HashMap<String, BaselineEntry>, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let data: HashMap<String, BaselineEntry> = serde_json::from_str(&content)?;
        Ok(data)
    }

    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(&self.baselines)?;
        std::fs::write(&self.path, content)?;
        Ok(())
    }

    fn ema_update(prev: f64, new: f64, alpha: f64) -> f64 {
        alpha * new + (1.0 - alpha) * prev
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_ema_tracking() {
        let tmp = NamedTempFile::new().unwrap();
        let mut tracker = PerformanceTracker::new(tmp.path());

        tracker.record("test_tool", Duration::from_secs_f64(2.0));
        tracker.record("test_tool", Duration::from_secs_f64(3.0));

        let baseline = tracker.get_baseline("test_tool").unwrap();
        // EMA: 2.0 * 0.7 + 3.0 * 0.3 = 2.3
        assert!((baseline.latency_p50 - 2.3).abs() < 0.01);
        assert_eq!(baseline.count, 2);
    }

    #[test]
    fn test_regression_detection() {
        let tmp = NamedTempFile::new().unwrap();
        let mut tracker = PerformanceTracker::new(tmp.path());

        tracker.record("test_tool", Duration::from_secs_f64(2.0));

        let (regressed, _, ratio) = tracker.check_regression("test_tool", Duration::from_secs_f64(4.0));
        assert!(regressed);
        assert!((ratio - 2.0).abs() < 0.01);

        let (regressed2, _, _) = tracker.check_regression("test_tool", Duration::from_secs_f64(2.5));
        assert!(!regressed2);
    }
}
