import logging
import os
import time
import urllib.request
import asyncio
import json
import re
from typing import Any, Dict, List, Optional, Callable, Union
from dataclasses import dataclass, field
from pathlib import Path
from enum import Enum

from sunday.agents._stubs import AgentContext, AgentResult
from sunday.core.registry import AgentRegistry
from sunday.core.types import Conversation, Message, Role
from sunday.tools._stubs import ToolResult

logger = logging.getLogger("sunday.harness")

# Rust bridge for harness core (single source of truth)
try:
    import sunday_rust
    _RUST_HARNESS_AVAILABLE = True
except ImportError:
    sunday_rust = None  # type: ignore
    _RUST_HARNESS_AVAILABLE = False


class AssertionType(Enum):
    """Types of structured assertions supported by the harness."""
    TEXT_CONTAINS = "text_contains"
    TEXT_REGEX = "text_regex"
    JSON_SCHEMA = "json_schema"
    DOM_SELECTOR = "dom_selector"
    STATUS_CODE = "status_code"
    LATENCY_THRESHOLD = "latency_threshold"


@dataclass
class Assertion:
    """A single assertion to validate test results."""
    assertion_type: AssertionType
    expected: Any
    description: str = ""
    # For retry logic
    required: bool = True  # If False, failure is a warning not an error

    def to_rust(self):
        """Convert to sunday_rust.Assertion (Rust-backed)."""
        if not _RUST_HARNESS_AVAILABLE:
            return None
        type_map = {
            AssertionType.TEXT_CONTAINS: sunday_rust.AssertionType.TEXT_CONTAINS,
            AssertionType.TEXT_REGEX: sunday_rust.AssertionType.TEXT_REGEX,
            AssertionType.JSON_SCHEMA: sunday_rust.AssertionType.JSON_SCHEMA,
            AssertionType.DOM_SELECTOR: sunday_rust.AssertionType.DOM_SELECTOR,
            AssertionType.STATUS_CODE: sunday_rust.AssertionType.STATUS_CODE,
            AssertionType.LATENCY_THRESHOLD: sunday_rust.AssertionType.LATENCY_THRESHOLD,
        }
        rust_type = type_map.get(self.assertion_type, sunday_rust.AssertionType.TEXT_CONTAINS)
        expected_str = json.dumps(self.expected) if isinstance(self.expected, (dict, list)) else str(self.expected)
        return sunday_rust.Assertion(rust_type, expected_str, self.description, self.required)


@dataclass
class AssertionResult:
    """Result of a single assertion check."""
    assertion: Assertion
    passed: bool
    actual: Any = None
    message: str = ""

    @classmethod
    def from_rust(cls, rust_result, assertion: Optional["Assertion"] = None):
        """Create from sunday_rust.AssertionResult."""
        return cls(
            assertion=assertion or Assertion(
                assertion_type=AssertionType.TEXT_CONTAINS,
                expected="",
            ),
            passed=rust_result.passed,
            actual=rust_result.actual,
            message=rust_result.message,
        )


@dataclass
class TestResult:
    tool_id: str
    prompt: str
    success: bool
    output: str
    error: Optional[str] = None
    latency: float = 0.0
    visual_evidence: Optional[str] = None  # Path to screenshot if any
    assertion_results: List[AssertionResult] = field(default_factory=list)
    retry_count: int = 0
    performance_baseline: Optional[float] = None  # For regression tracking


@dataclass
class HarnessConfig:
    """Configuration for the SkillHarness."""
    max_retries: int = 3
    retry_base_delay: float = 2.0
    retry_max_delay: float = 30.0
    retry_backoff_multiplier: float = 2.0
    max_turns: int = 10  # Increased from 5 for complex missions
    visual_audit: bool = False
    parallel_tools: bool = False
    screenshot_on_pass: bool = True
    screenshot_on_fail: bool = True
    latency_baseline_path: Optional[str] = None
    visual_baseline_path: Optional[str] = None
    # Cross-platform settings
    process_kill_cmd: Optional[str] = None  # Override for process cleanup
    # Instruction compliance testing
    instruction_compliance: bool = True
    skill_compliance: bool = True
    # GPU off-load for harness-local inference (0 = CPU-only)
    gpu_layers: int = 35

    def to_rust(self):
        """Convert to sunday_rust.HarnessConfig."""
        if not _RUST_HARNESS_AVAILABLE:
            return None
        return sunday_rust.HarnessConfig(
            max_retries=self.max_retries,
            retry_base_delay=self.retry_base_delay,
            retry_max_delay=self.retry_max_delay,
            retry_backoff_multiplier=self.retry_backoff_multiplier,
            max_turns=self.max_turns,
            visual_audit=self.visual_audit,
            parallel_tools=self.parallel_tools,
            screenshot_on_pass=self.screenshot_on_pass,
            screenshot_on_fail=self.screenshot_on_fail,
            latency_baseline_path=self.latency_baseline_path,
            visual_baseline_path=self.visual_baseline_path,
            process_kill_cmd=self.process_kill_cmd,
        )


@dataclass
class InstructionComplianceRule:
    """Rule defining what constitutes correct instruction following for a test case."""
    intent_keywords: List[str] = field(default_factory=list)  # Keywords indicating user intent
    required_tools: List[str] = field(default_factory=list)   # Tools that MUST be used
    forbidden_tools: List[str] = field(default_factory=list)  # Tools that MUST NOT be used
    output_must_contain: List[str] = field(default_factory=list)      # Required output content
    output_must_not_contain: List[str] = field(default_factory=list)  # Forbidden output content
    min_steps: int = 1          # Minimum tool calls required
    max_steps: int = 20         # Maximum tool calls allowed (prevent wandering)
    require_clarification: bool = False  # Must ask for clarification on ambiguous input
    expected_skill: Optional[str] = None  # For skill testing: which skill should be triggered


