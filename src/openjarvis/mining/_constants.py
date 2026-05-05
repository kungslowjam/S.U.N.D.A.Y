# src/openjarvis/mining/_constants.py
"""Constants for the Pearl mining subsystem.

Pinned Pearl ref OJ has tested against. Bumped per OJ release after
re-testing the integration end-to-end on a real H100/H200 host. See
spec ``docs/design/2026-05-05-vllm-pearl-mining-integration-design.md``
section 7.3 for the rev-bump workflow.
"""

from __future__ import annotations

from pathlib import Path

PEARL_REPO = "https://github.com/pearl-research-labs/pearl.git"
# TODO at implementation time: replace with the specific commit/tag verified
# against H100. Document the chosen ref in the OJ release notes.
PEARL_PINNED_REF = "main"
PEARL_IMAGE_TAG = f"openjarvis/pearl-miner:{PEARL_PINNED_REF}"

# Default Pearl-blessed model. Overridable via [mining.extra].model.
DEFAULT_PEARL_MODEL = "pearl-ai/Llama-3.3-70B-Instruct-pearl"

# Default ports as Pearl's container exposes them (network_mode="host").
DEFAULT_VLLM_PORT = 8000
DEFAULT_GATEWAY_RPC_PORT = 8337
# Pearl-gateway's Prometheus metrics endpoint. Verified default from
# pearl/miner/pearl-gateway/src/pearl_gateway/cli.py:72 (status message
# references http://127.0.0.1:9109/metrics) and pearl/miner/conftest.py
# line 301 (METRICS_BIND="127.0.0.1:9109"). Spec A's earlier 8339 guess
# was incorrect — kept the constant name for compatibility but bumped
# the value.
DEFAULT_GATEWAY_METRICS_PORT = 9109

# Default pearld RPC endpoint (mainnet).
DEFAULT_PEARLD_RPC_URL = "http://localhost:44107"

# Pre-flight free-disk requirement for the 70B model + headroom.
MIN_FREE_DISK_GB = 200

# Runtime sidecar location (single-session assumption — see spec §8.8).
RUNTIME_DIR = Path.home() / ".openjarvis" / "runtime"
SIDECAR_PATH = RUNTIME_DIR / "mining.json"
SIDECAR_LOCK_PATH = RUNTIME_DIR / "mining.lock"

# Pearl source cache for build-from-pin path (see spec §7.2).
PEARL_CACHE_DIR = Path.home() / ".openjarvis" / "cache" / "pearl"

# ── cpu-pearl provider (Apple Silicon / non-CUDA hosts) ────────────────────────
# See spec ``docs/design/2026-05-05-apple-silicon-pearl-mining-design.md`` §13.

# Default mining-loop matrix shapes. These are the same values used by Pearl's
# upstream test_python_api.py — known to produce a valid proof per call at test
# difficulty. Real difficulty is set per-block by the network and we can't
# control that; the only knob we expose is the matmul shape, which determines
# search space size per `mine()` call.
CPU_PEARL_DEFAULT_M = 256
CPU_PEARL_DEFAULT_N = 128
CPU_PEARL_DEFAULT_K = 1024
CPU_PEARL_DEFAULT_RANK = 32

# Default PeriodicPattern row/col index sets that select which tiles of the
# noised matmul produce mining shares. These specific values match Pearl's
# upstream test fixture (py-pearl-mining/tests/test_python_api.py); changing
# them changes which subset of the matrix gets probed each call. Don't change
# without understanding the impact on hashrate vs. proof shape.
CPU_PEARL_DEFAULT_ROWS_PATTERN = (0, 8, 64, 72)
CPU_PEARL_DEFAULT_COLS_PATTERN = (0, 1, 8, 9, 32, 33, 40, 41)

# Names of the Pearl Python packages cpu-pearl depends on, in install order.
# These are installed from local paths under PEARL_CACHE_DIR (or, in the
# future, from PyPI when Pearl publishes wheels).
PEARL_CPU_PACKAGES = (
    "py-pearl-mining",
    "miner-utils",
    "pearl-gateway",
    "miner-base",
)
