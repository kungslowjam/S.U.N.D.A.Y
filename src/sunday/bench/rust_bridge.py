"""Python wrapper to call Rust sunday-bench via sunday_rust PyO3 bridge.

Falls back to pure-Python implementations if the Rust bridge is not available.
"""

from __future__ import annotations

import json
import logging
import time
from typing import Any, Dict, List, Optional

from sunday.bench._stubs import BaseBenchmark, BenchmarkResult, BenchmarkSuite
from sunday.bench._stats import compute_stats
from sunday.core.registry import BenchmarkRegistry
from sunday.core.types import Message, Role
from sunday.engine._stubs import InferenceEngine

logger = logging.getLogger(__name__)

# Try to import Rust bridge
try:
    import sunday_rust
    _RUST_AVAILABLE = True
except ImportError:
    sunday_rust = None  # type: ignore
    _RUST_AVAILABLE = False


class RustBenchmarkResult:
    """Wrapper that exposes Rust BenchmarkResult with same API as Python."""

    def __init__(self, inner: Any) -> None:
        self._inner = inner

    @property
    def benchmark_name(self) -> str:
        return self._inner.benchmark_name

    @property
    def model(self) -> str:
        return self._inner.model

    @property
    def engine(self) -> str:
        return self._inner.engine

    @property
    def metrics(self) -> Dict[str, float]:
        return dict(self._inner.metrics)

    @property
    def metadata(self) -> Dict[str, Any]:
        return dict(self._inner.metadata)

    @property
    def samples(self) -> int:
        return self._inner.samples

    @property
    def errors(self) -> int:
        return self._inner.errors

    @property
    def warmup_samples(self) -> int:
        return self._inner.warmup_samples

    @property
    def steady_state_samples(self) -> int:
        return self._inner.steady_state_samples

    @property
    def steady_state_reached(self) -> bool:
        return self._inner.steady_state_reached

    @property
    def total_energy_joules(self) -> float:
        return self._inner.total_energy_joules

    @property
    def energy_per_token_joules(self) -> float:
        return self._inner.energy_per_token_joules

    @property
    def energy_method(self) -> str:
        return self._inner.energy_method

    def to_json(self) -> str:
        return self._inner.to_json()

    def __repr__(self) -> str:
        return repr(self._inner)


class RustLatencyBenchmark(BaseBenchmark):
    """Latency benchmark backed by Rust implementation."""

    @property
    def name(self) -> str:
        return "latency"

    @property
    def description(self) -> str:
        return "Measures per-call inference latency with short prompts (Rust)"

    def run(
        self,
        engine: InferenceEngine,
        model: str,
        *,
        num_samples: int = 10,
        warmup_samples: int = 0,
        **kwargs: Any,
    ) -> BenchmarkResult:
        if _RUST_AVAILABLE and hasattr(sunday_rust, "LatencyBenchmark"):
            try:
                # Wrap Python engine for Rust
                rust_engine = _PyEngineWrapper(engine)
                bench = sunday_rust.LatencyBenchmark()
                result = bench.run(rust_engine, model, num_samples, warmup_samples)
                return _rust_result_to_python(result)
            except Exception as exc:
                logger.debug("Rust latency benchmark failed, falling back to Python: %s", exc)

        # Fallback to Python implementation
        from sunday.bench.latency import LatencyBenchmark
        return LatencyBenchmark().run(engine, model, num_samples=num_samples, warmup_samples=warmup_samples)


class RustThroughputBenchmark(BaseBenchmark):
    """Throughput benchmark backed by Rust implementation."""

    @property
    def name(self) -> str:
        return "throughput"

    @property
    def description(self) -> str:
        return "Measures inference throughput in tokens per second (Rust)"

    def run(
        self,
        engine: InferenceEngine,
        model: str,
        *,
        num_samples: int = 10,
        warmup_samples: int = 0,
        **kwargs: Any,
    ) -> BenchmarkResult:
        if _RUST_AVAILABLE and hasattr(sunday_rust, "ThroughputBenchmark"):
            try:
                rust_engine = _PyEngineWrapper(engine)
                bench = sunday_rust.ThroughputBenchmark()
                result = bench.run(rust_engine, model, num_samples, warmup_samples)
                return _rust_result_to_python(result)
            except Exception as exc:
                logger.debug("Rust throughput benchmark failed, falling back to Python: %s", exc)

        from sunday.bench.throughput import ThroughputBenchmark
        return ThroughputBenchmark().run(engine, model, num_samples=num_samples, warmup_samples=warmup_samples)


