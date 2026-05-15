//! Harness configuration and assertion type definitions.

use std::path::PathBuf;
use std::time::Duration;

/// Types of structured assertions supported by the harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssertionType {
    /// Case-insensitive substring match.
    TextContains,
    /// Regex pattern match.
    TextRegex,
    /// Validate JSON output against required keys/types.
    JsonSchema,
    /// Browser DOM selector check.
    DomSelector,
    /// Extract `status: NNN` from output.
    StatusCode,
    /// Fail if latency exceeds threshold (seconds).
    LatencyThreshold,
}

/// A single assertion to validate test results.
#[derive(Debug, Clone)]
pub struct Assertion {
    pub assertion_type: AssertionType,
    /// Expected value (string for text/regex, object for json_schema, number for status/latency).
    pub expected: serde_json::Value,
    /// Human-readable description.
    pub description: String,
    /// If false, failure is a warning not an error.
    pub required: bool,
}

impl Assertion {
    pub fn new(
        assertion_type: AssertionType,
        expected: impl Into<serde_json::Value>,
    ) -> Self {
        Self {
            assertion_type,
            expected: expected.into(),
            description: String::new(),
            required: true,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }
}

/// Configuration for the SkillHarness.
#[derive(Debug, Clone)]
pub struct HarnessConfig {
    pub max_retries: u32,
    pub retry_base_delay: Duration,
    pub retry_max_delay: Duration,
    pub retry_backoff_multiplier: f64,
    pub max_turns: u32,
    pub visual_audit: bool,
    pub parallel_tools: bool,
    pub screenshot_on_pass: bool,
    pub screenshot_on_fail: bool,
    pub latency_baseline_path: Option<PathBuf>,
    pub visual_baseline_path: Option<PathBuf>,
    /// Override for process cleanup command.
    pub process_kill_cmd: Option<String>,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_base_delay: Duration::from_secs(2),
            retry_max_delay: Duration::from_secs(30),
            retry_backoff_multiplier: 2.0,
            max_turns: 10,
            visual_audit: false,
            parallel_tools: false,
            screenshot_on_pass: true,
            screenshot_on_fail: true,
            latency_baseline_path: None,
            visual_baseline_path: None,
            process_kill_cmd: None,
        }
    }
}
