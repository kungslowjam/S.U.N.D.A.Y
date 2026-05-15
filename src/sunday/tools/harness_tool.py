from typing import Any, Dict, List, Optional
from sunday.core.registry import ToolRegistry
from sunday.tools._stubs import BaseTool, ToolSpec
from sunday.core.types import ToolResult
from sunday.harness.runner import SkillHarness, HarnessConfig, Assertion, AssertionType


@ToolRegistry.register("run_harness_test")
class RunHarnessTestTool(BaseTool):
    """Tool to trigger the automated Skill Harness for testing and healing.

    Supports structured assertions, retry logic, and performance regression tracking.
    """

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="run_harness_test",
            description=(
                "MANDATORY for testing. Use this tool when the user asks to test, verify, "
                "or check the system's own skills or UI. Supports 'browser' mode for UI testing, "
                "structured assertions, retry logic, and parallel execution."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "tool_id": {
                        "type": "string",
                        "description": "The ID of the tool to test (e.g., 'system_health')"
                    },
                    "prompt": {
                        "type": "string",
                        "description": "The test prompt to send (e.g., 'Check my disk space')"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["direct", "browser"],
                        "description": "Testing mode: 'direct' for code-level, 'browser' for E2E UI testing via localhost:5173",
                        "default": "direct"
                    },
                    "assertions": {
                        "type": "array",
                        "description": "Optional structured assertions to validate the result",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": {
                                    "type": "string",
                                    "enum": ["text_contains", "text_regex", "json_schema", "status_code", "latency_threshold"],
                                    "description": "Type of assertion"
                                },
                                "expected": {
                                    "description": "Expected value (string for text_contains/text_regex, object for json_schema, number for status_code/latency_threshold)"
                                },
                                "description": {
                                    "type": "string",
                                    "description": "Human-readable description of what this assertion checks"
                                },
                                "required": {
                                    "type": "boolean",
                                    "description": "If false, failure is a warning not an error",
                                    "default": True
                                }
                            },
                            "required": ["type", "expected"]
                        }
                    },
                    "max_retries": {
                        "type": "integer",
                        "description": "Maximum retry attempts for flaky tests",
                        "default": 3
                    },
                    "heal_on_failure": {
                        "type": "boolean",
                        "description": "Trigger self-healing via learning system on failure",
                        "default": True
                    }
                },
                "required": ["tool_id", "prompt"]
            },
            category="quality_assurance"
        )

    def _parse_assertions(self, assertions_raw: List[Dict]) -> List[Assertion]:
        """Parse raw assertion dicts into Assertion objects."""
        assertions = []
        for raw in assertions_raw:
            assertion_type = AssertionType(raw["type"])
            assertions.append(Assertion(
                assertion_type=assertion_type,
                expected=raw["expected"],
                description=raw.get("description", ""),
                required=raw.get("required", True)
            ))
        return assertions

    def execute(self, tool_id: str, prompt: str, mode: str = "direct",
                assertions: List[Dict] = None, max_retries: int = 3,
                heal_on_failure: bool = True, **kwargs: Any) -> ToolResult:
        try:
            config = HarnessConfig(max_retries=max_retries)
            harness = SkillHarness(config=config)

            parsed_assertions = self._parse_assertions(assertions) if assertions else None

            if mode == "browser":
                result = harness.run_browser_test(prompt, assertions=parsed_assertions)
            else:
                result = harness.run_test(tool_id, prompt, assertions=parsed_assertions)

            # Build detailed report
            status = "✅ PASSED" if result.success else "❌ FAILED"
            report = (
                f"### Skill Test Report: {tool_id}\n"
                f"- **Status**: {status}\n"
                f"- **Prompt**: {result.prompt}\n"
                f"- **Latency**: {result.latency:.2f}s\n"
                f"- **Retries**: {result.retry_count}\n"
            )

            if result.assertion_results:
                report += "\n#### Assertions:\n"
                for ar in result.assertion_results:
                    a_status = "✅" if ar.passed else "⚠️" if not ar.assertion.required else "❌"
                    report += f"- {a_status} {ar.assertion.description or ar.assertion.assertion_type.value}: {ar.message}\n"

            if not result.success:
                report += f"\n- **Error**: {result.error}\n"
                if heal_on_failure:
                    report += "\n**🚑 Healing**: Triggering self-healing via learning system..."
                    harness.heal_tool(result)
            else:
                report += f"\n- **Output**: {result.output[:300]}..."

            return ToolResult(
                tool_name="run_harness_test",
                content=report,
                success=result.success
            )

        except Exception as e:
            return ToolResult(
                tool_name="run_harness_test",
                content=f"Harness Execution Error: {str(e)}",
                success=False
            )


