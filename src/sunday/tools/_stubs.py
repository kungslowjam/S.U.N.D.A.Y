"""ABC for tool implementations and the ToolExecutor dispatch engine.

Follows the same registry pattern as ``engine/_stubs.py`` and ``memory/_stubs.py``.
Each tool is registered via ``@ToolRegistry.register("name")`` and implements
``BaseTool`` with a ``spec`` property and ``execute()`` method.
"""

from __future__ import annotations

import concurrent.futures
import json
import time
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, Callable, Dict, List, Optional

from sunday.core.events import EventBus, EventType
from sunday.core.types import ToolCall, ToolResult

# ---------------------------------------------------------------------------
# ToolSpec — metadata describing a tool's interface
# ---------------------------------------------------------------------------


@dataclass(slots=True)
class ToolSpec:
    """Declarative description of a tool's interface and characteristics."""

    name: str
    description: str
    parameters: Dict[str, Any] = field(default_factory=dict)
    category: str = ""
    cost_estimate: float = 0.0
    latency_estimate: float = 0.0
    requires_confirmation: bool = False
    timeout_seconds: float = 30.0
    required_capabilities: List[str] = field(default_factory=list)
    metadata: Dict[str, Any] = field(default_factory=dict)


# ---------------------------------------------------------------------------
# BaseTool ABC
# ---------------------------------------------------------------------------


class BaseTool(ABC):
    """Base class for all tool implementations.

    Subclasses must be registered via
    ``@ToolRegistry.register("name")`` to become discoverable.
    """

    tool_id: str
    is_local: bool = True

    @property
    @abstractmethod
    def spec(self) -> ToolSpec:
        """Return the tool specification."""

    @abstractmethod
    def execute(self, **params: Any) -> ToolResult:
        """Execute the tool with the given parameters."""

    def to_openai_function(self) -> Dict[str, Any]:
        """Convert to OpenAI function-calling format."""
        from sunday.tools.description_loader import (
            get_tool_description_override,
        )

        s = self.spec
        desc = get_tool_description_override(s.name) or s.description
        return {
            "type": "function",
            "function": {
                "name": s.name,
                "description": desc,
                "parameters": s.parameters,
            },
        }


# ---------------------------------------------------------------------------
# ToolExecutor — dispatch engine for tool calls
# ---------------------------------------------------------------------------