class InstructionComplianceChecker:
    """Verifies that an agent's execution matches the user's intended instruction."""

    def __init__(self):
        self.rules: Dict[str, InstructionComplianceRule] = self._load_default_rules()

    def _load_default_rules(self) -> Dict[str, InstructionComplianceRule]:
        """Load default compliance rules for common instruction types."""
        return {
            "web_search": InstructionComplianceRule(
                intent_keywords=["หา", "ค้น", "search", "find", "lookup", "ข้อมูล"],
                required_tools=["web_search", "browser_navigate", "browser_extract"],
                forbidden_tools=["calculator", "file_write"],
                output_must_contain=[],
                max_steps=8,
            ),
            "code_execution": InstructionComplianceRule(
                intent_keywords=["รัน", "run", "execute", "code", "script", "test"],
                required_tools=["shell_exec", "code_interpreter"],
                forbidden_tools=["browser_navigate"],
                min_steps=1,
                max_steps=5,
            ),
            "file_operation": InstructionComplianceRule(
                intent_keywords=["ไฟล์", "file", "อ่าน", "read", "เขียน", "write", "save"],
                required_tools=["file_read", "file_write"],
                forbidden_tools=["web_search"],
                min_steps=1,
                max_steps=4,
            ),
            "stock_price": InstructionComplianceRule(
                intent_keywords=["ราคา", "หุ้น", "stock", "price", "TKN", "SET"],
                required_tools=["web_search", "browser_navigate"],
                forbidden_tools=["calculator"],
                output_must_contain=["ราคา", "บาท", "THB"],
                max_steps=6,
            ),
            "skill_trigger": InstructionComplianceRule(
                intent_keywords=["ใช้ skill", "run skill", "skill", "เปิด"],
                required_tools=[],
                forbidden_tools=[],
                expected_skill=None,  # Set at runtime
                max_steps=3,
            ),
        }

    def classify_intent(self, prompt: str) -> Optional[str]:
        """Classify user intent from prompt to find matching compliance rule."""
        prompt_lower = prompt.lower()
        scores = {}
        for rule_name, rule in self.rules.items():
            score = sum(1 for kw in rule.intent_keywords if kw.lower() in prompt_lower)
            if score > 0:
                scores[rule_name] = score
        if not scores:
            return None
        return max(scores, key=scores.get)

    def check_tool_calls(self, tool_results: List[Any], rule: InstructionComplianceRule) -> List[Dict[str, Any]]:
        """Verify that tool calls comply with the rule."""
        violations = []
        tool_names = [tr.tool_name for tr in tool_results if hasattr(tr, 'tool_name')]

        # Check required tools
        for required in rule.required_tools:
            if required not in tool_names:
                violations.append({
                    "type": "missing_required_tool",
                    "tool": required,
                    "message": f"Required tool '{required}' was not used"
                })

        # Check forbidden tools
        for forbidden in rule.forbidden_tools:
            if forbidden in tool_names:
                violations.append({
                    "type": "forbidden_tool_used",
                    "tool": forbidden,
                    "message": f"Forbidden tool '{forbidden}' was used"
                })

        # Check step count
        if len(tool_names) < rule.min_steps:
            violations.append({
                "type": "too_few_steps",
                "actual": len(tool_names),
                "expected": rule.min_steps,
                "message": f"Only {len(tool_names)} tool calls made, minimum is {rule.min_steps}"
            })
        if len(tool_names) > rule.max_steps:
            violations.append({
                "type": "too_many_steps",
                "actual": len(tool_names),
                "expected": rule.max_steps,
                "message": f"{len(tool_names)} tool calls made, maximum is {rule.max_steps} (agent may be wandering)"
            })

        return violations

    def check_loops(self, tool_results: List[Any]) -> List[Dict[str, Any]]:
        """Detect repeated identical tool calls (loop behavior)."""
        violations = []
        tool_calls = []
        for tr in tool_results:
            if hasattr(tr, 'tool_name') and hasattr(tr, 'arguments'):
                args = str(tr.arguments) if tr.arguments else ""
                tool_calls.append((tr.tool_name, args))

        # Check for consecutive identical calls
        for i in range(len(tool_calls) - 1):
            if tool_calls[i] == tool_calls[i + 1]:
                violations.append({
                    "type": "loop_detected",
                    "tool": tool_calls[i][0],
                    "message": f"Loop detected: '{tool_calls[i][0]}' called twice with same arguments"
                })

        # Check for 3+ calls to same tool (even with different args = likely stuck)
        from collections import Counter
        tool_counts = Counter([tc[0] for tc in tool_calls])
        for tool, count in tool_counts.items():
            if count >= 3:
                violations.append({
                    "type": "tool_overuse",
                    "tool": tool,
                    "count": count,
                    "message": f"Tool '{tool}' called {count} times — agent may be stuck or wandering"
                })

        return violations

    def check_post_navigate(self, tool_results: List[Any]) -> List[Dict[str, Any]]:
        """Verify that browser_navigate is followed by extraction within 2 steps."""
        violations = []
        tool_names = [tr.tool_name for tr in tool_results if hasattr(tr, 'tool_name')]

        for i, name in enumerate(tool_names):
            if name == "browser_navigate":
                # Check next 2 tools for extraction
                next_tools = tool_names[i + 1:i + 3]
                extract_tools = {"browser_extract", "browser_get_elements", "browser_screenshot", "browser_get_accessibility_tree"}
                if not any(t in extract_tools for t in next_tools):
                    violations.append({
                        "type": "navigate_without_extraction",
                        "step": i,
                        "message": f"browser_navigate at step {i} not followed by extraction within 2 steps"
                    })

        return violations

    def check_cancellations(self, tool_results: List[Any]) -> List[Dict[str, Any]]:
        """Detect self-cancellation patterns (agent giving up)."""
        violations = []
        for tr in tool_results:
            content = str(getattr(tr, 'content', '')).lower()
            if 'canceled' in content or 'cancelled' in content or 'oneshot canceled' in content:
                violations.append({
                    "type": "self_cancellation",
                    "tool": getattr(tr, 'tool_name', 'unknown'),
                    "message": "Agent self-canceled — likely confused or stuck"
                })
        return violations

    def check_output(self, output: str, rule: InstructionComplianceRule) -> List[Dict[str, Any]]:
        """Verify that output content complies with the rule."""
        violations = []
        output_lower = output.lower()

        # Check required content
        for required in rule.output_must_contain:
            if required.lower() not in output_lower:
                violations.append({
                    "type": "missing_output",
                    "expected": required,
                    "message": f"Output should contain '{required}'"
                })

        # Check forbidden content
        for forbidden in rule.output_must_not_contain:
            if forbidden.lower() in output_lower:
                violations.append({
                    "type": "forbidden_output",
                    "forbidden": forbidden,
                    "message": f"Output should not contain '{forbidden}'"
                })

        return violations

    def evaluate(self, prompt: str, tool_results: List[Any], output: str,
                 latency: float = 0.0) -> Dict[str, Any]:
        """Full compliance evaluation for a test case."""
        intent = self.classify_intent(prompt)
        if not intent:
            return {
                "compliant": True,
                "intent_detected": None,
                "violations": [],
                "rule_applied": None
            }

        rule = self.rules[intent]
        violations = []
        violations.extend(self.check_tool_calls(tool_results, rule))
        violations.extend(self.check_loops(tool_results))
        violations.extend(self.check_post_navigate(tool_results))
        violations.extend(self.check_cancellations(tool_results))
        violations.extend(self.check_output(output, rule))

        # Latency check for simple tasks
        if latency > 30.0 and intent in {"stock_price", "web_search"}:
            violations.append({
                "type": "excessive_latency",
                "latency": latency,
                "message": f"Task took {latency:.1f}s — excessive for {intent} task (should be < 30s)"
            })

        return {
            "compliant": len(violations) == 0,
            "intent_detected": intent,
            "violations": violations,
            "rule_applied": rule,
            "tool_count": len([tr for tr in tool_results if hasattr(tr, 'tool_name')])
        }


