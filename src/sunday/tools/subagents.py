"""Specialized Sub-Agents for delegation."""
from __future__ import annotations

import json
from typing import Any

from sunday.core.registry import ToolRegistry
from sunday.core.types import ToolResult
from sunday.tools._stubs import BaseTool, ToolSpec

@ToolRegistry.register("delegate_browser")
class DelegateBrowserTool(BaseTool):
    tool_id = "delegate_browser"

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="delegate_browser",
            description=(
                "Delegate a complex web browsing or scraping task to the Browser Sub-Agent. "
                "Use this when you need to search the web, navigate websites, or extract data from pages. "
                "The sub-agent will handle all navigation and return the final results to you."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "A highly detailed description of the task for the Browser Sub-Agent to perform. Include exactly what data you need.",
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
            from sunday.engine.factory import get_engine
            from sunday.agents.orchestrator import OrchestratorAgent
            
            config = load_config()
            engine = get_engine("multi", config)
            
            # Browser specific tools
            browser_tool_names = [
                "browser_navigate", "browser_click", "browser_type", 
                "browser_get_elements", "browser_extract", "web_search", "browser_get_accessibility_tree"
            ]
            
            tools = []
            for t_name in browser_tool_names:
                cls = ToolRegistry.get(t_name)
                if cls:
                    tools.append(cls())
                    
            system_prompt = (
                "You are a specialized Browser Sub-Agent. Your ONLY job is to execute the given web task, "
                "extract the requested information, and return a final answer. "
                "Do NOT attempt to do anything outside of browsing and searching."
            )
                    
            agent = OrchestratorAgent(
                engine=engine,
                model=config.intelligence.default_model,
                tools=tools,
                system_prompt=system_prompt,
                max_turns=10,
            )
            
            print(f"🤖 [SUB-AGENT] Spawning BrowserAgent for: {task[:50]}...")
            result = agent.run(task)
            print(f"✅ [SUB-AGENT] BrowserAgent finished.")
            
            return ToolResult(
                tool_name="delegate_browser",
                content=f"Browser Sub-Agent Final Answer:\n{result.content}",
                success=True
            )
        except Exception as e:
            return ToolResult(tool_name="delegate_browser", content=f"Failed to run sub-agent: {e}", success=False)


@ToolRegistry.register("delegate_research")
class DelegateResearchTool(BaseTool):
    tool_id = "delegate_research"

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="delegate_research",
            description=(
                "Delegate a deep research task to the Research Sub-Agent. "
                "Use this when you need comprehensive research spanning multiple topics or deep document analysis."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "Detailed research instructions.",
                    }
                },
                "required": ["task"],
            },
            category="agents",
        )

    def execute(self, **params: Any) -> ToolResult:
        task = params.get("task", "")
        if not task:
            return ToolResult(tool_name="delegate_research", content="Task cannot be empty.", success=False)
            
        try:
            from sunday.core.config import load_config
            from sunday.engine.factory import get_engine
            from sunday.agents.orchestrator import OrchestratorAgent
            
            config = load_config()
            engine = get_engine("multi", config)
            
            research_tool_names = [
                "web_search", "academic_search", "knowledge_search", 
                "http_request", "pdf_read"
            ]
            
            tools = []
            for t_name in research_tool_names:
                cls = ToolRegistry.get(t_name)
                if cls:
                    tools.append(cls())
                    
            system_prompt = (
                "You are a specialized Research Sub-Agent. Your job is to deeply investigate the given topic, "
                "cross-reference facts, and provide a highly detailed and accurate research report."
            )
                    
            agent = OrchestratorAgent(
                engine=engine,
                model=config.intelligence.default_model,
                tools=tools,
                system_prompt=system_prompt,
                max_turns=10,
            )
            
            print(f"🤖 [SUB-AGENT] Spawning ResearchAgent for: {task[:50]}...")
            result = agent.run(task)
            print(f"✅ [SUB-AGENT] ResearchAgent finished.")
            
            return ToolResult(
                tool_name="delegate_research",
                content=f"Research Sub-Agent Report:\n{result.content}",
                success=True
            )
        except Exception as e:
            return ToolResult(tool_name="delegate_research", content=f"Failed to run sub-agent: {e}", success=False)
