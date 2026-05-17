"""Entry-point wrapper that prefers the native Rust CLI when available."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


def _find_rust_cli() -> Path | None:
    """Look for sunday-cli binary in known locations."""
    # 1. Same directory as this script (installed alongside package)
    local = Path(__file__).parent / "sunday-cli"
    if local.exists():
        return local

    # 2. Project rust/target/release (development)
    project_root = Path(__file__).parent.parent.parent
    dev = project_root / "rust" / "target" / "release" / "sunday-cli"
    if dev.exists():
        return dev

    # 3. Windows exe variant
    dev_exe = project_root / "rust" / "target" / "release" / "sunday-cli.exe"
    if dev_exe.exists():
        return dev_exe

    # 4. PATH lookup
    for path_name in ("sunday-cli", "sunday-cli.exe"):
        if sys.platform == "win32" and not path_name.endswith(".exe"):
            path_name += ".exe"
        found = subprocess.run(["where" if sys.platform == "win32" else "which", path_name], capture_output=True, text=True)
        if found.returncode == 0:
            p = Path(found.stdout.strip().splitlines()[0])
            if p.exists():
                return p

    return None


def main() -> None:
    rust_cli = _find_rust_cli()
    if rust_cli is not None:
        # Forward all arguments to the Rust CLI
        os.execv(str(rust_cli), [str(rust_cli), *sys.argv[1:]])

    # Fallback to pure-Python CLI if Rust binary is missing
    print("⚠️  Rust CLI not found. Falling back to Python CLI...", file=sys.stderr)
    from sunday.cli import main as python_main

    python_main()


if __name__ == "__main__":
    main()
