#!/usr/bin/env bash
# Assign Meta+Ctrl+Space to the Spike 2 hotkey in KDE's global shortcut config.
# Use when kglobalshortcutsrc shows: flow-dictation=none,none,...
set -euo pipefail

TRIGGER="${1:-Meta+Ctrl+Space}"

kwriteconfig6 --file kglobalshortcutsrc \
  --group "io.flowlinux.SpikeHotkey" \
  --key "flow-dictation" \
  "${TRIGGER},none,Flow Linux dictation"

# Reload KDE shortcut daemon
if command -v qdbus6 &>/dev/null; then
  qdbus6 org.kde.kglobalaccel /component/io.flowlinux.SpikeHotkey reload 2>/dev/null || true
elif command -v qdbus &>/dev/null; then
  qdbus org.kde.kglobalaccel /component/io.flowlinux.SpikeHotkey reload 2>/dev/null || true
fi

echo "Set flow-dictation → $TRIGGER"
echo "Restart flow-spike-hotkey if it is already running, then test the key."