class RustEnergyBenchmark(BaseBenchmark):
    """Energy benchmark backed by Rust implementation."""

    @property
    def name(self) -> str:
        return "energy"

    @property
    def description(self) -> str:
        return "Measures energy per token at thermal equilibrium (Rust)"

    def run(
        self,
        engine: InferenceEngine,
        model: str,
        *,
        num_samples: int = 10,
        warmup_samples: int = 5,
        **kwargs: Any,
    ) -> BenchmarkResult:
        if _RUST_AVAILABLE and hasattr(sunday_rust, "EnergyBenchmark"):
            try:
                rust_engine = _PyEngineWrapper(engine)
                bench = sunday_rust.EnergyBenchmark()
                result = bench.run(rust_engine, model, num_samples, warmup_samples)
                return _rust_result_to_python(result)
            except Exception as exc:
                logger.debug("Rust energy benchmark failed, falling back to Python: %s", exc)

        from sunday.bench.energy import EnergyBenchmark
        return EnergyBenchmark().run(engine, model, num_samples=num_samples, warmup_samples=warmup_samples)


class RustBenchmarkSuite(BenchmarkSuite):
    """Benchmark suite that prefers Rust implementations."""

    def __init__(self, benchmarks: Optional[List[BaseBenchmark]] = None) -> None:
        # Use Rust-backed benchmarks by default
        if benchmarks is None:
            benchmarks = [
                RustLatencyBenchmark(),
                RustThroughputBenchmark(),
                RustEnergyBenchmark(),
            ]
        super().__init__(benchmarks)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

class _PyEngineWrapper:
    """Wraps a Python InferenceEngine for the Rust bridge.

    The Rust bridge expects an engine with ``engine_id()``, ``generate()``
    and ``health()`` methods.  We forward calls to the Python engine.
    """

    def __init__(self, engine: InferenceEngine) -> None:
        self._engine = engine

    @property
    def engine_id(self) -> str:
        return self._engine.engine_id

    def generate(
        self,
        messages: List[Dict[str, Any]],
        model: str,
        temperature: float = 0.7,
        max_tokens: int = 256,
    ) -> Dict[str, Any]:
        core_msgs = [
            Message(role=Role(m["role"]), content=m.get("content", ""))
            for m in messages
        ]
        result = self._engine.generate(core_msgs, model=model)
        # Normalize result to dict
        if isinstance(result, dict):
            return result
        # If result is a GenerateResult-like object
        return {
            "content": getattr(result, "content", ""),
            "usage": getattr(result, "usage", {}),
        }

    def health(self) -> bool:
        return self._engine.health()


def _rust_result_to_python(rust_result: Any) -> BenchmarkResult:
    """Convert a Rust BenchmarkResult (PyO3) to Python BenchmarkResult."""
    return BenchmarkResult(
        benchmark_name=rust_result.benchmark_name,
        model=rust_result.model,
        engine=rust_result.engine,
        metrics=dict(rust_result.metrics),
        metadata=dict(rust_result.metadata),
        samples=rust_result.samples,
        errors=rust_result.errors,
        warmup_samples=getattr(rust_result, "warmup_samples", 0),
        steady_state_samples=getattr(rust_result, "steady_state_samples", 0),
        steady_state_reached=getattr(rust_result, "steady_state_reached", False),
        total_energy_joules=getattr(rust_result, "total_energy_joules", 0.0),
        energy_per_token_joules=getattr(rust_result, "energy_per_token_joules", 0.0),
        energy_method=getattr(rust_result, "energy_method", ""),
    )


def ensure_rust_registered() -> None:
    """Register Rust benchmarks in the Python BenchmarkRegistry."""
    if not BenchmarkRegistry.contains("latency_rust"):
        BenchmarkRegistry.register_value("latency_rust", RustLatencyBenchmark)
    if not BenchmarkRegistry.contains("throughput_rust"):
        BenchmarkRegistry.register_value("throughput_rust", RustThroughputBenchmark)
    if not BenchmarkRegistry.contains("energy_rust"):
        BenchmarkRegistry.register_value("energy_rust", RustEnergyBenchmark)
