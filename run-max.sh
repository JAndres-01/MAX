#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUNDLE_DIR="$SCRIPT_DIR/ui/build/linux/x64/release/bundle"
CORE_SO="$SCRIPT_DIR/core/target/release/libmax_core.so"

# Free port 53317 (allowed by UFW firewall) if synapse-drop daemon is holding it
systemctl --user stop synapse-drop.service 2>/dev/null || true
pkill -x synapse-drop 2>/dev/null || true

# Safe update of libmax_core.so: only when no max_drop process is running
if [[ -f "$CORE_SO" && -d "$BUNDLE_DIR/lib" ]]; then
  if ! pgrep -x max_drop >/dev/null 2>&1; then
    if ! cmp -s "$CORE_SO" "$BUNDLE_DIR/lib/libmax_core.so" 2>/dev/null; then
      install -m 755 "$CORE_SO" "$BUNDLE_DIR/lib/libmax_core.so"
    fi
  fi
fi

exec "$BUNDLE_DIR/max_drop" "$@"
