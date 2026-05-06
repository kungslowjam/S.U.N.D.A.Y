#!/usr/bin/env bash
# sunday-wrapper.sh — symlinked to ~/.local/bin/sunday.
# Activates the managed venv and execs the real sunday CLI.

OPENSUNDAY_HOME="${OPENSUNDAY_HOME:-$HOME/.sunday}"
VENV="$OPENSUNDAY_HOME/.venv"

if [[ ! -d "$VENV" ]]; then
    echo "sunday: venv not found at $VENV" >&2
    echo "Re-run the installer: curl -fsSL https://sunday.ai/install.sh | bash" >&2
    exit 1
fi

exec "$VENV/bin/sunday" "$@"
