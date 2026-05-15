"""Sub-agent delegation tools for autonomous task partitioning."""

from __future__ import annotations

import json
import logging
from typing import Any, List, Optional

from sunday.core.registry import ToolRegistry
from sunday.core.types import ToolResult
from sunday.tools._stubs import BaseTool, ToolSpec

logger = logging.getLogger(__name__)

@ToolRegistry.register("delegate_browser")
class DelegateBrowserTool(BaseTool):
    """Delegate a web-based research or automation task to a specialized Browser Agent."""

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="delegate_browser",
            description="Delegate web browsing, data extraction, or research tasks to a sub-agent with a real browser.",
            parameters={
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "A highly detailed description of the task for the Browser Sub-Agent.",
                    }
                },
                "required": ["task"],
            },
            category="agents",
        )

    def execute(self, **params: Any) -> ToolResult:
        task = params.get("task", "")
        if not task:
            return ToolResult(tool_name="delegate_browser", content="Task cannot be empty.", success=False)
            
        try:
            from sunday.core.config import load_config
            from sunday.engine import get_engine
            from sunday.agents.orchestrator import OrchestratorAgent
            
            config = load_config()
            # Correct get_engine signature: (config, engine_key)
            # Returns: (key, engine_instance) or None
            engine_tuple = get_engine(config, "multi")
            if not engine_tuple:
                return ToolResult(tool_name="delegate_browser", content="No inference engine available for sub-agent.", success=False)
            
            _, engine_instance = engine_tuple
            
            # Browser specific tools
            browser_tool_names = [
                "browser_navigate", "browser_click", "browser_type", 
                "browser_get_elements", "browser_extract", "web_search", "browser_get_accessibility_tree"
            ]
            
            tools = []
            for t_name in browser_tool_names:
                cls = ToolRegistry.get(t_name)
                if cls:
                    tools.append(cls() if isinstance(cls, type) else cls)
            
            agent = OrchestratorAgent(
                engine=engine_instance,
                model=config.intelligence.default_model,
                tools=tools,
                system_prompt=(
                    "You are a SUNDAY Browser Sub-Agent. Your ONLY goal is to fulfill the user's research or web automation task. "
                    "Use your browser tools effectively. Be concise and return only the final answer or data requested."
                ),
                max_turns=10,
                interactive=False
            )
            
            result = agent.run(task)
            return ToolResult(
                tool_name="delegate_browser",
                content=result.content,
                success=True,
                metadata={"turns": result.turns}
            )
        except Exception as e:
            logger.exception("Delegate browser failed")
            return ToolResult(tool_name="delegate_browser", content=f"Failed to run sub-agent: {e}", success=False)


@ToolRegistry.register("delegate_research")
class DelegateResearchTool(BaseTool):
    """Delegate complex deep research tasks to a specialized Research Agent."""

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="delegate_research",
            description="Delegate complex research, data analysis, or deep information retrieval tasks.",
            parameters={
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "The research goal or question to answer.",
                    }
                },
                "required": ["task"],
            },
            category="agents",
        )

    def execute(self, **params: Any) -> ToolResult:
        task = params.get("task", "")
        try:
            from sunday.core.config import load_config
            from sunday.engine import get_engine
            from sunday.agents.orchestrator import OrchestratorAgent
            
            config = load_config()
            engine_tuple = get_engine(config, "multi")
            if not engine_tuple:
                return ToolResult(tool_name="delegate_research", content="No engine available.", success=False)
            
            _, engine_instance = engine_tuple
            
            # Research tools: web search + knowledge tools
            research_tool_names = ["web_search", "knowledge_search", "think", "calculator"]
            tools = []
            for t_name in research_tool_names:
                cls = ToolRegistry.get(t_name)
                if cls:
                    tools.append(cls() if isinstance(cls, type) else cls)
            
            agent = OrchestratorAgent(
                engine=engine_instance,
                model=config.intelligence.default_model,
                tools=tools,
                system_prompt="You are a SUNDAY Research Sub-Agent. Provide deep, cited, and accurate information.",
                max_turns=10,
                interactive=False
            )
            
            result = agent.run(task)
            return ToolResult(tool_name="delegate_research", content=result.content, success=True)
        except Exception as e:
            return ToolResult(tool_name="delegate_research", content=f"Failed: {e}", success=False)


@ToolRegistry.register("delegate_coding")
class DelegateCodingTool(BaseTool):
    """Delegate software engineering or codebase modification tasks to the Coding Architect."""

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="delegate_coding",
            description="Delegate coding, tool creation, or codebase analysis to the Coding Architect.",
            parameters={
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "Description of the coding task, file to create, or bug to fix.",
                    }
                },
                "required": ["task"],
            },
            category="agents",
        )

    def execute(self, **params: Any) -> ToolResult:
        task = params.get("task", "")
        try:
            from sunday.agents.manager import AgentManager
            from sunday.core.config import load_config
            from sunday.engine import get_engine
            
            config = load_config()
            manager = AgentManager()
            
            # Try to load the dedicated coding architect recipe
            try:
                recipe = manager.get_recipe("coding_assistant")
            except:
                recipe = None
                
            engine_tuple = get_engine(config, "multi")
            if not engine_tuple:
                return ToolResult(tool_name="delegate_coding", content="No engine.", success=False)
            
            _, engine_instance = engine_tuple
            
            from sunday.agents.orchestrator import OrchestratorAgent
            
            # Coding tools
            coding_tool_names = ["file_read", "file_write", "shell_exec", "graphify", "list_tools", "inspect_tool", "reload_tools", "think"]
            tools = []
            for t_name in coding_tool_names:
                cls = ToolRegistry.get(t_name)
                if cls:
                    tools.append(cls() if isinstance(cls, type) else cls)
            
            agent = OrchestratorAgent(
                engine=engine_instance,
                model=config.intelligence.default_model,
                tools=tools,
                system_prompt=(
                    "You are the SUNDAY Coding Architect (Architect Prime). "
                    "Your goal is to fulfill software engineering tasks. "
                    "ALWAYS use 'graphify' to see the project first. "
                    "On Windows, use powershell commands via shell_exec. "
                    "Use 'uv run python' for python execution. "
                    "After writing code, call 'reload_tools' to apply it. "
                    "Be extremely precise and follow codebase patterns."
                ),
                max_turns=12,
                interactive=False
            )
            
            result = agent.run(task)
            return ToolResult(tool_name="delegate_coding", content=result.content, success=True)
        except Exception as e:
            logger.exception("Coding delegation failed")
            return ToolResult(tool_name="delegate_coding", content=f"Failed to delegate coding: {e}", success=False)


__all__ = ["DelegateBrowserTool", "DelegateResearchTool", "DelegateCodingTool"]
