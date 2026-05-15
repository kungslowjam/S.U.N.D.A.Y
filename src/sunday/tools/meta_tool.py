"""Meta tools for tool discovery and system introspection."""
from __future__ import annotations

import importlib
import sys
from pathlib import Path
from typing import Any

from sunday.core.registry import ToolRegistry
from sunday.core.types import ToolResult
from sunday.tools._stubs import BaseTool, ToolSpec


@ToolRegistry.register("list_tools")
class ListToolsTool(BaseTool):
    """List all available tools in the SUNDAY system.
    Use this to discover new capabilities or check if a self-created tool has been correctly registered.
    """

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="list_tools",
            description="List all currently registered and available tools.",
            parameters={"type": "object", "properties": {}},
            category="meta"
        )

    def execute(self, **kwargs: Any) -> ToolResult:
        tools = ToolRegistry.keys()
        formatted = "\n".join([f"- {t}" for t in sorted(tools)])
        return ToolResult(
            tool_name="list_tools",
            content=f"Available Tools in SUNDAY:\n{formatted}",
            success=True
        )


@ToolRegistry.register("inspect_tool")
class InspectTool(BaseTool):
    """Get detailed information about a specific tool (docstring, parameters)."""

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="inspect_tool",
            description="Get detailed parameter schema and documentation for a specific tool.",
            parameters={
                "type": "object",
                "properties": {
                    "tool_name": {"type": "string", "description": "The name of the tool to inspect"}
                },
                "required": ["tool_name"]
            },
            category="meta"
        )

    def execute(self, tool_name: str, **kwargs: Any) -> ToolResult:
        try:
            tool_cls = ToolRegistry.get(tool_name)
            if not tool_cls:
                raise KeyError(tool_name)
                
            # If it's a class, we instantiate it to get the spec
            if isinstance(tool_cls, type):
                tool_instance = tool_cls()
            else:
                tool_instance = tool_cls
                
            spec = tool_instance.spec
            return ToolResult(
                tool_name="inspect_tool",
                content=(
                    f"Tool: {spec.name}\n"
                    f"Description: {spec.description}\n"
                    f"Parameters: {spec.parameters}"
                ),
                success=True
            )
        except KeyError:
            return ToolResult(tool_name="inspect_tool", content=f"Error: Tool '{tool_name}' not found.", success=False)
        except Exception as e:
            return ToolResult(tool_name="inspect_tool", content=f"Error inspecting tool: {e}", success=False)


@ToolRegistry.register("reload_tools")
class ReloadToolsTool(BaseTool):
    """Reload all tool modules from disk. Use this after modifying or creating tool files to apply changes without restarting the server."""

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="reload_tools",
            description="Reload all Python tool modules from src/sunday/tools. Essential for applying changes made by autonomous agents.",
            parameters={"type": "object", "properties": {}},
            category="meta"
        )

    def execute(self, **kwargs: Any) -> ToolResult:
        import sunday.tools
        try:
            tools_dir = Path(sunday.tools.__file__).parent
            reloaded = []
            new_found = []
            
            # 1. Discover and reload modules
            for f in tools_dir.glob("*.py"):
                if f.name.startswith("_") or f.name == "__init__.py":
                    continue
                
                mod_name = f"sunday.tools.{f.stem}"
                if mod_name in sys.modules:
                    importlib.reload(sys.modules[mod_name])
                    reloaded.append(f.stem)
                else:
                    importlib.import_module(mod_name)
                    new_found.append(f.stem)
            
            # 2. Re-trigger registration logic
            from sunday.tools import discover_tools
            discover_tools()
            
            msg = f"Successfully reloaded {len(reloaded)} modules and imported {len(new_found)} new tools."
            if new_found:
                msg += f"\nNew tools found: {', '.join(new_found)}"
                
            return ToolResult(tool_name="reload_tools", content=msg, success=True)
        except Exception as e:
            return ToolResult(tool_name="reload_tools", content=f"Failed to reload tools: {e}", success=False)


__all__ = ["ListToolsTool", "InspectTool", "ReloadToolsTool"]
