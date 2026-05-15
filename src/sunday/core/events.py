"""Thread-safe pub/sub event bus for inter-primitive telemetry.

Extends IPW's ``EventRecorder`` into a full publish/subscribe system so that
any primitive can emit events (e.g. ``INFERENCE_END``) and any other primitive can
react without direct coupling.
"""

from __future__ import annotations

import threading
import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Callable, Dict, List, Optional  # noqa: I001

# ---------------------------------------------------------------------------
# Event taxonomy
# ---------------------------------------------------------------------------


class EventType(str, Enum):
    """Supported event categories."""

    INFERENCE_START = "inference_start"
    INFERENCE_END = "inference_end"
    TOOL_CALL_START = "tool_call_start"
    TOOL_CALL_END = "tool_call_end"
    MEMORY_STORE = "memory_store"
    MEMORY_RETRIEVE = "memory_retrieve"
    AGENT_TURN_START = "agent_turn_start"
    AGENT_TURN_END = "agent_turn_end"
    TELEMETRY_RECORD = "telemetry_record"
    TRACE_STEP = "trace_step"
    TRACE_COMPLETE = "trace_complete"
    CHANNEL_MESSAGE_RECEIVED = "channel_message_received"
    CHANNEL_MESSAGE_SENT = "channel_message_sent"
    SECURITY_SCAN = "security_scan"
    SECURITY_ALERT = "security_alert"
    SECURITY_BLOCK = "security_block"
    SCHEDULER_TASK_START = "scheduler_task_start"
    SCHEDULER_TASK_END = "scheduler_task_end"
    BATCH_START = "batch_start"
    BATCH_END = "batch_end"
    # Phase 14 — Agent Hardening & Security
    TOOL_TIMEOUT = "tool_timeout"
    LOOP_GUARD_TRIGGERED = "loop_guard_triggered"
    CAPABILITY_DENIED = "capability_denied"
    TAINT_VIOLATION = "taint_violation"
    # Phase 15 — Workflow, Skills, Sessions
    WORKFLOW_START = "workflow_start"
    WORKFLOW_NODE_START = "workflow_node_start"
    WORKFLOW_NODE_END = "workflow_node_end"
    WORKFLOW_END = "workflow_end"
    SKILL_EXECUTE_START = "skill_execute_start"
    SKILL_EXECUTE_END = "skill_execute_end"
    SESSION_START = "session_start"
    SESSION_END = "session_end"
    # Phase 16 — A2A Protocol
    A2A_TASK_RECEIVED = "a2a_task_received"
    A2A_TASK_COMPLETED = "a2a_task_completed"
    # Phase 22 — Operators
    OPERATOR_TICK_START = "operator_tick_start"
    OPERATOR_TICK_END = "operator_tick_end"
    # Managed agent lifecycle (distinct from OPERATOR_TICK_* for the operator subsystem)
    AGENT_TICK_START = "agent_tick_start"
    AGENT_TICK_END = "agent_tick_end"
    AGENT_TICK_ERROR = "agent_tick_error"
    AGENT_BUDGET_EXCEEDED = "agent_budget_exceeded"
    AGENT_STALL_DETECTED = "agent_stall_detected"
    AGENT_LEARNING_STARTED = "agent_learning_started"
    AGENT_LEARNING_COMPLETED = "agent_learning_completed"
    AGENT_MESSAGE_RECEIVED = "agent_message_received"
    AGENT_CHECKPOINT_SAVED = "agent_checkpoint_saved"
    # Phase 25 — Configuration Optimization
    OPTIMIZE_RUN_START = "optimize_run_start"
    OPTIMIZE_TRIAL_START = "optimize_trial_start"
    OPTIMIZE_TRIAL_END = "optimize_trial_end"
    OPTIMIZE_RUN_END = "optimize_run_end"
    FEEDBACK_RECEIVED = "feedback_received"


