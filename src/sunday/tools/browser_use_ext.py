"""Integration with the browser-use library for advanced autonomous web tasks."""

from __future__ import annotations
import asyncio
import os
import threading
from typing import Any

from sunday.core.registry import ToolRegistry
from sunday.core.types import ToolResult
from sunday.tools._stubs import BaseTool, ToolSpec


# ---------------------------------------------------------------------------
# LLMProxy: Duck-typed wrapper that satisfies browser-use without Pydantic
# ---------------------------------------------------------------------------
class LLMProxy:
    """
    Wraps ChatOpenAI to guarantee all attributes browser-use expects:
    - .provider (str)
    - .model (str)
    - .invoke(...)
    - .ainvoke(...)
    
    Uses object.__setattr__ to avoid any Pydantic interference.
    Uses object.__getattribute__ inside __getattr__ to avoid recursion.
    """
    def __init__(self, base_url: str, model: str):
        from langchain_openai import ChatOpenAI
        # Use object.__setattr__ to bypass any descriptor magic
        object.__setattr__(self, '_llm', ChatOpenAI(
            base_url=base_url,
            api_key="sk-no-key-needed",
            model=model,
            max_tokens=1024,
        ))
        # Explicitly set fields browser-use reads
        object.__setattr__(self, 'provider', 'openai')
        object.__setattr__(self, 'model', model)
        object.__setattr__(self, 'model_name', model)

    def invoke(self, *args, **kwargs):
        return object.__getattribute__(self, '_llm').invoke(*args, **kwargs)

    async def ainvoke(self, *args, **kwargs):
        """Async invoke — wraps sync in executor."""
        llm = object.__getattribute__(self, '_llm')
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(None, lambda: llm.invoke(*args, **kwargs))

    def __getattr__(self, name):
        """Forward any other attribute access to the wrapped ChatOpenAI."""
        try:
            return getattr(object.__getattribute__(self, '_llm'), name)
        except AttributeError:
            raise AttributeError(f"'LLMProxy' object has no attribute '{name}'")


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
                " Use this for ANY multi-step web search or data extraction task."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "The natural language description of the task to perform.",
                    },
                    "start_url": {
                        "type": "string",
                        "description": "Optional URL to open before starting the task.",
                    },
                    "llm_base_url": {
                        "type": "string",
                        "description": "Optional OpenAI-compatible base URL for the browser-use controller LLM.",
                    },
                    "model": {
                        "type": "string",
                        "description": "Optional model name for the browser-use controller LLM.",
                    },
                },
                "required": ["task"],
            },
            category="browser",
            timeout_seconds=300.0,
        )

    def execute(self, **params: Any) -> ToolResult:
        task = params.get("task", "")
        if not task:
            return ToolResult(tool_name="browser_use_task", content="No task provided.", success=False)

        start_url = str(params.get("start_url") or "").strip()
        if start_url and start_url not in task:
            task = f"Open {start_url} first.\n\n{task}"

        llm_base_url = (
            params.get("llm_base_url")
            or os.environ.get("SUNDAY_BROWSER_USE_LLM_BASE_URL")
            or "http://127.0.0.1:8081/v1"
        )
        model = (
            params.get("model")
            or os.environ.get("SUNDAY_BROWSER_USE_MODEL")
            or "local-model"
        )
        headless = os.environ.get("SUNDAY_BROWSER_USE_HEADLESS", "0") == "1"

        try:
            from browser_use import Agent  # noqa: F401
            llm = LLMProxy(base_url=llm_base_url, model=model)
        except Exception as e:
            return ToolResult(tool_name="browser_use_task", content=f"Init error: {e}", success=False)

        async def _run():
            from browser_use import Agent
            from browser_use.browser import BrowserSession
            print(f"[🤖 BROWSER-USE] Starting autonomous agent...")
            browser = BrowserSession(headless=headless)
            agent = Agent(task=task, llm=llm, browser_session=browser)
            try:
                result = await agent.run()
                return result
            finally:
                await browser.stop()

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
        timeout = float(self.spec.timeout_seconds)
        t.join(timeout=timeout)

        if "error" in result_holder:
            return ToolResult(
                tool_name="browser_use_task",
                content=f"Error: {result_holder['error']}",
                success=False,
            )
        if t.is_alive():
            return ToolResult(
                tool_name="browser_use_task",
                content=f"Timed out after {timeout:.0f}s.",
                success=False,
            )
        return ToolResult(
            tool_name="browser_use_task",
            content=str(result_holder.get("result", "Task completed.")),
            success=True,
        )


__all__ = ["BrowserUseTaskTool"]
