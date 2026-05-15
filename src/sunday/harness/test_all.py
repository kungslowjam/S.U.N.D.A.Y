import sys
import os
from pathlib import Path

# Add src to path
sys.path.append(str(Path(__file__).parent.parent.parent))

from sunday.core.registry import ToolRegistry
from sunday.harness.runner import SkillHarness, HarnessConfig, Assertion, AssertionType


def test_everything(parallel: bool = True, max_workers: int = 4):
    """Run comprehensive batch tests against all registered tools.

    Args:
        parallel: Run tests in parallel for faster execution
        max_workers: Maximum number of parallel workers
    """
    print("="*60)
    print("   SUNDAY AUTOMATED SKILL VERIFICATION (V3)")
    print(f"   Mode: {'Parallel' if parallel else 'Sequential'} ({max_workers} workers)")
    print("="*60)

    config = HarnessConfig(
        max_retries=2,
        retry_base_delay=1.5,
        max_turns=10,
        screenshot_on_pass=True,
        screenshot_on_fail=True,
    )
    harness = SkillHarness(config=config)

    # Exclude meta-tools and core tools from basic tests
    exclude = [
        "think", "list_tools", "reload_tools", "inspect_tool",
        "run_harness_test", "e2e_browser_test", "auto_self_test",
        "harness_batch_test"
    ]

    tools = [name for name, _ in ToolRegistry.items() if name not in exclude]
    print(f"[🧪] Testing {len(tools)} tools...")

    if parallel and len(tools) > 1:
        # Parallel execution
        test_cases = [(tool_id, f"Run a basic check of your {tool_id} capabilities.")
                      for tool_id in tools]
        results = harness.run_tests_parallel(test_cases, max_workers=max_workers)
    else:
        # Sequential execution
        results = []
        for tool_id in tools:
            print(f"\n[🔄] Testing: {tool_id}...")
            test_prompt = f"Run a basic check of your {tool_id} capabilities."
            res = harness.run_test(tool_id, test_prompt)
            results.append(res)

            status = "✅ PASSED" if res.success else "❌ FAILED"
            print(f"    Status: {status} ({res.latency:.2f}s, retries: {res.retry_count})")
            if not res.success:
                print(f"    Error: {res.error}")

    # Generate summary
    print("\n" + "="*60)
    print("   FINAL TEST SUMMARY")
    print("="*60)

    passed = [r for r in results if r.success]
    failed = [r for r in results if not r.success]

    print(f"Total Tested: {len(results)}")
    print(f"PASSED: {len(passed)}")
    print(f"FAILED: {len(failed)}")

    if failed:
        print(f"\nTotal Retries Used: {sum(r.retry_count for r in results)}")
        print(f"\n[🚑] Healing triggered for failed tools:")
        for r in failed:
            print(f"  - {r.tool_id}: {r.error or 'Unknown error'}")
            harness.heal_tool(r)

    # Performance metrics
    if results:
        avg_latency = sum(r.latency for r in results) / len(results)
        max_latency = max(r.latency for r in results)
        print(f"\n📊 Performance:")
        print(f"  Average Latency: {avg_latency:.2f}s")
        print(f"  Max Latency: {max_latency:.2f}s")

    print("="*60)

    return len(failed) == 0


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description="SUNDAY Harness Batch Test Runner")
    parser.add_argument("--sequential", action="store_true",
                        help="Run tests sequentially instead of parallel")
    parser.add_argument("--workers", type=int, default=4,
                        help="Number of parallel workers (default: 4)")
    args = parser.parse_args()

    success = test_everything(
        parallel=not args.sequential,
        max_workers=args.workers
    )
    sys.exit(0 if success else 1)
