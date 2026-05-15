"""Tool builder helper — provides a standardized template for new SUNDAY tools."""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any, Dict, List, Optional

from sunday.core.registry import ToolRegistry
from sunday.core.types import ToolResult
from sunday.tools._stubs import BaseTool, ToolSpec


@ToolRegistry.register("create_tool_scaffold")
class CreateToolScaffoldTool(BaseTool):
    """Generate a standardized Python file for a new tool.
    This ensures proper imports, registration, and structure, reducing AI coding errors.
    """

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="create_tool_scaffold",
            description="Generate a standardized Python file for a new SUNDAY tool.",
            parameters={
                "type": "object",
                "properties": {
                    "tool_id": {"type": "string", "description": "Unique ID for the tool (snake_case)"},
                    "description": {"type": "string", "description": "Brief description of the tool's purpose"}
                },
                "required": ["tool_id", "description"]
            },
            category="coding"
        )

    def execute(self, tool_id: str, description: str, **kwargs: Any) -> ToolResult:
        if not tool_id.isidentifier():
            return ToolResult(tool_name="create_tool_scaffold", content=f"Invalid tool_id: {tool_id}", success=False)

        file_name = f"{tool_id}.py"
        target_path = Path("src/sunday/tools") / file_name
        
        if target_path.exists():
            return ToolResult(tool_name="create_tool_scaffold", content=f"File already exists: {target_path}", success=False)

        class_name = "".join(word.capitalize() for word in tool_id.split("_")) + "Tool"

        template = f'''"""SUNDAY Tool: {tool_id}"""

from __future__ import annotations
from typing import Any
from sunday.core.registry import ToolRegistry
from sunday.core.types import ToolResult
from sunday.tools._stubs import BaseTool, ToolSpec

@ToolRegistry.register("{tool_id}")
class {class_name}(BaseTool):
    """{description}"""

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="{tool_id}",
            description="{description}",
            parameters={{
                "type": "object",
                "properties": {{
                    "query": {{"type": "string", "description": "Input for the tool"}}
                }},
                "required": ["query"]
            }},
            category="custom"
        )

    def execute(self, **params: Any) -> ToolResult:
        query = params.get("query", "")
        # TODO: Implement tool logic here
        return ToolResult(
            tool_name="{tool_id}",
            content=f"Tool executed with query: {{query}}",
            success=True
        )
'''
        try:
            target_path.parent.mkdir(parents=True, exist_ok=True)
            with open(target_path, "w", encoding="utf-8") as f:
                f.write(template)
            
            return ToolResult(
                tool_name="create_tool_scaffold",
                content=f"Successfully created tool scaffold at {target_path}. Now modify the 'execute' method to add logic.",
                success=True,
                metadata={"file_path": str(target_path)}
            )
        except Exception as e:
            return ToolResult(tool_name="create_tool_scaffold", content=f"Failed to create scaffold: {e}", success=False)

__all__ = ["CreateToolScaffoldTool"]
