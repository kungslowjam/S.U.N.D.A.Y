"""External-framework subprocess backends (Hermes Agent, OpenClaw)."""

from sunday.evals.backends.external.hermes_agent import HermesBackend
from sunday.evals.backends.external.openclaw import OpenClawBackend

__all__ = ["HermesBackend", "OpenClawBackend"]