class ToolExecutor:
    """Dispatch tool calls to registered tools with event bus integration.

    Parameters
    ----------
    tools:
        List of tool instances to make available.
    bus:
        Optional event bus for publishing ``TOOL_CALL_START``/``TOOL_CALL_END``.
    """

    def __init__(
        self,
        tools: List[BaseTool],
        bus: Optional[EventBus] = None,
        *,
        interactive: bool = False,
        confirm_callback: Optional[Callable[[str], bool]] = None,
        default_timeout: float = 30.0,
        capability_policy: Optional[Any] = None,
        agent_id: str = "",
        boundary_guard: Optional[Any] = None,
    ) -> None:
        self._tools: Dict[str, BaseTool] = {t.spec.name: t for t in tools}
        self._bus = bus
        self._interactive = interactive
        self._confirm_callback = confirm_callback
        self._default_timeout = default_timeout
        self._capability_policy = capability_policy
        self._agent_id = agent_id
        self._boundary_guard = boundary_guard
        # Persistent thread pool (max_workers=1) ensures Playwright state 
        # is preserved on the same thread across calls.
        self._pool = concurrent.futures.ThreadPoolExecutor(max_workers=1)

    def __del__(self) -> None:
        """Shut down the persistent thread pool on destruction."""
        try:
            self._pool.shutdown(wait=False)
        except Exception:
            pass

    def execute(self, tool_call: ToolCall) -> ToolResult:
        """Parse arguments, dispatch to tool, measure latency, emit events."""
        tool = self._tools.get(tool_call.name)
        if tool is None:
            return ToolResult(
                tool_name=tool_call.name,
                content=f"Unknown tool: {tool_call.name}",
                success=False,
            )

        # Sanitize arguments string
        args_str = tool_call.arguments.strip() if tool_call.arguments else ""
        if args_str.startswith("```json"):
            args_str = args_str[7:]
        elif args_str.startswith("```"):
            args_str = args_str[3:]
        if args_str.endswith("```"):
            args_str = args_str[:-3]
        args_str = args_str.strip()

        # Parse arguments
        try:
            params = json.loads(args_str) if args_str else {}
            # Robustness: if params is a string, wrap it in a dict using the first required param
            if isinstance(params, str):
                required = tool.spec.parameters.get("required", [])
                key = required[0] if required else "query"
                params = {key: params}
        except json.JSONDecodeError:
            import re
            params = {}
            # Try to salvage malformed JSON by extracting the first string value if it looks like JSON
            if args_str.startswith("{"):
                # Try to find a URL or string value
                match = re.search(r'"(?:url|query|code|selector)"\s*:\s*"([^"]+)"', args_str)
                if match:
                    val = match.group(1)
                    # We need to unescape newlines if it's code
                    val = val.replace('\\n', '\n').replace('\\"', '"')
                    required = tool.spec.parameters.get("required", [])
                    key = required[0] if required else "query"
                    params = {key: val}
            
            # If still empty, use the raw string as fallback
            if not params:
                required = tool.spec.parameters.get("required", [])
                key = required[0] if required else "query"
                params = {key: args_str}

        # Specific sanitization for code_interpreter
        if tool_call.name == "code_interpreter" and "code" in params:
            code_str = params["code"]
            if isinstance(code_str, str):
                code_str = code_str.strip()
                if code_str.startswith("```python"):
                    code_str = code_str[9:]
                elif code_str.startswith("```"):
                    code_str = code_str[3:]
                if code_str.endswith("```"):
                    code_str = code_str[:-3]
                params["code"] = code_str.strip()

        # Boundary guard: scan external tool arguments
        if self._boundary_guard is not None and not getattr(tool, "is_local", True):
            try:
                # Sync arguments back to ToolCall for boundary check
                tool_call.arguments = json.dumps(params)
                tool_call = self._boundary_guard.check_outbound(tool_call)
                # Re-parse arguments after potential redaction
                params = json.loads(tool_call.arguments)
            except Exception as exc:
                return ToolResult(
                    tool_name=tool_call.name,
                    content=f"Security block: {exc}",
                    success=False,
                )

        # RBAC capability check
        if self._capability_policy and tool.spec.required_capabilities:
            for cap in tool.spec.required_capabilities:
                if not self._capability_policy.check(
                    self._agent_id,
                    cap,
                    tool_call.name,
                ):
                    if self._bus:
                        self._bus.publish(
                            EventType.CAPABILITY_DENIED,
                            {
                                "agent_id": self._agent_id,
                                "capability": cap,
                                "tool": tool_call.name,
                            },
                        )
                    return ToolResult(
                        tool_name=tool_call.name,
                        content=(
                            f"Capability '{cap}' denied for"
                            f" agent '{self._agent_id}'"
                            f" on tool '{tool_call.name}'."
                        ),
                        success=False,
                    )

        # Taint checking (sink policy)
        taint_set = params.get("_taint") if isinstance(params, dict) else None
        if taint_set is not None:
            try:
                from sunday.security.taint import TaintSet, check_taint

                if isinstance(taint_set, TaintSet):
                    violation = check_taint(tool_call.name, taint_set)
                    if violation:
                        if self._bus:
                            self._bus.publish(
                                EventType.TAINT_VIOLATION,
                                {
                                    "tool": tool_call.name,
                                    "violation": violation,
                                },
                            )
                        return ToolResult(
                            tool_name=tool_call.name,
                            content=f"Taint violation: {violation}",
                            success=False,
                        )
            except ImportError:
                pass
            # Remove internal taint key before passing to tool
            if isinstance(params, dict):
                params.pop("_taint", None)

        # Confirmation check for sensitive tools
        if tool.spec.requires_confirmation:
            if not self._interactive or self._confirm_callback is None:
                return ToolResult(
                    tool_name=tool_call.name,
                    content=(
                        f"Tool '{tool_call.name}' requires"
                        " confirmation but no confirmation"
                        " callback is available."
                    ),
                    success=False,
                )
            prompt = f"Allow execution of tool '{tool_call.name}' with args {params}?"
            if not self._confirm_callback(prompt):
                return ToolResult(
                    tool_name=tool_call.name,
                    content=f"Tool '{tool_call.name}' execution denied by user.",
                    success=False,
                )

        # Emit start event
        if self._bus:
            self._bus.publish(
                EventType.TOOL_CALL_START,
                {"tool": tool_call.name, "arguments": params},
            )

        # Execute with timeout on the persistent pool
        timeout = tool.spec.timeout_seconds or self._default_timeout
        t0 = time.time()
        try:
            future = self._pool.submit(tool.execute, **params)
            result = future.result(timeout=timeout)
        except concurrent.futures.TimeoutError:
            if self._bus:
                self._bus.publish(
                    EventType.TOOL_TIMEOUT,
                    {"tool": tool_call.name, "timeout": timeout},
                )
            result = ToolResult(
                tool_name=tool_call.name,
                content=(f"Tool '{tool_call.name}' timed out after {timeout:.0f}s."),
                success=False,
            )
        except Exception as exc:
            result = ToolResult(
                tool_name=tool_call.name,
                content=f"Tool execution error: {exc}",
                success=False,
            )
        latency = time.time() - t0
        result.latency_seconds = latency
        result.metadata["arguments"] = params

        # Auto-detect taints in results
        if result.success:
            try:
                from sunday.security.taint import auto_detect_taint

                detected = auto_detect_taint(result.content)
                if detected and detected.labels:
                    result.metadata["_taint"] = detected
            except ImportError:
                pass

        # Emit end event
        if self._bus:
            if (
                result.metadata.get("skill_kind") == "instructional"
                or result.tool_name.startswith("skill_")
                and result.metadata.get("steps", 0) == 0
            ):
                result_text = (
                    f"Loaded instructional skill "
                    f"`{result.metadata.get('skill', result.tool_name)}`."
                )
            else:
                result_text = str(result.content)[:10240] if result.content else ""
            
            event_metadata = self._json_safe_metadata(result.metadata)
            self._bus.publish(
                EventType.TOOL_CALL_END,
                {
                    "tool": tool_call.name,
                    "success": result.success,
                    "latency": latency,
                    "result": result_text,
                    "metadata": event_metadata,
                },
            )

        return result

    @staticmethod
    def _json_safe_metadata(metadata: Optional[Dict[str, Any]]) -> Dict[str, Any]:
        """Return a copy of *metadata* containing only JSON-serializable values."""
        if not metadata:
            return {}

        import json

        safe: Dict[str, Any] = {}
        for key, value in metadata.items():
            if not isinstance(key, str):
                continue
            try:
                json.dumps(value)
            except (TypeError, ValueError):
                continue
            safe[key] = value
        return safe

    def available_tools(self) -> List[ToolSpec]:
        """Return specs for all available tools."""
        return [t.spec for t in self._tools.values()]

    def get_openai_tools(self, filter_names: Optional[List[str]] = None) -> List[Dict[str, Any]]:
        """Return tools in OpenAI function-calling format."""
        if filter_names is None:
            return [t.to_openai_function() for t in self._tools.values()]
        return [
            t.to_openai_function() 
            for name, t in self._tools.items() 
            if name in filter_names
        ]


def build_tool_descriptions(
    tools: List[BaseTool],
    *,
    include_category: bool = False,
    include_cost: bool = False,
) -> str:
    """Build ultra-compact text descriptions to save tokens."""
    if not tools:
        return "No tools available."

    from sunday.tools.description_loader import (
        get_tool_description_override,
    )

    sections: list[str] = []
    for t in tools:
        s = t.spec
        desc = get_tool_description_override(s.name) or s.description
        lines = [f"- **{s.name}**: {desc}"]
        
        props = s.parameters.get("properties", {})
        if props:
            params = []
            for name, p in props.items():
                p_desc = p.get("description", "")
                params.append(f"{name}: {p_desc}")
            lines.append(f"  Args: {', '.join(params)}")
            
        sections.append("\n".join(lines))

    return "\n".join(sections)


__all__ = ["BaseTool", "ToolExecutor", "ToolSpec", "build_tool_descriptions"]
