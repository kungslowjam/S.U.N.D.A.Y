"""Bundle dataclasses that group cohesive subsystems of JarvisSystem."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Optional

if TYPE_CHECKING:
    from sunday.agents._stubs import BaseAgent
    from sunday.agents.executor import AgentExecutor
    from sunday.agents.manager import AgentManager
    from sunday.agents.scheduler import AgentScheduler
    from sunday.scheduler.scheduler import TaskScheduler
    from sunday.scheduler.store import SchedulerStore
    from sunday.security.audit import AuditLogger
    from sunday.security.boundary import BoundaryGuard
    from sunday.security.capabilities import CapabilityPolicy
    from sunday.telemetry.gpu_monitor import GpuMonitor
    from sunday.telemetry.store import TelemetryStore
    from sunday.traces.collector import TraceCollector
    from sunday.traces.store import TraceStore


@dataclass
class SecurityContext:
    """Security policy, audit, and boundary enforcement."""

    capability_policy: Optional[CapabilityPolicy] = None
    audit_logger: Optional[AuditLogger] = None
    boundary_guard: Optional[BoundaryGuard] = None


@dataclass
class Observability:
    """Telemetry, traces, and hardware monitoring."""

    telemetry_store: Optional[TelemetryStore] = None
    trace_store: Optional[TraceStore] = None
    trace_collector: Optional[TraceCollector] = None
    gpu_monitor: Optional[GpuMonitor] = None


@dataclass
class AgentRuntime:
    """Active agent and agent lifecycle managers."""

    agent: Optional[BaseAgent] = None
    agent_name: str = ""
    manager: Optional[AgentManager] = None
    scheduler: Optional[AgentScheduler] = None
    executor: Optional[AgentExecutor] = None


@dataclass
class Scheduling:
    """Task scheduler and its persistent store."""

    store: Optional[SchedulerStore] = None
    runner: Optional[TaskScheduler] = None