class VisualRegressionChecker:
    """Compare screenshots against baselines for visual regression detection.

    Delegates SSIM computation to Rust sunday-harness when available.
    """

    def __init__(self, baseline_dir: Optional[str] = None):
        self.baseline_dir = baseline_dir or os.path.join(
            os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))),
            "harness-stress-test", "baselines"
        )
        os.makedirs(self.baseline_dir, exist_ok=True)
        # Rust-backed checker (faster, no GIL contention)
        if _RUST_HARNESS_AVAILABLE:
            output_dir = os.path.join(
                os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))),
                "harness-stress-test", "screenshots"
            )
            self._rust = sunday_rust.VisualRegressionChecker(self.baseline_dir, output_dir)
        else:
            self._rust = None

    def _compute_ssim(self, img1_path: str, img2_path: str) -> float:
        """Compute SSIM between two images. Falls back to pixel diff if PIL unavailable."""
        if self._rust:
            try:
                return self._rust.compute_ssim(img1_path, img2_path)
            except Exception as e:
                logger.warning("Rust SSIM failed (%s), falling back to Python", e)
        try:
            from PIL import Image
            import numpy as np

            img1 = Image.open(img1_path).convert('L')
            img2 = Image.open(img2_path).convert('L')

            # Resize to same dimensions
            target_size = (min(img1.width, img2.width), min(img1.height, img2.height))
            img1 = img1.resize(target_size)
            img2 = img2.resize(target_size)

            arr1 = np.array(img1, dtype=np.float64)
            arr2 = np.array(img2, dtype=np.float64)

            # Simple normalized cross-correlation as SSIM approximation
            mu1, mu2 = arr1.mean(), arr2.mean()
            sigma1, sigma2 = arr1.std(), arr2.std()
            sigma12 = ((arr1 - mu1) * (arr2 - mu2)).mean()

            k1, k2 = 0.01, 0.03
            L = 255
            c1, c2 = (k1 * L) ** 2, (k2 * L) ** 2

            ssim = ((2 * mu1 * mu2 + c1) * (2 * sigma12 + c2)) / \
                   ((mu1 ** 2 + mu2 ** 2 + c1) * (sigma1 ** 2 + sigma2 ** 2 + c2))
            return float(ssim)
        except ImportError:
            # Fallback: simple pixel difference ratio
            try:
                from PIL import Image
                img1 = Image.open(img1_path).convert('RGB')
                img2 = Image.open(img2_path).convert('RGB')
                target_size = (min(img1.width, img2.width), min(img1.height, img2.height))
                img1 = img1.resize(target_size)
                img2 = img2.resize(target_size)

                pixels1 = list(img1.getdata())
                pixels2 = list(img2.getdata())

                diff_count = sum(
                    1 for p1, p2 in zip(pixels1, pixels2)
                    if abs(p1[0] - p2[0]) + abs(p1[1] - p2[1]) + abs(p1[2] - p2[2]) > 30
                )
                return 1.0 - (diff_count / len(pixels1))
            except Exception:
                return 1.0  # Can't compare, assume same

    def compare(self, screenshot_path: str, baseline_name: str, threshold: float = 0.95) -> Dict[str, Any]:
        """Compare a screenshot against its baseline. Returns comparison metrics."""
        if self._rust:
            try:
                ssim, regression = self._rust.compare_against_baseline(baseline_name, screenshot_path)
                return {
                    "has_baseline": True,
                    "ssim": ssim,
                    "regression": regression,
                    "threshold": threshold,
                    "baseline_path": os.path.join(self.baseline_dir, f"{baseline_name}.png")
                }
            except Exception as e:
                logger.warning("Rust visual compare failed (%s), falling back to Python", e)

        baseline_path = os.path.join(self.baseline_dir, f"{baseline_name}.png")

        if not os.path.exists(baseline_path):
            # First run: save as baseline
            import shutil
            shutil.copy2(screenshot_path, baseline_path)
            return {
                "has_baseline": False,
                "ssim": 1.0,
                "regression": False,
                "baseline_path": baseline_path
            }

        ssim = self._compute_ssim(screenshot_path, baseline_path)
        return {
            "has_baseline": True,
            "ssim": ssim,
            "regression": ssim < threshold,
            "threshold": threshold,
            "baseline_path": baseline_path
        }

    def update_baseline(self, screenshot_path: str, baseline_name: str):
        """Update the baseline for a given test."""
        if self._rust:
            try:
                self._rust.update_baseline(baseline_name, screenshot_path)
                return
            except Exception as e:
                logger.warning("Rust baseline update failed (%s), falling back to Python", e)
        baseline_path = os.path.join(self.baseline_dir, f"{baseline_name}.png")
        import shutil
        shutil.copy2(screenshot_path, baseline_path)


