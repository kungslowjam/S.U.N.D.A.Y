"""Tools primitive — tool system with ABC interface and built-in tools."""

from __future__ import annotations

from sunday.core.registry import ToolRegistry
from sunday.tools._stubs import BaseTool, ToolExecutor, ToolSpec

# Lazy Tool Registration
# ----------------------
# To ensure near-instant CLI startup, we avoid importing heavy tool modules
# (and their dependencies like Playwright, PyTorch, or Faiss) at top-level.
# Instead, we register their keys and only import the module when requested.

_LAZY_TOOLS = {
    "calculator": "sunday.tools.calculator",
    "think": "sunday.tools.think",
    "retrieval": "sunday.tools.retrieval",
    "llm": "sunday.tools.llm_tool",
    "file_read": "sunday.tools.file_read",
    "web_search": "sunday.tools.web_search",
    "semantic_scholar_search": "sunday.tools.academic_search",
    "arxiv_search": "sunday.tools.academic_search",
    "openalex_search": "sunday.tools.academic_search",
    "code_interpreter": "sunday.tools.code_interpreter",
    "code_interpreter_docker": "sunday.tools.code_interpreter_docker",
    "repl": "sunday.tools.repl",
    "memory_store": "sunday.tools.storage_tools",
    "memory_retrieve": "sunday.tools.storage_tools",
    "memory_search": "sunday.tools.storage_tools",
    "memory_index": "sunday.tools.storage_tools",
    "channel_send": "sunday.tools.channel_tools",
    "channel_list": "sunday.tools.channel_tools",
    "channel_status": "sunday.tools.channel_tools",
    "http_request": "sunday.tools.http_request",
    "shell_exec": "sunday.tools.shell_exec",
    "memory_manage": "sunday.tools.memory_manage",
    "user_profile_manage": "sunday.tools.user_profile_manage",
    "skill_manage": "sunday.tools.skill_manage",
    "file_write": "sunday.tools.file_write",
    "apply_patch": "sunday.tools.apply_patch",
    "git_status": "sunday.tools.git_tool",
    "git_diff": "sunday.tools.git_tool",
    "git_commit": "sunday.tools.git_tool",
    "git_log": "sunday.tools.git_tool",
    "db_query": "sunday.tools.db_query",
    "pdf_extract": "sunday.tools.pdf_tool",
    "image_generate": "sunday.tools.image_tool",
    "audio_transcribe": "sunday.tools.audio_tool",
    "kg_add_entity": "sunday.tools.knowledge_tools",
    "kg_add_relation": "sunday.tools.knowledge_tools",
    "kg_query": "sunday.tools.knowledge_tools",
    "kg_neighbors": "sunday.tools.knowledge_tools",
    "knowledge_sql": "sunday.tools.knowledge_sql",
    "knowledge_search": "sunday.tools.knowledge_search",
    "text_to_speech": "sunday.tools.text_to_speech",
    "digest_collect": "sunday.tools.digest_collect",
    "browser_navigate": "sunday.tools.browser",
    "browser_reset": "sunday.tools.browser",
    "browser_click": "sunday.tools.browser",
    "browser_type": "sunday.tools.browser",
    "browser_screenshot": "sunday.tools.browser",
    "browser_extract": "sunday.tools.browser",
    "browser_get_elements": "sunday.tools.browser",
    "browser_drag": "sunday.tools.browser",
    "browser_scroll": "sunday.tools.browser",
    "browser_get_accessibility_tree": "sunday.tools.browser",
    "browser_use_task": "sunday.tools.browser_use_ext",
    "delegate_browser": "sunday.tools.subagents",
    "delegate_research": "sunday.tools.subagents",
    "scan_chunks": "sunday.tools.scan_chunks",
    "browser_axtree": "sunday.tools.browser_axtree",
    "graphify": "sunday.tools.graphify_tool",
    "create_tool_scaffold": "sunday.tools.scaffold_tool",
    "system_health": "sunday.tools.system_health_tool",
    "run_harness_test": "sunday.tools.harness_tool",
    "e2e_browser_test": "sunday.tools.harness_tool",
    "auto_self_test": "sunday.tools.harness_tool",
    "list_tools": "sunday.tools.meta_tool",
    "inspect_tool": "sunday.tools.meta_tool",
    "reload_tools": "sunday.tools.meta_tool",
}

for key, mod in _LAZY_TOOLS.items():
    ToolRegistry.register_lazy(key, mod)

def discover_tools() -> None:
    """Scan the tools directory and register any new tools found."""
    import os
    from pathlib import Path
    
    tools_dir = Path(__file__).parent
    for f in tools_dir.glob("*.py"):
        if f.name.startswith("_") or f.name == "__init__.py":
            continue
        
        # If the module isn't already handled by _LAZY_TOOLS, 
        # register it lazily using the filename as the key.
        # This ensures that even if we don't know the specific tool keys inside,
        # importing the module will trigger their @ToolRegistry.register decorators.
        mod_name = f"sunday.tools.{f.stem}"
        if mod_name not in _LAZY_TOOLS.values():
            # We use the stem as a 'trigger' key. 
            # When the AI asks for 'my_tool', if it's not found, 
            # we can have the registry try to load 'my_tool_tool.py'
            ToolRegistry.register_lazy(f.stem, mod_name)

# Run discovery on startup
discover_tools()

__all__ = ["BaseTool", "ToolExecutor", "ToolSpec"]
