//! Assertion engine — evaluates structured assertions against test output.

pub use crate::config::{Assertion, AssertionType};
use serde_json::Value;

/// Result of evaluating a single assertion.
#[derive(Debug, Clone)]
pub struct AssertionResult {
    pub assertion: Assertion,
    pub passed: bool,
    pub actual: Option<Value>,
    pub message: String,
}

/// Engine that evaluates assertions against test output.
pub struct AssertionEngine;

impl AssertionEngine {
    /// Evaluate a single assertion against the given output and latency.
    pub fn evaluate(
        assertion: &Assertion,
        output: &str,
        latency: std::time::Duration,
    ) -> AssertionResult {
        let (passed, actual, message) = match assertion.assertion_type {
            AssertionType::TextContains => {
                let expected = assertion.expected.as_str().unwrap_or("");
                let passed = output.to_lowercase().contains(&expected.to_lowercase());
                (
                    passed,
                    Some(Value::String(output.to_string())),
                    if passed {
                        format!("Output contains '{}'", expected)
                    } else {
                        format!("Output does not contain '{}'", expected)
                    },
                )
            }
            AssertionType::TextRegex => {
                let pattern = assertion.expected.as_str().unwrap_or("");
                let re = match regex::Regex::new(pattern) {
                    Ok(r) => r,
                    Err(e) => {
                        return AssertionResult {
                            assertion: assertion.clone(),
                            passed: false,
                            actual: None,
                            message: format!("Invalid regex: {}", e),
                        }
                    }
                };
                let passed = re.is_match(output);
                (
                    passed,
                    Some(Value::String(output.to_string())),
                    if passed {
                        format!("Output matches regex '{}'", pattern)
                    } else {
                        format!("Output does not match regex '{}'", pattern)
                    },
                )
            }
            AssertionType::JsonSchema => {
                let actual_json: Value = match serde_json::from_str(output) {
                    Ok(v) => v,
                    Err(_) => {
                        return AssertionResult {
                            assertion: assertion.clone(),
                            passed: false,
                            actual: Some(Value::String(output.to_string())),
                            message: "Output is not valid JSON".to_string(),
                        }
                    }
                };
                let passed = Self::check_json_schema(&actual_json, &assertion.expected);
                (
                    passed,
                    Some(actual_json),
                    if passed {
                        "JSON matches expected schema".to_string()
                    } else {
                        "JSON does not match expected schema".to_string()
                    },
                )
            }
            AssertionType::DomSelector => {
                // DOM selector checks are handled during browser execution
                // This is a placeholder for post-hoc evaluation
                (
                    true,
                    None,
                    "DOM selector evaluated during browser execution".to_string(),
                )
            }
            AssertionType::StatusCode => {
                let expected_code = assertion.expected.as_u64().unwrap_or(200) as u16;
                let actual_code = Self::extract_status_code(output);
                let passed = actual_code == Some(expected_code);
                (
                    passed,
                    actual_code.map(|c| Value::Number(c.into())),
                    if passed {
                        format!("Status code matches {}", expected_code)
                    } else {
                        format!(
                            "Expected status {}, got {:?}",
                            expected_code, actual_code
                        )
                    },
                )
            }
            AssertionType::LatencyThreshold => {
                let threshold_secs = assertion.expected.as_f64().unwrap_or(f64::MAX);
                let latency_secs = latency.as_secs_f64();
                let passed = latency_secs <= threshold_secs;
                (
                    passed,
                    Some(Value::Number(serde_json::Number::from_f64(latency_secs).unwrap_or(0.into()))),
                    if passed {
                        format!(
                            "Latency {:.2}s is within threshold {:.2}s",
                            latency_secs, threshold_secs
                        )
                    } else {
                        format!(
                            "Latency {:.2}s exceeds threshold {:.2}s",
                            latency_secs, threshold_secs
                        )
                    },
                )
            }
        };

        AssertionResult {
            assertion: assertion.clone(),
            passed,
            actual,
            message,
        }
    }

    /// Evaluate multiple assertions, returning results for all.
    pub fn evaluate_all(
        assertions: &[Assertion],
        output: &str,
        latency: std::time::Duration,
    ) -> Vec<AssertionResult> {
        assertions
            .iter()
            .map(|a| Self::evaluate(a, output, latency))
            .collect()
    }

    /// Check if all required assertions passed.
    pub fn all_required_passed(results: &[AssertionResult]) -> bool {
        results
            .iter()
            .all(|r| !r.assertion.required || r.passed)
    }

    /// Extract status code like `status: 200` from output text.
    fn extract_status_code(output: &str) -> Option<u16> {
        let re = regex::Regex::new(r"status:\s*(\d+)").ok()?;
        re.captures(output)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok())
    }

    /// Validate that actual JSON contains all expected keys (shallow check).
    fn check_json_schema(actual: &Value, expected: &Value) -> bool {
        match expected {
            Value::Object(expected_map) => {
                let actual_map = match actual.as_object() {
                    Some(m) => m,
                    None => return false,
                };
                for (key, expected_val) in expected_map {
                    let actual_val = match actual_map.get(key) {
                        Some(v) => v,
                        None => return false,
                    };
                    if !Self::values_compatible(actual_val, expected_val) {
                        return false;
                    }
                }
                true
            }
            _ => true,
        }
    }

    /// Check if actual value is compatible with expected type hint.
    fn values_compatible(actual: &Value, expected: &Value) -> bool {
        match expected {
            Value::String(type_hint) => match type_hint.as_str() {
                "string" => actual.is_string(),
                "number" => actual.is_number(),
                "boolean" => actual.is_boolean(),
                "array" => actual.is_array(),
                "object" => actual.is_object(),
                "null" => actual.is_null(),
                _ => true, // Unknown hint, accept anything
            },
            Value::Object(_) => Self::check_json_schema(actual, expected),
            _ => actual == expected,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_contains() {
        let a = Assertion::new(AssertionType::TextContains, "hello");
        let r = AssertionEngine::evaluate(&a, "Hello World", Duration::from_secs(1));
        assert!(r.passed);

        let r2 = AssertionEngine::evaluate(&a, "Goodbye", Duration::from_secs(1));
        assert!(!r2.passed);
    }

    #[test]
    fn test_latency_threshold() {
        let a = Assertion::new(AssertionType::LatencyThreshold, 2.0);
        let r = AssertionEngine::evaluate(&a, "ok", Duration::from_millis(1500));
        assert!(r.passed);

        let r2 = AssertionEngine::evaluate(&a, "ok", Duration::from_millis(2500));
        assert!(!r2.passed);
    }

    #[test]
    fn test_json_schema() {
        let expected = serde_json::json!({
            "name": "string",
            "count": "number"
        });
        let a = Assertion::new(AssertionType::JsonSchema, expected);
        let r = AssertionEngine::evaluate(&a, r#"{"name":"test","count":42}"#, Duration::from_secs(1));
        assert!(r.passed);

        let r2 = AssertionEngine::evaluate(&a, r#"{"name":"test","count":"not_a_number"}"#, Duration::from_secs(1));
        assert!(!r2.passed);
    }
}