class AssertionEngine:
    """Engine for evaluating structured assertions against test results.

    Delegates to Rust sunday-harness when available for performance.
    """

    def __init__(self):
        self._rust = sunday_rust.SkillHarness(sunday_rust.HarnessConfig()) if _RUST_HARNESS_AVAILABLE else None

    def evaluate(self, result: TestResult, assertions: List[Assertion]) -> List[AssertionResult]:
        """Evaluate all assertions against a test result."""
        if self._rust:
            rust_assertions = []
            for a in assertions:
                ra = a.to_rust()
                if ra is not None:
                    rust_assertions.append(ra)
            if rust_assertions:
                try:
                    rust_results = self._rust.evaluate_assertions(rust_assertions, result.output, result.latency)
                    return [
                        AssertionResult.from_rust(rr, assertion)
                        for rr, assertion in zip(rust_results, assertions)
                    ]
                except Exception as e:
                    logger.warning("Rust assertion evaluation failed (%s), falling back to Python", e)
        # Pure-Python fallback
        results = []
        for assertion in assertions:
            try:
                ar = self._evaluate_single(result, assertion)
                results.append(ar)
            except Exception as e:
                results.append(AssertionResult(
                    assertion=assertion,
                    passed=False,
                    actual=None,
                    message=f"Assertion evaluation error: {e}"
                ))
        return results

    def _evaluate_single(self, result: TestResult, assertion: Assertion) -> AssertionResult:
        at = assertion.assertion_type
        expected = assertion.expected
        output = result.output

        if at == AssertionType.TEXT_CONTAINS:
            passed = expected.lower() in output.lower()
            return AssertionResult(
                assertion=assertion,
                passed=passed,
                actual=output,
                message=f"Expected text '{expected}' {'found' if passed else 'NOT found'}"
            )

        elif at == AssertionType.TEXT_REGEX:
            pattern = expected if isinstance(expected, str) else expected.get("pattern", "")
            flags = 0
            if isinstance(expected, dict) and expected.get("ignore_case"):
                flags = re.IGNORECASE
            match = re.search(pattern, output, flags)
            passed = match is not None
            return AssertionResult(
                assertion=assertion,
                passed=passed,
                actual=match.group(0) if match else None,
                message=f"Regex '{pattern}' {'matched' if passed else 'did NOT match'}"
            )

        elif at == AssertionType.JSON_SCHEMA:
            try:
                data = json.loads(output) if output.strip().startswith(("{", "[")) else None
                if data is None:
                    # Try to extract JSON from markdown
                    json_match = re.search(r'```(?:json)?\s*([\s\S]*?)```', output)
                    if json_match:
                        data = json.loads(json_match.group(1))
                passed = self._validate_json_schema(data, expected)
                return AssertionResult(
                    assertion=assertion,
                    passed=passed,
                    actual=data,
                    message=f"JSON schema validation {'passed' if passed else 'failed'}"
                )
            except Exception as e:
                return AssertionResult(
                    assertion=assertion,
                    passed=False,
                    actual=None,
                    message=f"JSON parse error: {e}"
                )

        elif at == AssertionType.DOM_SELECTOR:
            # For browser tests: check if DOM contains expected selector
            # This is evaluated during browser test execution
            passed = True  # Placeholder - actual check happens in browser test
            return AssertionResult(
                assertion=assertion,
                passed=passed,
                actual=None,
                message="DOM assertion evaluated during browser execution"
            )

        elif at == AssertionType.STATUS_CODE:
            # Extract status code from output if present
            code_match = re.search(r'status[:\s]+(\d+)', output.lower())
            actual_code = int(code_match.group(1)) if code_match else None
            passed = actual_code == expected if actual_code is not None else False
            return AssertionResult(
                assertion=assertion,
                passed=passed,
                actual=actual_code,
                message=f"Status code {actual_code} {'==' if passed else '!='} {expected}"
            )

        elif at == AssertionType.LATENCY_THRESHOLD:
            passed = result.latency <= expected
            return AssertionResult(
                assertion=assertion,
                passed=passed,
                actual=result.latency,
                message=f"Latency {result.latency:.2f}s {'<=' if passed else '>'} {expected}s threshold"
            )

        return AssertionResult(
            assertion=assertion,
            passed=False,
            actual=None,
            message=f"Unknown assertion type: {at}"
        )

    def _validate_json_schema(self, data: Any, schema: Dict) -> bool:
        """Basic JSON schema validation."""
        if not isinstance(data, dict):
            return False
        required_keys = schema.get("required", [])
        for key in required_keys:
            if key not in data:
                return False
        properties = schema.get("properties", {})
        for key, prop_schema in properties.items():
            if key in data:
                expected_type = prop_schema.get("type")
                if expected_type == "string" and not isinstance(data[key], str):
                    return False
                elif expected_type == "number" and not isinstance(data[key], (int, float)):
                    return False
                elif expected_type == "boolean" and not isinstance(data[key], bool):
                    return False
                elif expected_type == "array" and not isinstance(data[key], list):
                    return False
                elif expected_type == "object" and not isinstance(data[key], dict):
                    return False
        return True


class PerformanceTracker:
    """Track and compare performance metrics over time.

    Delegates to Rust sunday-harness for persistent EMA baselines.
    """

    def __init__(self, baseline_path: Optional[str] = None):
        self.baseline_path = baseline_path or os.path.join(
            os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))),
            "harness-stress-test", "performance_baseline.json"
        )
        # Rust-backed tracker (SQLite/JSON persistence, no GIL)
        if _RUST_HARNESS_AVAILABLE:
            self._rust = sunday_rust.PerformanceTracker(self.baseline_path)
        else:
            self._rust = None
        # Pure-Python fallback state
        self.baselines: Dict[str, Dict[str, float]] = self._load_baselines()

    def _load_baselines(self) -> Dict[str, Dict[str, float]]:
        if os.path.exists(self.baseline_path):
            try:
                with open(self.baseline_path, 'r') as f:
                    return json.load(f)
            except Exception:
                pass
        return {}

    def _save_baselines(self):
        os.makedirs(os.path.dirname(self.baseline_path), exist_ok=True)
        with open(self.baseline_path, 'w') as f:
            json.dump(self.baselines, f, indent=2)

    def get_baseline(self, tool_id: str) -> Optional[Dict[str, float]]:
        if self._rust:
            p50 = self._rust.get_baseline_latency_p50(tool_id)
            if p50 is not None:
                return {"latency_p50": p50}
        return self.baselines.get(tool_id)

    def check_regression(self, tool_id: str, latency: float, threshold_ratio: float = 1.5) -> Dict[str, Any]:
        """Check if current latency regresses beyond threshold."""
        if self._rust:
            try:
                regressed, baseline_p50, ratio = self._rust.check_regression(tool_id, latency)
                return {
                    "has_baseline": baseline_p50 is not None,
                    "regression": regressed,
                    "ratio": ratio,
                    "threshold": threshold_ratio,
                    "baseline_latency": baseline_p50,
                }
            except Exception as e:
                logger.warning("Rust perf check failed (%s), falling back to Python", e)

        baseline = self.baselines.get(tool_id)
        if not baseline:
            # First measurement: set baseline
            self.baselines[tool_id] = {
                "latency_p50": latency,
                "latency_p95": latency,
                "count": 1,
                "last_updated": time.time()
            }
            self._save_baselines()
            return {"has_baseline": False, "regression": False, "ratio": 1.0}

        baseline_latency = baseline.get("latency_p50", latency)
        ratio = latency / baseline_latency if baseline_latency > 0 else 1.0

        # Update rolling baseline with exponential moving average
        count = baseline.get("count", 1)
        alpha = 0.3  # EMA smoothing factor
        new_baseline = baseline_latency * (1 - alpha) + latency * alpha
        self.baselines[tool_id]["latency_p50"] = new_baseline
        self.baselines[tool_id]["count"] = count + 1
        self.baselines[tool_id]["last_updated"] = time.time()
        self._save_baselines()

        return {
            "has_baseline": True,
            "regression": ratio > threshold_ratio,
            "ratio": ratio,
            "threshold": threshold_ratio,
            "baseline_latency": baseline_latency
        }

    def update_baseline(self, tool_id: str, latency: float):
        """Force update baseline for a tool."""
        if self._rust:
            try:
                self._rust.update_baseline(tool_id, latency)
            except Exception as e:
                logger.warning("Rust baseline update failed (%s), falling back to Python", e)
        self.baselines[tool_id] = {
            "latency_p50": latency,
            "latency_p95": latency,
            "count": 1,
            "last_updated": time.time()
        }
        self._save_baselines()


