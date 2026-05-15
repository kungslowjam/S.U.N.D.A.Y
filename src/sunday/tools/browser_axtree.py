"""Browser accessibility tree extraction tool.

Extracts the accessibility tree (AX tree) from the current browser page,
providing a structured text representation of the DOM with element IDs,
roles, names, and states. Used by top-performing agents on WebArena-family
benchmarks.
"""

from __future__ import annotations

from typing import Any

from sunday.core.registry import ToolRegistry
from sunday.core.types import ToolResult
from sunday.tools._stubs import BaseTool, ToolSpec
from sunday._rust_bridge import AXTreeProcessor
import json

# Re-use the shared browser session from the browser module.
# This is imported at module level so tests can patch
# ``sunday.tools.browser_axtree._session``.
try:
    from sunday.tools.browser import _session
except Exception:  # pragma: no cover — optional dependency
    _session = None  # type: ignore[assignment]


@ToolRegistry.register("browser_axtree")
class BrowserAXTreeTool(BaseTool):
    """Extract the accessibility tree from the current browser page."""

    tool_id = "browser_axtree"
    is_local = False

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="browser_axtree",
            description=(
                "Extract the accessibility tree from the current browser page. "
                "Returns a structured text representation with element roles, "
                "names, values, and states. More structured than raw HTML."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "max_depth": {
                        "type": "integer",
                        "description": "Maximum tree depth to traverse. Default: 10.",
                    },
                },
            },
            category="browser",
            required_capabilities=["network:fetch"],
        )

    def execute(self, **params: Any) -> ToolResult:
        max_depth = params.get("max_depth", 10)

        try:
            page = _session.page  # type: ignore[union-attr]
        except ImportError as exc:
            return ToolResult(
                tool_name="browser_axtree",
                content=f"Playwright not installed: {exc}",
                success=False,
            )
        except AttributeError:
            return ToolResult(
                tool_name="browser_axtree",
                content="Browser session not available.",
                success=False,
            )

        try:
            # 🔍 ROBUST EXTRACTION: Try multiple ways to get the tree
            snapshot = None
            if hasattr(page, "accessibility"):
                snapshot = page.accessibility.snapshot()
            
            if not snapshot:
                # Fallback: Check if it's a Chrome DevTools Protocol session
                try:
                    snapshot = page.evaluate("window.__get_ax_tree()")
                except:
                    pass

            if not snapshot:
                return ToolResult(
                    tool_name="browser_axtree",
                    content="No accessibility tree available (API not supported or page empty).",
                    success=False,
                )

            # --- RUST OPTIMIZATION: AXTree Semantic Filtering ---
            if AXTreeProcessor:
                # Use Rust for fast formatting and semantic noise reduction
                processor = AXTreeProcessor(max_depth=max_depth, filter_unimportant=True)
                # Playwright snapshot is a dict, Rust expects JSON string
                text = processor.process_json(json.dumps(snapshot))
            else:
                # Fallback to slow Python formatting
                text = _format_axtree(snapshot, max_depth=max_depth)

            return ToolResult(
                tool_name="browser_axtree",
                content=text,
                success=True,
                metadata={"node_count": _count_nodes(snapshot)},
            )
        except Exception as exc:
            return ToolResult(
                tool_name="browser_axtree",
                content=f"AX tree extraction error: {exc}",
                success=False,
            )


def _format_axtree(
    node: dict,
    depth: int = 0,
    max_depth: int = 10,
) -> str:
    """Format an accessibility tree node as indented text."""
    if depth >= max_depth:
        return ""

    indent = "  " * depth
    role = node.get("role", "unknown")
    name = node.get("name", "")
    value = node.get("value", "")

    parts = [f"{indent}[{role}]"]
    if name:
        parts.append(f' "{name}"')
    if value:
        parts.append(f" value={value}")

    line = "".join(parts)
    lines = [line]

    for child in node.get("children", []):
        child_text = _format_axtree(child, depth + 1, max_depth)
        if child_text:
            lines.append(child_text)

    return "\n".join(lines)


def _count_nodes(node: dict) -> int:
    """Count total nodes in the accessibility tree."""
    count = 1
    for child in node.get("children", []):
        count += _count_nodes(child)
    return count


__all__ = ["BrowserAXTreeTool"]
