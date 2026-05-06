"""Tools primitive — tool system with ABC interface and built-in tools."""

from __future__ import annotations

from sunday.tools._stubs import BaseTool, ToolExecutor, ToolSpec

# Import built-in tools to trigger @ToolRegistry.register() decorators.
# Each is wrapped in try/except so the package loads even before the
# individual tool modules are created.
try:
    import sunday.tools.calculator  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.think  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.retrieval  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.llm_tool  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.file_read  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.web_search  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.code_interpreter  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.code_interpreter_docker  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.repl  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.storage_tools  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.mcp_adapter  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.channel_tools  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.http_request  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.shell_exec  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.memory_manage  # noqa: F401
except ImportError:
    pass
try:
    import sunday.tools.user_profile_manage  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.skill_manage  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.file_write  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.apply_patch  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.git_tool  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.db_query  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.pdf_tool  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.image_tool  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.audio_tool  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.knowledge_tools  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.text_to_speech  # noqa: F401
except ImportError:
    pass

try:
    import sunday.tools.digest_collect  # noqa: F401
except ImportError:
    pass

__all__ = ["BaseTool", "ToolExecutor", "ToolSpec"]