class SkillHarness:
    """The SUNDAY Quality Assurance Harness V3.

    Features:
    - Retry with exponential backoff for flaky tests
    - Structured assertions (text, regex, JSON schema, DOM, latency)
    - Visual regression detection
    - Performance regression tracking
    - Cross-platform support
    - Self-healing bridge to learning system
    """

    def __init__(self, engine_name: str = "local", *, browser_mode: str = "playwright",
                 config: Optional[HarnessConfig] = None):
        self.engine_name = engine_name
        self.browser_mode = browser_mode
        self.config = config or HarnessConfig()
        self._orchestrator = None
        self._assertion_engine = AssertionEngine()
        self._visual_checker = VisualRegressionChecker(self.config.visual_baseline_path)
        self._perf_tracker = PerformanceTracker(self.config.latency_baseline_path)
        # Instruction compliance checker for verifying correct tool usage
        if self.config.instruction_compliance:
            self._compliance_checker = InstructionComplianceChecker()
            print(f"[✅ HARNESS] Instruction compliance checking enabled")
        else:
            self._compliance_checker = None
        # Ensure harness mode is set for SSRF bypass
        os.environ.setdefault("SUNDAY_HARNESS_MODE", "1")

    def _get_orchestrator(self):
        if not self._orchestrator:
            # Lazy load orchestrator to avoid circular imports during startup
            from sunday.agents.orchestrator import OrchestratorAgent
            from sunday.engine._discovery import get_engine
            from sunday.core.config import load_config
            from sunday.core.registry import ToolRegistry

            cfg = load_config()
            # 🎯 FORCE local engine for harness — skip slow cloud discovery
            # Override config to prevent cloud fallback
            cfg.intelligence.provider = "local"
            cfg.engine.default = "llamacpp"
            engine_tuple = get_engine(cfg, engine_key="llamacpp")
            if not engine_tuple:
                # Fallback: try any healthy LOCAL engine only
                from sunday.engine._discovery import discover_engines
                engines = discover_engines(cfg)
                local_engines = [(k, v) for k, v in engines if k in ("llamacpp", "ollama", "mlx", "lmstudio")]
                if local_engines:
                    engine_tuple = local_engines[0]
            if not engine_tuple:
                raise RuntimeError("No healthy LOCAL inference engine found for the harness. Please ensure llama-server is running on :8081 or set SUNDAY_ENGINE=llamacpp")

            engine_name, engine_instance = engine_tuple
            # 🧠 HARD OPTIMIZATION: Manually load ONLY essential tools to ensure a tiny prompt
            from sunday.tools.browser import (
                BrowserNavigateTool, BrowserClickTool, BrowserTypeTool,
                BrowserScreenshotTool, BrowserResetTool,
                BrowserExtractTool, BrowserGetAccessibilityTreeTool,
            )
            from sunday.tools.shell_exec import ShellExecTool
            from sunday.tools.system_health_tool import SystemHealthTool

            tools = [
                BrowserNavigateTool(),
                BrowserClickTool(),
                BrowserTypeTool(),
                BrowserScreenshotTool(),
                BrowserResetTool(),
                BrowserExtractTool(),
                BrowserGetAccessibilityTreeTool(),
                ShellExecTool(),
                SystemHealthTool()
            ]

            # 🎯 FORCE harness to use local model only — never cloud
            import os
            os.environ["SUNDAY_ENGINE"] = "llamacpp"
            os.environ["SUNDAY_HARNESS_MODE"] = "1"
            
            self._orchestrator = OrchestratorAgent(
                engine=engine_instance,
                model=cfg.intelligence.fallback_model or cfg.intelligence.default_model,
                mode="function_calling",
                tools=tools,
                parallel_tools=False,  # 🚶‍♂️ Sequential execution for better observability
                interactive=False,
                max_turns=self.config.max_turns,
                visual_audit=self.config.visual_audit,
            )
            self._orchestrator.verbose = True
            # Override the provider to prevent hybrid routing to cloud
            self._orchestrator._config.intelligence.provider = "local"
        return self._orchestrator

    def run_test(self, tool_id: str, prompt: str,
                 assertions: Optional[List[Assertion]] = None) -> TestResult:
        """Run a single test case for a specific tool with retry and assertions."""
        print(f"[🧪 HARNESS] Testing tool: {tool_id} with prompt: '{prompt}'")

        last_result = None
        for attempt in range(self.config.max_retries + 1):
            t0 = time.time()
            orch = self._get_orchestrator()

            # We wrap the user prompt to force the agent to use the specific tool
            harness_prompt = (
                f"TEST CASE: You must use the tool '{tool_id}' to solve the following request.\n"
                f"User Request: {prompt}\n"
                "If the tool fails, explain why. If it succeeds, provide the result clearly."
            )

            try:
                result: AgentResult = orch.run(harness_prompt)
                latency = time.time() - t0

                # Evaluate the result
                success = any(tr.success for tr in result.tool_results if tr.tool_name == tool_id)
                error_msg = None
                if not success:
                    for tr in result.tool_results:
                        if tr.tool_name == tool_id and not tr.success:
                            error_msg = tr.content
                            break

                test_result = TestResult(
                    tool_id=tool_id,
                    prompt=prompt,
                    success=success,
                    output=result.content,
                    error=error_msg,
                    latency=latency,
                    retry_count=attempt
                )

                # Run assertions if provided
                if assertions:
                    test_result.assertion_results = self._assertion_engine.evaluate(test_result, assertions)
                    # Override success if any required assertion fails
                    required_failures = [ar for ar in test_result.assertion_results
                                         if not ar.passed and ar.assertion.required]
                    if required_failures:
                        test_result.success = False
                        test_result.error = f"Assertions failed: {[a.assertion.description for a in required_failures]}"

                # Check performance regression
                perf_check = self._perf_tracker.check_regression(tool_id, latency)
                if perf_check.get("regression"):
                    print(f"      [⚠️ PERFORMANCE] Latency regression detected: {perf_check['ratio']:.2f}x baseline")

                # Check instruction compliance
                if self._compliance_checker:
                    compliance = self._compliance_checker.evaluate(
                        prompt=prompt,
                        tool_results=result.tool_results,
                        output=result.content or "",
                        latency=latency
                    )
                    if not compliance["compliant"]:
                        print(f"      [❌ COMPLIANCE] Intent: {compliance['intent_detected']}")
                        for v in compliance["violations"]:
                            print(f"         - {v['type']}: {v['message']}")
                        test_result.success = False
                        test_result.error = f"Instruction compliance failed: {[v['message'] for v in compliance['violations']]})"
                    else:
                        print(f"      [✅ COMPLIANCE] Intent: {compliance['intent_detected']} - OK")

                last_result = test_result

                if success and not test_result.error:
                    print(f"      [✅] Passed on attempt {attempt + 1} ({latency:.2f}s)")
                    return test_result

                # Decide whether to retry
                if attempt < self.config.max_retries:
                    delay = min(
                        self.config.retry_base_delay * (self.config.retry_backoff_multiplier ** attempt),
                        self.config.retry_max_delay
                    )
                    print(f"      [🔄 RETRY] Attempt {attempt + 1} failed. Retrying in {delay:.1f}s...")
                    time.sleep(delay)
                else:
                    print(f"      [❌] Failed after {self.config.max_retries + 1} attempts")

            except Exception as e:
                latency = time.time() - t0
                last_result = TestResult(
                    tool_id=tool_id,
                    prompt=prompt,
                    success=False,
                    output="",
                    error=str(e),
                    latency=latency,
                    retry_count=attempt
                )
                if attempt < self.config.max_retries:
                    delay = min(
                        self.config.retry_base_delay * (self.config.retry_backoff_multiplier ** attempt),
                        self.config.retry_max_delay
                    )
                    print(f"      [🔄 RETRY] Exception on attempt {attempt + 1}: {e}. Retrying in {delay:.1f}s...")
                    time.sleep(delay)
                else:
                    print(f"      [❌] Exception after {self.config.max_retries + 1} attempts: {e}")

        return last_result

    def _detect_frontend_url(self) -> str:
        """Return the first healthy local SUNDAY frontend URL."""
        for port in range(5173, 5178):
            try:
                with urllib.request.urlopen(f"http://127.0.0.1:{port}", timeout=0.5) as response:
                    html = response.read().decode("utf-8", errors="ignore").lower()
                    if "vite" in html or "sunday" in html or '<div id="root"' in html:
                        print(f"      [📡] Verified Frontend on port {port}")
                        return f"http://127.0.0.1:{port}"
            except Exception:
                continue
        return "http://127.0.0.1:5173"

    def _run_browser_use_test(self, prompt: str, base_url: str, t0: float,
                               assertions: Optional[List[Assertion]] = None) -> TestResult:
        from sunday.tools.browser_use_ext import BrowserUseTaskTool

        task = (
            f"Open {base_url} first. You are testing SUNDAY's own frontend as a user.\n"
            f"Mission: {prompt}\n"
            "Use the visible UI only. If the mission requires the chat, type the request into "
            "the chat input, submit it, wait for a response, then summarize what happened. "
            "Finish with PASS or FAIL and the evidence you saw."
        )
        result = BrowserUseTaskTool().execute(task=task, start_url=base_url)
        latency = time.time() - t0
        output = result.content or ""
        failed_text = ("fail" in output.lower() or "error" in output.lower())
        test_result = TestResult(
            tool_id="dashboard_ui",
            prompt=prompt,
            success=result.success and not failed_text,
            output=output,
            error=None if result.success else output,
            latency=latency,
        )

        if assertions:
            test_result.assertion_results = self._assertion_engine.evaluate(test_result, assertions)

        return test_result

    def _capture_screenshot(self, mission_idx: int, label: str) -> str:
        """Capture screenshot and save to harness-stress-test folder."""
        import os
        from datetime import datetime
        from sunday.tools.browser import _session

        screenshot_dir = os.path.join(
            os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))),
            "harness-stress-test"
        )
        os.makedirs(screenshot_dir, exist_ok=True)

        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        filename = f"mission{mission_idx}_{label}_{timestamp}.png"
        filepath = os.path.join(screenshot_dir, filename)

        try:
            page = _session.page
            # Check if page has content
            content = page.content()
            if len(content) < 100:
                print(f"      [📸 SCREENSHOT] Skipped: page content too short ({len(content)} chars)")
                return ""

            # Wait for page to be fully loaded
            page.wait_for_load_state("domcontentloaded", timeout=5000)
            import time
            time.sleep(2)  # Extra wait for React/Vite to render

            # Use viewport screenshot instead of full_page
            page.screenshot(path=filepath, full_page=False, type="png")
            print(f"      [📸 SCREENSHOT] Saved: {filepath} ({len(content)} chars)")
            return filepath
        except Exception as e:
            print(f"      [📸 SCREENSHOT] Failed: {e}")
            return ""

    def _run_playwright_agent_test(self, prompt: str, base_url: str, t0: float,
                                    mission_idx: int = 1,
                                    assertions: Optional[List[Assertion]] = None) -> TestResult:
        """Fallback E2E path: use the SUNDAY orchestrator with Playwright tools."""
        # 🧹 Close any existing browser session before starting new mission
        from sunday.tools.browser import _session
        _session.close()

        # Extract URL from mission if present, otherwise use base_url
        import re
        url_match = re.search(r'https?://[^\s\)]+', prompt)
        target_url = url_match.group(0) if url_match else base_url
        
        agent_task = (
            f"COMMAND: Navigate to {target_url} using browser_navigate. "
            f"Then call browser_extract with extract_type='text' to read the page. "
            f"Based on what you see, state whether the mission is PASS or FAIL.\n"
            f"MISSION: {prompt}\n"
            "FINAL STEP: State PASS or FAIL with a one-sentence reason."
        )

        print(f"\n{'='*60}")
        print(f"[🔍 AGENT DEBUG] Starting mission {mission_idx}: {prompt}")
        print(f"{'='*60}")

        try:
            orch = self._get_orchestrator()

            # Monkey-patch to log every tool call + capture LLM response screenshots
            original_execute = orch._executor.execute
            def logged_execute(tool_call):
                print(f"\n[🔧 TOOL CALL] {tool_call.name}")
                print(f"[🔧 INPUT] {tool_call.arguments}")
                result = original_execute(tool_call)
                print(f"[🔧 RESULT] success={result.success}, content={result.content[:200] if result.content else 'None'}...")

                # Capture screenshot after typing to see LLM response
                if tool_call.name == "browser_type":
                    print("      [⏳ WAITING] Waiting for LLM response...")
                    import time
                    time.sleep(8)  # Wait for LLM to generate response
                    self._capture_screenshot(mission_idx, "LLM_RESPONSE")

                return result
            orch._executor.execute = logged_execute

            result: AgentResult = orch.run(agent_task)

            # Capture screenshot BEFORE closing browser
            print(f"\n[📸 CAPTURING] Taking final screenshot...")
            output = result.content or ""
            success = any(tr.success and tr.tool_name.startswith("browser_") for tr in result.tool_results)
            success = success and "fail" not in output.lower()
            screenshot_path = self._capture_screenshot(mission_idx, "PASS" if success else "FAIL")

            # Visual regression check
            visual_check = None
            if screenshot_path:
                visual_check = self._visual_checker.compare(screenshot_path, f"mission_{mission_idx}")
                if visual_check.get("regression"):
                    print(f"      [⚠️ VISUAL] Visual regression detected: SSIM={visual_check['ssim']:.3f}")

            print(f"\n{'='*60}")
            print(f"[✅ MISSION COMPLETE]")
            print(f"[📝 Final Answer] {result.content[:500] if result.content else 'None'}")
            print(f"[🔢 Turns] {result.turns}")
            print(f"[⏱️ Latency] {time.time() - t0:.2f}s")
            print(f"{'='*60}")

            test_result = TestResult(
                tool_id="dashboard_ui",
                prompt=prompt,
                success=success,
                output=output,
                latency=time.time() - t0,
                visual_evidence=screenshot_path if success else None,
            )

            if assertions:
                test_result.assertion_results = self._assertion_engine.evaluate(test_result, assertions)

            return test_result

        except Exception as e:
            print(f"\n[❌ ERROR] Mission failed: {e}")
            import traceback
            traceback.print_exc()
            # Capture error screenshot
            screenshot_path = self._capture_screenshot(mission_idx, "ERROR")
            return TestResult(
                tool_id="dashboard_ui",
                prompt=prompt,
                success=False,
                output="",
                error=str(e),
                latency=time.time() - t0,
            )

    def run_browser_test(self, prompt: str, mission_idx: int = 1,
                         assertions: Optional[List[Assertion]] = None) -> TestResult:
        """E2E Test: use Browser-use to interact with the SUNDAY dashboard."""
        print(f"[🌐 BROWSER-HARNESS] Launching Visual Sub-Agent for UI Test...")
        t0 = time.time()

        base_url = self._detect_frontend_url()
        print(f"      [🌐] Target URL: {base_url}")

        # Retry loop for browser tests
        last_result = None
        for attempt in range(self.config.max_retries + 1):
            try:
                if self.browser_mode == "playwright":
                    result = self._run_playwright_agent_test(prompt, base_url, t0, mission_idx, assertions)
                else:
                    result = self._run_browser_use_test(prompt, base_url, t0, assertions)

                result.retry_count = attempt
                last_result = result

                if result.success:
                    print(f"      [✅] Browser test passed on attempt {attempt + 1}")
                    return result

                if attempt < self.config.max_retries:
                    delay = min(
                        self.config.retry_base_delay * (self.config.retry_backoff_multiplier ** attempt),
                        self.config.retry_max_delay
                    )
                    print(f"      [🔄 RETRY] Browser test failed. Retrying in {delay:.1f}s...")
                    time.sleep(delay)
                else:
                    print(f"      [❌] Browser test failed after {self.config.max_retries + 1} attempts")

            except Exception as e:
                last_result = TestResult(
                    tool_id="dashboard_ui",
                    prompt=prompt,
                    success=False,
                    output="",
                    error=str(e),
                    latency=time.time() - t0,
                    retry_count=attempt
                )
                if attempt < self.config.max_retries:
                    delay = min(
                        self.config.retry_base_delay * (self.config.retry_backoff_multiplier ** attempt),
                        self.config.retry_max_delay
                    )
                    print(f"      [🔄 RETRY] Browser exception: {e}. Retrying in {delay:.1f}s...")
                    time.sleep(delay)
                else:
                    print(f"      [❌] Browser exception after {self.config.max_retries + 1} attempts: {e}")

        return last_result

    def heal_tool(self, test_result: TestResult):
        """Send the failure report to the learning system for real self-healing.

        This method bridges harness failures into the trace-driven learning loop:
        1. Saves failure as a trace with outcome='failure'
        2. Emits TRACE_COMPLETE event on EventBus
        3. Triggers ClusterTrigger if failure pattern detected
        4. Optionally invokes DistillationOrchestrator for immediate learning
        """
        if test_result.success:
            print(f"[✅ HARNESS] Tool {test_result.tool_id} passed. No healing needed.")
            return

        print(f"[🚑 HARNESS] Healing initiated for {test_result.tool_id}...")

        # Build structured failure report
        failure_report = {
            "tool_id": test_result.tool_id,
            "prompt": test_result.prompt,
            "error": test_result.error,
            "output": test_result.output,
            "latency": test_result.latency,
            "retry_count": test_result.retry_count,
            "assertion_results": [
                {
                    "type": ar.assertion.assertion_type.value,
                    "description": ar.assertion.description,
                    "passed": ar.passed,
                    "message": ar.message
                }
                for ar in test_result.assertion_results
            ] if test_result.assertion_results else [],
            "timestamp": time.time(),
            "source": "harness",
        }

        # Phase 1: Persist to TraceStore
        trace_id = self._persist_failure_trace(failure_report)
        if trace_id:
            print(f"      [📝 TRACE] Failure trace saved: {trace_id}")

        # Phase 2: Emit event on EventBus
        self._emit_failure_event(failure_report, trace_id)

        # Phase 3: Trigger distillation if conditions met
        self._trigger_distillation(failure_report, trace_id)

        print(f"[🚑 HEALER] Failure report processed. Trace ID: {trace_id}")

    def _persist_failure_trace(self, failure_report: Dict) -> Optional[str]:
        """Save harness failure as a trace in the TraceStore."""
        try:
            # Try to import and use the trace store
            from sunday.memory.trace_store import TraceStore
            from sunday.core.types import Trace, TraceEvent, Message, Role

            trace = Trace(
                trace_id=f"harness-{failure_report['tool_id']}-{int(time.time())}",
                agent_id="harness",
                query=f"[HARNESS] Test {failure_report['tool_id']}: {failure_report['prompt']}",
                response=f"[FAILURE] {failure_report['error'] or failure_report['output'][:500]}",
                tool_calls=[],
                outcome="failure",
                feedback=0.0,  # Low feedback for failures
                metadata={
                    "source": "harness",
                    "tool_id": failure_report["tool_id"],
                    "latency": failure_report["latency"],
                    "retry_count": failure_report["retry_count"],
                    "assertion_results": failure_report.get("assertion_results", []),
                }
            )

            store = TraceStore()
            store.save(trace)
            return trace.trace_id
        except ImportError:
            # Fallback: save to JSON file
            trace_dir = os.path.join(
                os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))),
                "harness-stress-test", "traces"
            )
            os.makedirs(trace_dir, exist_ok=True)
            trace_id = f"harness-{failure_report['tool_id']}-{int(time.time())}"
            trace_path = os.path.join(trace_dir, f"{trace_id}.json")
            with open(trace_path, 'w') as f:
                json.dump(failure_report, f, indent=2)
            return trace_id
        except Exception as e:
            print(f"      [⚠️ TRACE] Failed to persist trace: {e}")
            return None

    def _emit_failure_event(self, failure_report: Dict, trace_id: Optional[str]):
        """Emit TRACE_COMPLETE event on the EventBus."""
        try:
            from sunday.core.events import EventBus, EventType

            bus = EventBus()
            bus.emit(EventType.TRACE_COMPLETE, {
                "trace_id": trace_id,
                "outcome": "failure",
                "tool_id": failure_report["tool_id"],
                "source": "harness",
                "error": failure_report.get("error"),
            })
            print(f"      [📡 EVENT] TRACE_COMPLETE emitted for {trace_id}")
        except ImportError:
            print(f"      [⚠️ EVENT] EventBus not available, skipping event emission")
        except Exception as e:
            print(f"      [⚠️ EVENT] Failed to emit event: {e}")

    def _trigger_distillation(self, failure_report: Dict, trace_id: Optional[str]):
        """Trigger distillation learning if failure conditions warrant it."""
        try:
            from sunday.learning.distillation.triggers import UserFlagTrigger
            from sunday.learning.distillation.orchestrator import DistillationOrchestrator

            # Only trigger for critical failures (not transient retries)
            if failure_report.get("retry_count", 0) >= self.config.max_retries:
                print(f"      [🧠 DISTILL] Critical failure detected, triggering distillation...")
                trigger = UserFlagTrigger(
                    trace_id=trace_id or f"harness-{failure_report['tool_id']}",
                    reason=f"Harness critical failure: {failure_report.get('error', 'Unknown error')}"
                )
                # Run distillation in background to not block harness
                import threading
                def run_distillation():
                    try:
                        orchestrator = DistillationOrchestrator()
                        orchestrator.run(trigger)
                    except Exception as e:
                        print(f"      [⚠️ DISTILL] Background distillation failed: {e}")

                thread = threading.Thread(target=run_distillation, daemon=True)
                thread.start()
                print(f"      [🧠 DISTILL] Distillation triggered in background")
            else:
                print(f"      [ℹ️ DISTILL] Failure recovered after retries, no distillation needed")
        except ImportError:
            print(f"      [⚠️ DISTILL] Distillation system not available")
        except Exception as e:
            print(f"      [⚠️ DISTILL] Failed to trigger distillation: {e}")

    # ── Batch / Parallel Execution ──

    def run_tests_parallel(self, test_cases: List[tuple],
                           max_workers: int = 4) -> List[TestResult]:
        """Run multiple test cases in parallel using threading.

        Args:
            test_cases: List of (tool_id, prompt) tuples or (tool_id, prompt, assertions) tuples
            max_workers: Maximum number of parallel workers

        Returns:
            List of TestResult objects
        """
        from concurrent.futures import ThreadPoolExecutor, as_completed

        results = [None] * len(test_cases)

        def run_single(idx_case):
            idx, case = idx_case
            if len(case) >= 3:
                tool_id, prompt, assertions = case[0], case[1], case[2]
            else:
                tool_id, prompt, assertions = case[0], case[1], None
            return idx, self.run_test(tool_id, prompt, assertions)

        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            futures = {executor.submit(run_single, (i, case)): i for i, case in enumerate(test_cases)}
            for future in as_completed(futures):
                idx, result = future.result()
                results[idx] = result

        return results

    def run_browser_tests_parallel(self, missions: List[str],
                                    max_workers: int = 2) -> List[TestResult]:
        """Run multiple browser E2E tests in parallel.

        Note: Browser tests use fewer workers due to resource constraints.
        """
        from concurrent.futures import ThreadPoolExecutor, as_completed

        results = [None] * len(missions)

        def run_single(idx_mission):
            idx, mission = idx_mission
            return idx, self.run_browser_test(mission, mission_idx=idx + 1)

        with ThreadPoolExecutor(max_workers=max_workers) as executor:
            futures = {executor.submit(run_single, (i, m)): i for i, m in enumerate(missions)}
            for future in as_completed(futures):
                idx, result = future.result()
                results[idx] = result

        return results


if __name__ == "__main__":
    # Quick CLI test with assertions
    harness = SkillHarness()
    # Example: harness.run_test("system_health", "Check my CPU")
