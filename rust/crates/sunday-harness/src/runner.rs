//! Core harness runner — `SkillHarness` with retry logic and test execution.

use crate::assertions::{Assertion, AssertionEngine, AssertionResult};
use crate::config::HarnessConfig;
use crate::healing::HealingEngine;
use crate::performance::PerformanceTracker;
use crate::visual::VisualRegressionChecker;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Result of a single test run.
#[derive(Debug, Clone)]
pub struct TestResult {
    pub tool_id: String,
    pub prompt: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub latency: Duration,
    pub visual_evidence: Option<PathBuf>,
    pub assertion_results: Vec<AssertionResult>,
    pub retry_count: u32,
    pub performance_baseline: Option<f64>,
}

impl TestResult {
    pub fn failed(tool_id: &str, prompt: &str, error: impl Into<String>) -> Self {
        Self {
            tool_id: tool_id.to_string(),
            prompt: prompt.to_string(),
            success: false,
            output: String::new(),
            error: Some(error.into()),
            latency: Duration::ZERO,
            visual_evidence: None,
            assertion_results: vec![],
            retry_count: 0,
            performance_baseline: None,
        }
    }
}

/// The core harness engine that runs tests with retry, assertions, and healing.
pub struct SkillHarness {
    config: HarnessConfig,
    visual_checker: Option<VisualRegressionChecker>,
    perf_tracker: Option<PerformanceTracker>,
    healing: HealingEngine,
}

impl SkillHarness {
    pub fn new(config: HarnessConfig) -> Self {
        let visual_checker = config.visual_baseline_path.as_ref().map(|p| {
            VisualRegressionChecker::new(p, "harness-stress-test/screenshots")
        });
        let perf_tracker = config.latency_baseline_path.as_ref().map(|p| {
            PerformanceTracker::new(p)
        });
        Self {
            config,
            visual_checker,
            perf_tracker,
            healing: HealingEngine::new(),
        }
    }

