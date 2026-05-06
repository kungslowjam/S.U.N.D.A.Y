"""Agents primitive — multi-turn reasoning and tool use."""

from __future__ import annotations

import logging

from sunday.agents._stubs import (
    AgentContext,
    AgentResult,
    BaseAgent,
    ToolUsingAgent,
)

logger = logging.getLogger(__name__)

# Import agent modules to trigger @AgentRegistry.register() decorators
try:
    import sunday.agents.simple  # noqa: F401
except ImportError:
    pass

try:
    import sunday.agents.orchestrator  # noqa: F401
except ImportError:
    pass

try:
    import sunday.agents.native_react  # noqa: F401
except ImportError:
    pass

try:
    import sunday.agents.native_openhands  # noqa: F401
except ImportError:
    pass

try:
    import sunday.agents.react  # noqa: F401 -- backward-compat shim
except ImportError:
    pass

try:
    import sunday.agents.openhands  # noqa: F401
except ImportError:
    pass

try:
    import sunday.agents.rlm  # noqa: F401
except ImportError:
    pass

try:
    import sunday.agents.claude_code  # noqa: F401
except ImportError:
    pass

try:
    import sunday.agents.operative  # noqa: F401
except ImportError:
    pass

try:
    import sunday.agents.monitor  # noqa: F401
except ImportError:
    pass

try:
    import sunday.agents.monitor_operative  # noqa: F401
except ImportError:
    pass

try:
    import sunday.agents.deep_research  # noqa: F401
except ImportError:
    pass

try:
    import sunday.agents.morning_digest  # noqa: F401
except ImportError:
    pass

# Registry alias: "react" -> NativeReActAgent (for backward compat)
try:
    from sunday.core.registry import AgentRegistry

    if AgentRegistry.contains("native_react") and not AgentRegistry.contains("react"):
        AgentRegistry.register_value("react", AgentRegistry.get("native_react"))
except Exception as exc:
    logger.debug("Registry alias 'react' creation skipped: %s", exc)

__all__ = ["AgentContext", "AgentResult", "BaseAgent", "ToolUsingAgent"]
