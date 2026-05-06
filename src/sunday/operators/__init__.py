"""Operators — persistent, scheduled autonomous agents."""

from sunday.operators.loader import load_operator
from sunday.operators.manager import OperatorManager
from sunday.operators.types import OperatorManifest

__all__ = ["OperatorManifest", "OperatorManager", "load_operator"]
