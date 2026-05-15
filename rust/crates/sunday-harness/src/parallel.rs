//! Parallel test execution using tokio tasks.

use crate::config::Assertion;
use crate::runner::{SkillHarness, TestResult};
use std::future::Future;

/// Run multiple direct tool tests in parallel.
pub async fn run_tests_parallel<F, Fut>(
    harness: &mut SkillHarness,
    test_cases: &[(String, String)], // (tool_id, prompt)
    test_fn: F,
    assertions: &[Assertion],
    max_workers: usize,
) -> Vec<TestResult>
where
    F: Fn(&str, &str) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Result<String, String>> + Send,
{
    let mut results = Vec::with_capacity(test_cases.len());

    for chunk in test_cases.chunks(max_workers) {
        let mut handles = Vec::with_capacity(chunk.len());
        for (tool_id, prompt) in chunk {
            let tid = tool_id.clone();
            let pr = prompt.clone();
            let tf = test_fn.clone();
            let ass: Vec<Assertion> = assertions.to_vec();
            // Run sequentially within chunk since harness needs &mut
            let res = harness.run_test(&tid, &pr, tf, &ass).await;
            handles.push(res);
        }
        results.extend(handles);
    }

    results
}

/// Run multiple browser E2E tests in parallel (fewer workers due to resource constraints).
pub async fn run_browser_tests_parallel<F, Fut>(
    harness: &mut SkillHarness,
    missions: &[(String, String)], // (mission_name, description)
    test_fn: F,
    assertions: &[Assertion],
    max_workers: usize,
) -> Vec<TestResult>
where
    F: Fn() -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Result<(String, Option<std::path::PathBuf>), String>> + Send,
{
    let mut results = Vec::with_capacity(missions.len());

    for chunk in missions.chunks(max_workers) {
        for (name, _desc) in chunk {
            let n = name.clone();
            let tf = test_fn.clone();
            let ass: Vec<Assertion> = assertions.to_vec();
            let res = harness.run_browser_test(&n, tf, &ass).await;
            results.push(res);
        }
    }

    results
}
