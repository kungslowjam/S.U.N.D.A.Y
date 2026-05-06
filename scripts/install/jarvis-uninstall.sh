#!/usr/bin/env bash
# sunday-uninstall.sh — clean removal of SUNDAY from $HOME.
#
# Removes:
#   ~/.sunday/
#   ~/.local/bin/sunday
#   ~/.local/bin/sunday-uninstall
#
# Does NOT remove: ollama, uv, or the Rust toolchain.

set -euo pipefail

OPENSUNDAY_HOME="${OPENSUNDAY_HOME:-$HOME/.sunday}"

if [[ -f "$OPENSUNDAY_HOME/.state/bg.pid" ]]; then
    pid=$(cat "$OPENSUNDAY_HOME/.state/bg.pid" 2>/dev/null || echo "")
    if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
        echo "Stopping background work (pid=$pid)..."
        kill "$pid" 2>/dev/null || true
    fi
fi

if command -v ollama >/dev/null 2>&1; then
    ollama stop >/dev/null 2>&1 || true
fi

if [[ -d "$OPENSUNDAY_HOME" ]]; then
    rm -rf "$OPENSUNDAY_HOME"
    echo "Removed $OPENSUNDAY_HOME"
fi

for f in "$HOME/.local/bin/sunday" "$HOME/.local/bin/sunday-uninstall"; do
    if [[ -L "$f" ]] || [[ -f "$f" ]]; then
        rm -f "$f"
        echo "Removed $f"
    fi
done

cat <<EOF

SUNDAY removed.

Left intact (may be used by other tools):
  - Ollama       (uninstall: brew uninstall ollama  /  rm -f /usr/local/bin/ollama)
  - uv           (uninstall: rm -rf ~/.local/share/uv ~/.cargo/bin/uv)
  - Rust toolchain (uninstall: rustup self uninstall)
EOF