@ToolRegistry.register("e2e_browser_test")
class E2EBrowserTestTool(BaseTool):
    """Run a focused Browser-use self-test against the SUNDAY frontend."""

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="e2e_browser_test",
            description="Run one Browser-use E2E mission against SUNDAY's own frontend.",
            parameters={
                "type": "object",
                "properties": {
                    "prompt": {
                        "type": "string",
                        "description": "The UI mission to test, e.g. 'open chat and ask for system health'.",
                    },
                    "assertions": {
                        "type": "array",
                        "description": "Optional structured assertions",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": {
                                    "type": "string",
                                    "enum": ["text_contains", "text_regex", "latency_threshold"],
                                },
                                "expected": {},
                                "description": {"type": "string"},
                                "required": {"type": "boolean", "default": True}
                            },
                            "required": ["type", "expected"]
                        }
                    }
                },
                "required": ["prompt"],
            },
            category="quality_assurance",
            timeout_seconds=360.0,
        )

    def execute(self, prompt: str, assertions: List[Dict] = None, **kwargs: Any) -> ToolResult:
        try:
            harness = SkillHarness()
            parsed_assertions = None
            if assertions:
                parsed_assertions = [
                    Assertion(
                        assertion_type=AssertionType(a["type"]),
                        expected=a["expected"],
                        description=a.get("description", ""),
                        required=a.get("required", True)
                    )
                    for a in assertions
                ]

            result = harness.run_browser_test(prompt, assertions=parsed_assertions)
            status = "PASSED" if result.success else "FAILED"
            content = (
                f"### E2E Browser Test: {status}\n"
                f"- Prompt: {result.prompt}\n"
                f"- Latency: {result.latency:.2f}s\n"
                f"- Retries: {result.retry_count}\n"
                f"- Output: {result.output[:1000]}"
            )
            if result.error:
                content += f"\n- Error: {result.error[:1000]}"
            if result.assertion_results:
                content += "\n\n#### Assertions:\n"
                for ar in result.assertion_results:
                    a_status = "✅" if ar.passed else "⚠️" if not ar.assertion.required else "❌"
                    content += f"- {a_status} {ar.assertion.description or ar.assertion.assertion_type.value}: {ar.message}\n"

            return ToolResult(
                tool_name="e2e_browser_test",
                content=content,
                success=result.success,
            )
        except Exception as e:
            return ToolResult(
                tool_name="e2e_browser_test",
                content=f"E2E Browser Test Error: {e}",
                success=False,
            )


@ToolRegistry.register("auto_self_test")
class AutoSelfTestTool(BaseTool):
    """Ultimate Zero-Config E2E Test: AI tests itself autonomously."""

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="auto_self_test",
            description="Run autonomous E2E tests for one or more use cases sequentially or in parallel.",
            parameters={
                "type": "object",
                "properties": {
                    "use_cases": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "A list of scenarios to test.",
                        "default": ["Check system health", "Search for current news"]
                    },
                    "parallel": {
                        "type": "boolean",
                        "description": "Run tests in parallel (faster but uses more resources)",
                        "default": False
                    },
                    "max_workers": {
                        "type": "integer",
                        "description": "Maximum parallel workers when parallel=true",
                        "default": 2
                    },
                    "heal_failures": {
                        "type": "boolean",
                        "description": "Trigger healing for failed tests",
                        "default": True
                    }
                }
            },
            category="quality_assurance"
        )

    def execute(self, use_cases: List[str] = None, parallel: bool = False,
                max_workers: int = 2, heal_failures: bool = True, **kwargs: Any) -> ToolResult:
        if not use_cases:
            use_cases = ["Choose a realistic task to test your own capabilities."]

        harness = SkillHarness()
        overall_reports = []
        all_success = True

        if parallel:
            print(f"[🔄 QA-PIPE] Running {len(use_cases)} missions in parallel (max {max_workers} workers)...")
            results = harness.run_browser_tests_parallel(use_cases, max_workers=max_workers)
            for i, result in enumerate(results):
                status = "✅ PASSED" if result.success else "❌ FAILED"
                overall_reports.append(
                    f"**Mission {i+1}**: {use_cases[i]}\n"
                    f"- Status: {status}\n"
                    f"- Latency: {result.latency:.2f}s\n"
                    f"- Retries: {result.retry_count}\n"
                    f"- Report: {result.output[:150]}..."
                )
                if not result.success:
                    all_success = False
                    if heal_failures:
                        harness.heal_tool(result)
        else:
            for i, mission in enumerate(use_cases):
                print(f"[🔄 QA-PIPE] Running Mission {i+1}/{len(use_cases)}: {mission}")
                result = harness.run_browser_test(
                    f"MISSION {i+1}: {mission}\nVerify the outcome carefully.",
                    mission_idx=i+1
                )

                status = "✅ PASSED" if result.success else "❌ FAILED"
                overall_reports.append(
                    f"**Mission {i+1}**: {mission}\n"
                    f"- Status: {status}\n"
                    f"- Latency: {result.latency:.2f}s\n"
                    f"- Retries: {result.retry_count}\n"
                    f"- Report: {result.output[:150]}..."
                )

                if not result.success:
                    all_success = False
                    if heal_failures:
                        harness.heal_tool(result)

        summary = "\n\n".join(overall_reports)
        final_status = "🏆 ALL TESTS PASSED" if all_success else "⚠️ SOME TESTS FAILED"

        return ToolResult(
            tool_name="auto_self_test",
            content=f"## Continuous Test Results\nOverall Status: {final_status}\n\n{summary}",
            success=all_success
        )


