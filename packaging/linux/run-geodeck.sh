#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_BIN="$SCRIPT_DIR/geodedeck"

if [ ! -x "$APP_BIN" ]; then
    chmod +x "$APP_BIN" || true
fi

if [ -n "${APPIMAGE:-}" ] || [ "${XDG_SESSION_TYPE:-}" = "wayland" ]; then
    export WINIT_UNIX_BACKEND="${WINIT_UNIX_BACKEND:-x11}"
fi

exec "$APP_BIN" "$@"
