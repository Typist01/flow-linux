#!/usr/bin/env bash
# Install the .desktop file required by KDE's portal for app ID registration.
# Without this, the daemon fails with:
#   "App info not found for 'io.github.Typist01.FlowLinux'"
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
APP_ID="io.github.Typist01.FlowLinux"
DESKTOP_SRC="$PROJECT_ROOT/assets/${APP_ID}.desktop"
DESKTOP_DEST="$HOME/.local/share/applications/${APP_ID}.desktop"
LEGACY_DEST="$HOME/.local/share/applications/io.flowlinux.SpikeHotkey.desktop"
BINARY="${FLOW_DESKTOP_BINARY:-$PROJECT_ROOT/target/release/flow-daemon}"

mkdir -p "$HOME/.local/share/applications"

# Point Exec= at the actual release binary path (absolute path required by KDE portal)
sed -E "s|^Exec=.*|Exec=$BINARY|" "$DESKTOP_SRC" > "$DESKTOP_DEST"
rm -f "$LEGACY_DEST"

if command -v update-desktop-database &>/dev/null; then
  update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
fi

echo "Installed: $DESKTOP_DEST"
echo "App ID: $APP_ID"
echo "Rebind Meta+Ctrl+Space once if the portal shortcut was previously registered under the old id."
