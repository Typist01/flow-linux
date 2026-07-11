#!/usr/bin/env bash
# Install the .desktop file required by KDE's portal for app ID registration.
# Without this, flow-spike-hotkey fails with:
#   "App info not found for 'io.flowlinux.SpikeHotkey'"
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DESKTOP_SRC="$PROJECT_ROOT/assets/io.flowlinux.SpikeHotkey.desktop"
DESKTOP_DEST="$HOME/.local/share/applications/io.flowlinux.SpikeHotkey.desktop"
BINARY="${FLOW_DESKTOP_BINARY:-$PROJECT_ROOT/target/release/flow-daemon}"

mkdir -p "$HOME/.local/share/applications"

# Point Exec= at the actual release binary path (absolute path required by KDE portal)
sed -E "s|^Exec=.*|Exec=$BINARY|" "$DESKTOP_SRC" > "$DESKTOP_DEST"

if command -v update-desktop-database &>/dev/null; then
  update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
fi

echo "Installed: $DESKTOP_DEST"
echo "App ID: io.flowlinux.SpikeHotkey"
