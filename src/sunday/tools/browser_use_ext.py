"""Integration with the browser-use library for advanced autonomous web tasks."""

from __future__ import annotations
import asyncio
import threading
from typing import Any

from sunday.core.registry import ToolRegistry
from sunday.core.types import ToolResult
from sunday.tools._stubs import BaseTool, ToolSpec


# ---------------------------------------------------------------------------
# LLMProxy: A robust wrapper to satisfy browser-use and Pydantic
# ---------------------------------------------------------------------------
class LLMProxy:
    """Wraps ChatOpenAI to provide required fields and async methods for browser-use."""
    def __init__(self, base_url: str, model: str):
        from langchain_openai import ChatOpenAI
        self._llm = ChatOpenAI(
            base_url=base_url,
            api_key="sk-no-key-needed",
            model=model,
            max_tokens=1024
        )
        # browser-use reads this property
        self.provider = "openai"

    def invoke(self, *args, **kwargs):
        return self._llm.invoke(*args, **kwargs)

    async def ainvoke(self, *args, **kwargs):
        """The critical missing piece for browser-use."""
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(
            None, lambda: self._llm.invoke(*args, **kwargs)
        )
    
    # Forward other common calls if needed
    def __getattr__(self, name):
        return getattr(self._llm, name)


@ToolRegistry.register("browser_use_task")
class BrowserUseTaskTool(BaseTool):
    """Executes a complex browser task using the browser-use library."""

    tool_id = "browser_use_task"

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="browser_use_task",
            description=(
                "Executes a complex, multi-step autonomous browser task using the browser-use library."
                " Use this for ANY multi-step web search or data extraction task. It is the preferred way."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "The natural language description of the task to perform.",
                    },
                },
                "required": ["task"],
            },
            category="browser",
        )

    def execute(self, **params: Any) -> ToolResult:
        task = params.get("task", "")
        if not task:
            return ToolResult(tool_name="browser_use_task", content="No task provided.", success=False)

        try:
            from browser_use import Agent
            # AI Engine is on 8081
            llm = LLMProxy(base_url="http://127.0.0.1:8081/v1", model="local-model")
        except Exception as e:
            return ToolResult(tool_name="browser_use_task", content=f"Initialization error: {e}", success=False)

        async def _run():
            agent = Agent(task=task, llm=llm)
            return await agent.run()

        result_holder: dict = {}

        def _run_in_thread():
            try:
                import nest_asyncio
                nest_asyncio.apply()
            except ImportError:
                pass
            new_loop = asyncio.new_event_loop()
            asyncio.set_event_loop(new_loop)
            try:
                result_holder["result"] = new_loop.run_until_complete(_run())
            except Exception as e:
                result_holder["error"] = str(e)
            finally:
                new_loop.close()

        t = threading.Thread(target=_run_in_thread, daemon=True)
        t.start()
        t.join(timeout=300)

        if "error" in result_holder:
            return ToolResult(
                tool_name="browser_use_task",
                content=f"Error executing browser-use task: {result_holder['error']}",
                success=False,
            )
        return ToolResult(
            tool_name="browser_use_task",
            content=str(result_holder.get("result", "Task completed.")),
            success=True,
        )


__all__ = ["BrowserUseTaskTool"]
