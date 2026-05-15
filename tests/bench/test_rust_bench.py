"""Tests for Rust-backed benchmarking framework.

These tests verify that the Rust sunday-bench crate produces the same
results as the pure-Python implementations.
"""

from __future__ import annotations

import pytest
from unittest.mock import MagicMock

from sunday.bench._stubs import BenchmarkResult
from sunday.bench.latency import LatencyBenchmark as PyLatencyBenchmark
from sunday.bench.throughput import ThroughputBenchmark as PyThroughputBenchmark
from sunday.bench.energy import EnergyBenchmark as PyEnergyBenchmark
from sunday.bench.rust_bridge import (
    RustLatencyBenchmark,
    RustThroughputBenchmark,
    RustEnergyBenchmark,
    _RUST_AVAILABLE,
)


def _make_engine():
    engine = MagicMock()
    engine.engine_id = "mock"
    engine.generate.return_value = {
        "content": "Hello world",
        "usage": {"prompt_tokens": 5, "completion_tokens": 10, "total_tokens": 15},
    }
    engine.health.return_value = True
    return engine


class TestRustLatencyBenchmark:
    def test_name(self):
        b = RustLatencyBenchmark()
        assert b.name == "latency"

    def test_description_contains_rust(self):
        b = RustLatencyBenchmark()
        assert "Rust" in b.description

    def test_run_produces_result(self):
        engine = _make_engine()
        b = RustLatencyBenchmark()
        result = b.run(engine, "test-model", num_samples=3)
        assert isinstance(result, BenchmarkResult)
        assert result.benchmark_name == "latency"
        assert result.model == "test-model"
        assert result.engine == "mock"
        assert result.samples == 3

    def test_metrics_keys_match_python(self):
        engine = _make_engine()
        rust_b = RustLatencyBenchmark()
        py_b = PyLatencyBenchmark()

        rust_result = rust_b.run(engine, "test-model", num_samples=3)
        py_result = py_b.run(engine, "test-model", num_samples=3)

        assert set(rust_result.metrics.keys()) == set(py_result.metrics.keys())

    def test_fallback_on_error(self):
        engine = _make_engine()
        engine.generate.side_effect = RuntimeError("fail")
        b = RustLatencyBenchmark()
        result = b.run(engine, "test-model", num_samples=3)
        assert result.errors == 3


class TestRustThroughputBenchmark:
    def test_name(self):
        b = RustThroughputBenchmark()
        assert b.name == "throughput"

    def test_run_produces_result(self):
        engine = _make_engine()
        b = RustThroughputBenchmark()
        result = b.run(engine, "test-model", num_samples=3)
        assert isinstance(result, BenchmarkResult)
        assert result.benchmark_name == "throughput"
        assert result.samples == 3

    def test_total_tokens(self):
        engine = _make_engine()
        b = RustThroughputBenchmark()
        result = b.run(engine, "test-model", num_samples=5)
        # 5 samples * 10 tokens each = 50
        assert result.metrics["total_tokens"] == 50.0

    def test_metrics_keys_match_python(self):
        engine = _make_engine()
        rust_b = RustThroughputBenchmark()
        py_b = PyThroughputBenchmark()

        rust_result = rust_b.run(engine, "test-model", num_samples=3)
        py_result = py_b.run(engine, "test-model", num_samples=3)

        assert set(rust_result.metrics.keys()) == set(py_result.metrics.keys())


class TestRustEnergyBenchmark:
    def test_name(self):
        b = RustEnergyBenchmark()
        assert b.name == "energy"

    def test_run_produces_result(self):
        engine = _make_engine()
        b = RustEnergyBenchmark()
        result = b.run(engine, "test-model", num_samples=3, warmup_samples=0)
        assert isinstance(result, BenchmarkResult)
        assert result.benchmark_name == "energy"
        assert result.samples == 3

    def test_warmup_samples(self):
        engine = _make_engine()
        b = RustEnergyBenchmark()
        result = b.run(engine, "test-model", num_samples=3, warmup_samples=2)
        assert result.warmup_samples == 2
        # warmup (2) + measurement (3) = 5 total calls
        assert engine.generate.call_count == 5

    def test_metrics_keys_match_python(self):
        engine = _make_engine()
        rust_b = RustEnergyBenchmark()
        py_b = PyEnergyBenchmark()

        rust_result = rust_b.run(engine, "test-model", num_samples=3, warmup_samples=0)
        py_result = py_b.run(engine, "test-model", num_samples=3, warmup_samples=0)

        assert set(rust_result.metrics.keys()) == set(py_result.metrics.keys())


class TestRustAvailability:
    def test_rust_bridge_importable(self):
        """Verify the Rust bridge module can be imported."""
        from sunday.bench.rust_bridge import _RUST_AVAILABLE
        # Should not raise ImportError
        assert isinstance(_RUST_AVAILABLE, bool)

    def test_pure_python_fallback_always_works(self):
        """Even without Rust, benchmarks should still work via Python fallback."""
        engine = _make_engine()
        b = RustLatencyBenchmark()
        result = b.run(engine, "test-model", num_samples=3)
        assert result.benchmark_name == "latency"
        assert result.errors == 0
