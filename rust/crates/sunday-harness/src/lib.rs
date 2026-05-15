//! SUNDAY Harness — Automated testing, visual regression, and self-healing.
//!
//! Rust port of `src/sunday/harness/` (runner.py, harness_boot.py, test_all.py).

pub mod assertions;
pub mod boot;
pub mod browser;
pub mod config;
pub mod healing;
pub mod parallel;
pub mod performance;
pub mod runner;
pub mod visual;

pub use assertions::{AssertionEngine, AssertionResult};
pub use config::{Assertion, AssertionType};
pub use boot::BootOrchestrator;
pub use config::HarnessConfig;
pub use healing::HealingEngine;
pub use parallel::{run_browser_tests_parallel, run_tests_parallel};
pub use performance::PerformanceTracker;
pub use runner::{SkillHarness, TestResult};
pub use visual::VisualRegressionChecker;
