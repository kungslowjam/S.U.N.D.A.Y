"""Python wrapper to call Rust sunday-system via sunday_rust PyO3 bridge.

Falls back to pure-Python implementations if the Rust bridge is not available.
"""

from __future__ import annotations

import logging
from typing import TYPE_CHECKING, Any, Dict, Optional

if TYPE_CHECKING:
    from sunday.system.core import JarvisSystem
    from sunday.system.builder import SystemBuilder
    from sunday.system.orchestrator import QueryOrchestrator

logger = logging.getLogger(__name__)

# Try to import Rust bridge
try:
    import sunday_rust
    _RUST_AVAILABLE = True
except ImportError:
    sunday_rust = None  # type: ignore
    _RUST_AVAILABLE = False


def rust_jarvis_system() -> Optional[Any]:
    """Get Rust JarvisSystem class if available."""
    if _RUST_AVAILABLE and hasattr(sunday_rust, "JarvisSystem"):
        return sunday_rust.JarvisSystem
    return None


def rust_system_builder() -> Optional[Any]:
    """Get Rust SystemBuilder class if available."""
    if _RUST_AVAILABLE and hasattr(sunday_rust, "SystemBuilder"):
        return sunday_rust.SystemBuilder
    return None


def rust_query_orchestrator() -> Optional[Any]:
    """Get Rust QueryOrchestrator class if available."""
    if _RUST_AVAILABLE and hasattr(sunday_rust, "QueryOrchestrator"):
        return sunday_rust.QueryOrchestrator
    return None


def detect_agent_intent(query: str) -> Optional[str]:
    """Detect agent intent from query using Rust if available."""
    if _RUST_AVAILABLE and hasattr(sunday_rust, "QueryOrchestrator"):
        try:
            orch = sunday_rust.QueryOrchestrator()
            return orch.detect_agent_intent(query)
        except Exception as exc:
            logger.debug("Rust intent detection failed, using Python: %s", exc)

    # Python fallback
    lower = query.lower()
    if "morning digest" in lower or "daily summary" in lower:
        return "morning_digest"
    elif "deep research" in lower:
        return "deep_research"
    return None
