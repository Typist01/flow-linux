#!/usr/bin/env bash
# Assign Meta+Ctrl+Space to Flow Linux in KDE's global shortcut config.
# Use when kglobalshortcutsrc shows: flow-dictation=none,none,...
set -euo pipefail

APP_ID="io.github.Typist01.FlowLinux"
TRIGGER="${1:-Meta+Ctrl+Space}"

kwriteconfig6 --file kglobalshortcutsrc \
  --group "$APP_ID" \
  --key "flow-dictation" \
  "${TRIGGER},none,Flow Linux dictation"

if command -v qdbus6 &>/dev/null; then
  qdbus6 org.kde.kglobalaccel "/component/${APP_ID}" reload 2>/dev/null || true
elif command -v qdbus &>/dev/null; then
  qdbus org.kde.kglobalaccel "/component/${APP_ID}" reload 2>/dev/null || true
fi

echo "Set flow-dictation → $TRIGGER"
echo "Restart flow-daemon if it is already running, then test the key."
