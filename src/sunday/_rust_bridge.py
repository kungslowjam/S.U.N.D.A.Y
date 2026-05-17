"""Single point of contact between Python and the Rust ``sunday_rust`` module.

Every Python module that wants to delegate to Rust should import helpers from
here rather than importing ``sunday_rust`` directly.  The Rust backend is
mandatory — if it cannot be imported, a hard ``ImportError`` is raised.
"""

from __future__ import annotations

import functools
import json
from typing import TYPE_CHECKING, Any, List, Optional

if TYPE_CHECKING:
    import types as _types

# ---------------------------------------------------------------------------
# Mandatory import — Rust backend is required
# ---------------------------------------------------------------------------


@functools.lru_cache(maxsize=1)
def get_rust_module() -> _types.ModuleType:
    """Return the ``sunday_rust`` module.

    Raises ``ImportError`` if the compiled extension is not available.
    The Rust backend is mandatory for all modules that have Rust
    implementations — there is no Python fallback.
    """
    import sunday_rust  # type: ignore[import-untyped]

    return sunday_rust


RUST_AVAILABLE: bool = True


# ---------------------------------------------------------------------------
# JSON -> Python dataclass converters
# ---------------------------------------------------------------------------


def scan_result_from_json(json_str: str) -> object:
    """Convert a Rust scanner JSON string to a Python ``ScanResult``."""
    from sunday.security.types import (
        ScanFinding,
        ScanResult,
        ThreatLevel,
    )

    data = json.loads(json_str)
    findings: List[ScanFinding] = []
    for f in data.get("findings", []):
        findings.append(
            ScanFinding(
                pattern_name=f.get("pattern_name", ""),
                matched_text=f.get("matched_text", ""),
                threat_level=ThreatLevel(
                    f.get("threat_level", "low").lower(),
                ),
                start=f.get("start", 0),
                end=f.get("end", 0),
                description=f.get("description", ""),
            )
        )
    return ScanResult(findings=findings)


def injection_result_from_json(json_str: str) -> object:
    """Convert Rust ``InjectionScanner.scan()`` JSON to dataclass."""
    from sunday.security.injection_scanner import (
        InjectionScanResult,
    )
    from sunday.security.types import ScanFinding, ThreatLevel

    data = json.loads(json_str)
    findings: List[ScanFinding] = []
    for f in data.get("findings", []):
        findings.append(
            ScanFinding(
                pattern_name=f.get("pattern_name", ""),
                matched_text=f.get("matched_text", ""),
                threat_level=ThreatLevel(
                    f.get("threat_level", "low").lower(),
                ),
                start=f.get("start", 0),
                end=f.get("end", 0),
                description=f.get("description", ""),
            )
        )

    threat_raw = data.get("threat_level", "low").lower()
    try:
        threat = ThreatLevel(threat_raw)
    except ValueError:
        threat = ThreatLevel.LOW

    return InjectionScanResult(
        is_clean=data.get("is_clean", True),
        findings=findings,
        threat_level=threat,
    )


def retrieval_results_from_json(json_str: str) -> list:
    """Convert Rust memory ``retrieve()`` JSON to a list of results."""
    from sunday.tools.storage._stubs import RetrievalResult

    items = json.loads(json_str)
    results: List[RetrievalResult] = []
    for item in items:
        meta = item.get("metadata", {})
        if isinstance(meta, str):
            try:
                meta = json.loads(meta)
            except (json.JSONDecodeError, TypeError):
                meta = {}
        results.append(
            RetrievalResult(
                content=item.get("content", ""),
                score=float(item.get("score", 0.0)),
                source=item.get("source", ""),
                metadata=meta,
            )
        )
    return results


# ---------------------------------------------------------------------------
# Phase 2 converters — optimization & engine types
# ---------------------------------------------------------------------------


def optimization_store_from_rust(path: str = ":memory:") -> object | None:
    """Get a Rust-backed OptimizationStore, or None if Rust unavailable."""
    mod = get_rust_module()
    if mod is None:
        return None
    try:
        return mod.OptimizationStore(path)
    except Exception:
        return None


def trial_result_from_json(json_str: str) -> dict:
    """Convert Rust TrialResult JSON to a Python dict."""
    return json.loads(json_str)


def optimization_run_from_json(json_str: str) -> dict:
    """Convert Rust OptimizationRun JSON to a Python dict."""
    return json.loads(json_str)


def generate_result_from_json(json_str: str) -> dict:
    """Convert Rust GenerateResult JSON to a Python dict."""
    data = json.loads(json_str)
    return {
        "content": data.get("content", ""),
        "model": data.get("model", ""),
        "finish_reason": data.get("finish_reason", "stop"),
        "usage": data.get("usage", {}),
        "tool_calls": data.get("tool_calls"),
        "ttft": data.get("ttft", 0.0),
        "cost_usd": data.get("cost_usd", 0.0),
        "metadata": data.get("metadata", {}),
    }


__all__ = [
    "RUST_AVAILABLE",
    "generate_result_from_json",
    "get_rust_module",
    "injection_result_from_json",
    "optimization_run_from_json",
    "optimization_store_from_rust",
    "retrieval_results_from_json",
    "scan_result_from_json",
    "trial_result_from_json",
    "Tokenizer",
    "AXTreeProcessor",
    "get_skill_evolution_engine",
    "parse_skill_markdown",
    "load_skill",
]

def get_tokenizer():
    """Return the Rust-backed Tokenizer class."""
    mod = get_rust_module()
    return getattr(mod, "Tokenizer", None)

def get_axtree_processor():
    """Return the Rust-backed AXTreeProcessor class."""
    mod = get_rust_module()
    return getattr(mod, "AXTreeProcessor", None)

def parse_skill_markdown(raw: str) -> object:
    mod = get_rust_module()
    if mod and hasattr(mod, "parse_skill_markdown"):
        return mod.parse_skill_markdown(raw)
    raise NotImplementedError("Rust parse_skill_markdown is not available")

def load_skill(toml_str: str) -> object:
    mod = get_rust_module()
    if mod and hasattr(mod, "load_skill"):
        return mod.load_skill(toml_str)
    raise NotImplementedError("Rust load_skill is not available")


def get_skill_evolution_engine(
    trace_db_path: Optional[str] = None, output_dir: Optional[str] = None
) -> Any:
    """Return a Rust-backed SkillEvolutionEngine instance.

    Parameters
    ----------
    trace_db_path:
        Path to the trace SQLite database. Defaults to
        ``~/.sunday/traces.db`` when *None*.
    output_dir:
        Directory where discovered SKILL.md files are written. Defaults
        to ``~/.sunday/skills/discovered/`` when *None*.
    """
    import pathlib

    mod = get_rust_module()
    cls = getattr(mod, "SkillEvolutionEngine", None)
    if cls is None:
        return None

    if trace_db_path is None:
        trace_db_path = str(pathlib.Path("~/.sunday/traces.db").expanduser())
    if output_dir is None:
        output_dir = str(pathlib.Path("~/.sunday/skills/discovered").expanduser())

    try:
        return cls(trace_db_path, output_dir)
    except Exception:
        return None


Tokenizer = get_tokenizer()
AXTreeProcessor = get_axtree_processor()