@dataclass(slots=True)
class Event:
    """A single event published on the bus."""

    event_type: EventType
    timestamp: float
    data: Dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_rust(cls, rust_event: Any) -> Event:
        """Convert a Rust Event object to a Python Event dataclass."""
        raw_type = str(rust_event.event_type)
        
        # Robust parsing: handle "EventType.NAME", "AGENT_TURN_START", or "agent_turn_start"
        clean_type = raw_type.split(".")[-1].lower()
        
        try:
            # Try to match by value (e.g. "agent_turn_start")
            et = EventType(clean_type)
        except ValueError:
            # Fallback: try to match by name (e.g. if clean_type is "agent_turn_start" but we need "AGENT_TURN_START")
            # Enums in Python can be tricky; let's try direct attribute access if it matches uppercase
            try:
                et = EventType[clean_type.upper()]
            except (KeyError, ValueError):
                # If all fails, use a generic record or re-raise with context
                print(f"[WARN] Unknown event type from Rust: {raw_type} -> {clean_type}")
                # We'll try to find any member that matches the value case-insensitively
                for member in EventType:
                    if member.value.lower() == clean_type:
                        et = member
                        break
                else:
                    raise ValueError(f"'{raw_type}' is not a valid EventType (tried value '{clean_type}')")

        return cls(
            event_type=et,
            timestamp=rust_event.timestamp,
            data=rust_event.data,
        )


# Type alias for subscriber callbacks
Subscriber = Callable[[Event], None]


# ---------------------------------------------------------------------------
# EventBus
# ---------------------------------------------------------------------------


class EventBus:
    """High-performance Rust-backed publish/subscribe event bus."""

    def __init__(self, *, record_history: bool = False) -> None:
        from sunday._rust_bridge import get_rust_module
        mod = get_rust_module()
        # Rust PyO3 __new__ may not accept positional args; try keyword or fallback
        try:
            self._rust_bus = mod.EventBus(record_history=record_history)
        except TypeError:
            # Fallback: call without args and ignore record_history
            self._rust_bus = mod.EventBus()
        self._record_history = record_history
        # Keep track of callbacks to prevent garbage collection and allow unsubscription
        self._callbacks: Dict[EventType, List[Subscriber]] = {}
        self._lock = threading.Lock()

    def subscribe(self, event_type: EventType, callback: Subscriber) -> None:
        """Register *callback* to be called whenever *event_type* is published."""
        with self._lock:
            self._callbacks.setdefault(event_type, []).append(callback)
        
        # Define a wrapper that converts Rust Event back to Python dataclass
        def _callback_wrapper(rust_event: Any) -> None:
            callback(Event.from_rust(rust_event))
            
        self._rust_bus.subscribe(event_type.value, _callback_wrapper)

    def unsubscribe(self, event_type: EventType, callback: Subscriber) -> None:
        """Remove *callback* (Note: Rust-level unsubscription not yet implemented)."""
        with self._lock:
            listeners = self._callbacks.get(event_type, [])
            try:
                listeners.remove(callback)
            except ValueError:
                pass

    def publish(
        self,
        event_type: EventType,
        data: Optional[Dict[str, Any]] = None,
    ) -> Event:
        """Dispatch an event via the Rust core."""
        rust_event = self._rust_bus.publish(event_type.value, data or {})
        return Event.from_rust(rust_event)

    @property
    def history(self) -> List[Event]:
        """Return all recorded events from the Rust history."""
        return [Event.from_rust(e) for e in self._rust_bus.history()]

    def clear_history(self) -> None:
        """Discard all recorded events in Rust."""
        self._rust_bus.clear_history()


# ---------------------------------------------------------------------------
# Module-level singleton
# ---------------------------------------------------------------------------

_bus: Optional[EventBus] = None
_bus_lock = threading.Lock()


def get_event_bus(*, record_history: bool = False) -> EventBus:
    """Return the module-level ``EventBus`` singleton, creating it if needed."""
    global _bus
    with _bus_lock:
        if _bus is None:
            _bus = EventBus(record_history=record_history)
        return _bus


def reset_event_bus() -> None:
    """Replace the singleton with a fresh instance (for tests)."""
    global _bus
    with _bus_lock:
        _bus = None


__all__ = [
    "Event",
    "EventBus",
    "EventType",
    "Subscriber",
    "get_event_bus",
    "reset_event_bus",
]