@ToolRegistry.register("harness_batch_test")
class HarnessBatchTestTool(BaseTool):
    """Run batch tests against multiple tools with parallel execution and comprehensive reporting."""

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="harness_batch_test",
            description=(
                "Run comprehensive batch tests against multiple tools in parallel. "
                "Generates a full test report with performance metrics, assertion results, "
                "and healing recommendations."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "tools": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of tool IDs to test. Use ['all'] to test all registered tools.",
                        "default": ["all"]
                    },
                    "max_workers": {
                        "type": "integer",
                        "description": "Maximum parallel workers",
                        "default": 4
                    },
                    "exclude": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Tool IDs to exclude from testing",
                        "default": ["think", "list_tools", "reload_tools", "inspect_tool",
                                    "run_harness_test", "e2e_browser_test", "auto_self_test",
                                    "harness_batch_test"]
                    }
                }
            },
            category="quality_assurance"
        )

    def execute(self, tools: List[str] = None, max_workers: int = 4,
                exclude: List[str] = None, **kwargs: Any) -> ToolResult:
        try:
            from sunday.core.registry import ToolRegistry

            if tools is None:
                tools = ["all"]
            if exclude is None:
                exclude = ["think", "list_tools", "reload_tools", "inspect_tool",
                           "run_harness_test", "e2e_browser_test", "auto_self_test",
                           "harness_batch_test"]

            harness = SkillHarness()

            # Determine which tools to test
            if "all" in tools:
                all_tools = [name for name, _ in ToolRegistry.items() if name not in exclude]
            else:
                all_tools = [t for t in tools if t not in exclude]

            print(f"[🧪 BATCH] Testing {len(all_tools)} tools with {max_workers} workers...")

            # Build test cases
            test_cases = [(tool_id, f"Run a basic check of your {tool_id} capabilities.")
                          for tool_id in all_tools]

            results = harness.run_tests_parallel(test_cases, max_workers=max_workers)

            # Generate report
            passed = [r for r in results if r.success]
            failed = [r for r in results if not r.success]

            report = (
                f"## Batch Test Report\n\n"
                f"**Total**: {len(results)} | **Passed**: {len(passed)} | **Failed**: {len(failed)}\n\n"
            )

            if passed:
                report += "### ✅ Passed\n"
                for r in passed:
                    report += f"- `{r.tool_id}`: {r.latency:.2f}s (retries: {r.retry_count})\n"

            if failed:
                report += "\n### ❌ Failed\n"
                for r in failed:
                    report += f"- `{r.tool_id}`: {r.error or 'Unknown error'}\n"
                    if r.assertion_results:
                        for ar in r.assertion_results:
                            if not ar.passed:
                                report += f"  - Assertion failed: {ar.message}\n"
                    # Trigger healing
                    harness.heal_tool(r)

            # Performance summary
            if results:
                avg_latency = sum(r.latency for r in results) / len(results)
                max_latency = max(r.latency for r in results)
                report += f"\n### 📊 Performance\n"
                report += f"- Average latency: {avg_latency:.2f}s\n"
                report += f"- Max latency: {max_latency:.2f}s\n"
                report += f"- Total retries: {sum(r.retry_count for r in results)}\n"

            return ToolResult(
                tool_name="harness_batch_test",
                content=report,
                success=len(failed) == 0
            )

        except Exception as e:
            return ToolResult(
                tool_name="harness_batch_test",
                content=f"Batch Test Error: {str(e)}",
                success=False
            )


# No auto-register here to avoid import loops; handled by the registry loader.