    /// Run a single test against a tool with retry logic.
    pub async fn run_test<F, Fut>(
        &mut self,
        tool_id: &str,
        prompt: &str,
        test_fn: F,
        assertions: &[Assertion],
    ) -> TestResult
    where
        F: Fn(&str, &str) -> Fut,
        Fut: std::future::Future<Output = Result<String, String>>,
    {
        let mut last_error = None;
        let mut retry_count = 0u32;
        let start = Instant::now();

        for attempt in 0..=self.config.max_retries {
            retry_count = attempt;
            let attempt_start = Instant::now();

            match test_fn(tool_id, prompt).await {
                Ok(output) => {
                    let latency = attempt_start.elapsed();

                    // Evaluate assertions
                    let assertion_results = AssertionEngine::evaluate_all(assertions, &output, latency);
                    let assertions_passed = AssertionEngine::all_required_passed(&assertion_results);

                    // Record performance
                    if let Some(ref mut tracker) = self.perf_tracker {
                        tracker.record(tool_id, latency);
                    }

                    let success = assertions_passed;
                    let result = TestResult {
                        tool_id: tool_id.to_string(),
                        prompt: prompt.to_string(),
                        success,
                        output,
                        error: None,
                        latency: start.elapsed(),
                        visual_evidence: None,
                        assertion_results,
                        retry_count,
                        performance_baseline: self.perf_tracker.as_ref()
                            .and_then(|t| t.get_baseline(tool_id).map(|b| b.latency_p50)),
                    };

                    if !success {
                        tracing::warn!(
                            "Test '{}' failed assertions after {} retries",
                            tool_id, retry_count
                        );
                        self.healing.heal(&result);
                    }

                    return result;
                }
                Err(e) => {
                    last_error = Some(e);
                    tracing::warn!(
                        "Test '{}' attempt {}/{} failed: {}",
                        tool_id, attempt + 1, self.config.max_retries + 1,
                        last_error.as_ref().unwrap()
                    );

                    if attempt < self.config.max_retries {
                        let delay = self.compute_backoff(attempt);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        // All retries exhausted
        let result = TestResult {
            tool_id: tool_id.to_string(),
            prompt: prompt.to_string(),
            success: false,
            output: String::new(),
            error: last_error,
            latency: start.elapsed(),
            visual_evidence: None,
            assertion_results: vec![],
            retry_count,
            performance_baseline: None,
        };

        self.healing.heal(&result);
        result
    }

    /// Run a browser E2E test.
    pub async fn run_browser_test<F, Fut>(
        &mut self,
        mission_name: &str,
        test_fn: F,
        assertions: &[Assertion],
    ) -> TestResult
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<(String, Option<PathBuf>), String>>,
    {
        let start = Instant::now();
        let mut last_error = None;
        let mut retry_count = 0u32;

        for attempt in 0..=self.config.max_retries {
            retry_count = attempt;
            let attempt_start = Instant::now();

            match test_fn().await {
                Ok((output, screenshot)) => {
                    let latency = attempt_start.elapsed();
                    let assertion_results = AssertionEngine::evaluate_all(assertions, &output, latency);
                    let success = AssertionEngine::all_required_passed(&assertion_results);

                    if let Some(ref mut tracker) = self.perf_tracker {
                        tracker.record(mission_name, latency);
                    }

                    // Visual regression check
                    let visual_evidence = if let (Some(ref checker), Some(ref ss_path)) =
                        (&self.visual_checker, &screenshot)
                    {
                        match checker.compare_against_baseline(mission_name, ss_path) {
                            Ok((ssim, regressed)) => {
                                if regressed {
                                    tracing::warn!(
                                        "Visual regression in '{}': SSIM={:.4}",
                                        mission_name, ssim
                                    );
                                }
                            }
                            Err(e) => tracing::error!("Visual check failed: {}", e),
                        }
                        Some(ss_path.clone())
                    } else {
                        screenshot
                    };

                    let result = TestResult {
                        tool_id: mission_name.to_string(),
                        prompt: String::new(),
                        success,
                        output,
                        error: None,
                        latency: start.elapsed(),
                        visual_evidence,
                        assertion_results,
                        retry_count,
                        performance_baseline: self.perf_tracker.as_ref()
                            .and_then(|t| t.get_baseline(mission_name).map(|b| b.latency_p50)),
                    };

                    if !success {
                        self.healing.heal(&result);
                    }

                    return result;
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries {
                        let delay = self.compute_backoff(attempt);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        let result = TestResult {
            tool_id: mission_name.to_string(),
            prompt: String::new(),
            success: false,
            output: String::new(),
            error: last_error,
            latency: start.elapsed(),
            visual_evidence: None,
            assertion_results: vec![],
            retry_count,
            performance_baseline: None,
        };

        self.healing.heal(&result);
        result
    }

    /// Compute exponential backoff delay.
    fn compute_backoff(&self, attempt: u32) -> Duration {
        let delay_secs = self.config.retry_base_delay.as_secs_f64()
            * self.config.retry_backoff_multiplier.powi(attempt as i32);
        let clamped = delay_secs.min(self.config.retry_max_delay.as_secs_f64());
        Duration::from_secs_f64(clamped)
    }

    /// Generate a markdown report from test results.
    pub fn generate_report(results: &[TestResult]) -> String {
        let mut report = String::from("# SUNDAY Harness Test Report\n\n");
        report.push_str(&format!("| Tool | Status | Latency | Retries | Assertions |\n"));
        report.push_str(&format!("|------|--------|---------|---------|------------|\n"));

        for r in results {
            let status = if r.success { "✅ PASS" } else { "❌ FAIL" };
            let assertions = r.assertion_results.iter()
                .map(|a| if a.passed { "✓" } else { "✗" })
                .collect::<String>();
            report.push_str(&format!(
                "| {} | {} | {:.2}s | {} | {} |\n",
                r.tool_id, status, r.latency.as_secs_f64(), r.retry_count, assertions
            ));
        }

        let passed = results.iter().filter(|r| r.success).count();
        let failed = results.len() - passed;
        report.push_str(&format!("\n**Summary**: {} passed, {} failed\n", passed, failed));

        report
    }
}
