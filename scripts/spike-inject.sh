#!/usr/bin/env bash
# =============================================================================
# spike-inject.sh — Spike 1: prove text injection works on your machine
# =============================================================================
#
# This is the FIRST spike in the Flow Linux build plan. Before writing any
# Rust code for the full dictation loop, we confirm that text can actually
# reach focused apps on your KDE Wayland desktop.
#
# Two injection methods are tested:
#   1. Direct typing — ydotool type (works on KDE; Arch wtype 0.4 often does not)
#   2. Clipboard paste — wl-copy + Ctrl+Shift+V (works in terminals)
#
# Pass criteria: paste MUST pass. Direct typing is a bonus on KDE with Arch wtype 0.4.
# =============================================================================

set -euo pipefail

# Text to inject — use first argument or a sensible default
TEXT="${1:-hello from flow-linux}"

# Resolve this script's directory so we can point to install-deps.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# -----------------------------------------------------------------------------
# Dependency checks
# -----------------------------------------------------------------------------
# Fail fast with a helpful message if packages from install-deps.sh are missing.
check_cmd() {
  if ! command -v "$1" &>/dev/null; then
    echo "MISSING: $1"
    echo "Run: $SCRIPT_DIR/install-deps.sh"
    exit 1
  fi
}

check_cmd wtype      # direct typing on Wayland
check_cmd wl-copy    # clipboard copy on Wayland
check_cmd ydotool    # simulated key presses (for paste)

# ydotool user service must be active — without it, ydotool commands do nothing.
# Arch unit: ydotool.service  (NOT ydotoold.service)
if ! systemctl --user is-active --quiet ydotool.service 2>/dev/null; then
  echo "WARN: ydotool.service is not active."
  echo "Fix: systemctl --user enable --now ydotool.service"
  echo "Check: systemctl --user is-active ydotool.service"
  echo ""
fi

# -----------------------------------------------------------------------------
# Helper: give user time to click the target window after confirming in terminal
# -----------------------------------------------------------------------------
# Pressing Enter in the terminal keeps focus here — so we wait a few seconds
# AFTER Enter so you can click Kate/Firefox/Konsole before injection runs.
countdown_to_inject() {
  local seconds="${1:-3}"
  echo "  Click the target window now — injecting in ${seconds}s..."
  for ((i = seconds; i >= 1; i--)); do
    echo "    ${i}..."
    sleep 1
  done
}

echo "=== Spike 1: Injection validation ==="
echo "Session: ${XDG_SESSION_TYPE:-unknown}  WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-unset}"
echo ""

# -----------------------------------------------------------------------------
# Test 1: direct typing into an editor
# -----------------------------------------------------------------------------
# Arch/CachyOS ships wtype 0.4 (github.com/atx/wtype) — it only supports the
# Wayland virtual-keyboard protocol, which KDE Plasma does NOT implement.
# That is NOT an install failure; the package is just too old for KDE.
#
# On KDE we use ydotool type instead (same uinput path as paste). Flow Linux
# will use clipboard-paste as the primary injection method on your setup.
echo "Test 1: direct typing (ydotool type — KDE-compatible)"
echo "  Note: Arch wtype 0.4 usually fails on KDE; we use ydotool type here."
echo "  1. Click a text editor (Kate, Firefox textarea, Cursor, etc.)"
echo "  2. Come back here and press Enter"
echo "  3. Quickly click the editor again during the countdown"
read -r -p "  Press Enter to start countdown... "
countdown_to_inject 3

if ydotool type -- "$TEXT"; then
  echo "PASS: ydotool type executed successfully"
  TYPE_OK=1
else
  echo "FAIL: ydotool type failed — is ydotool.service active?"
  echo "  Check: systemctl --user is-active ydotool.service"
  TYPE_OK=0
fi

echo ""

# -----------------------------------------------------------------------------
# Test 2: clipboard + simulated paste
# -----------------------------------------------------------------------------
# Terminals (Konsole, etc.) often ignore direct typing and need bracketed paste.
# Flow Linux always falls back to: copy text to clipboard, then simulate paste.
# Ctrl+Shift+V is the standard paste shortcut in most Linux terminals.
echo "Test 2: clipboard + Ctrl+Shift+V paste"
echo "  1. Click a terminal (Konsole, Alacritty, etc.)"
echo "  2. Come back here and press Enter"
echo "  3. Quickly click the terminal again during the countdown"
read -r -p "  Press Enter to start countdown... "
countdown_to_inject 3

# Copy our test text to the Wayland clipboard
wl-copy -- "$TEXT"

# Simulate Ctrl+Shift+V using Linux input keycodes:
#   KEY_LEFTCTRL  = 29   (press=29:1, release=29:0)
#   KEY_LEFTSHIFT = 42   (press=42:1, release=42:0)
#   KEY_V         = 47   (press=47:1, release=47:0)
# Format: keycode:state  where 1=press, 0=release
if ydotool key 29:1 42:1 47:1 47:0 42:0 29:0; then
  echo "PASS: paste keystroke sent"
  PASTE_OK=1
else
  echo "FAIL: ydotool paste failed — is ydotool.service active?"
  echo "  Check: systemctl --user is-active ydotool.service"
  PASTE_OK=0
fi

# -----------------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------------
echo ""
echo "=== Spike 1 summary ==="
echo "  type:   $([[ ${TYPE_OK:-0} -eq 1 ]] && echo PASS || echo FAIL)"
echo "  paste:  $([[ ${PASTE_OK:-0} -eq 1 ]] && echo PASS || echo FAIL)"

# Paste is the required gate on KDE + Arch wtype 0.4
if [[ ${PASTE_OK:-0} -eq 1 ]]; then
  echo ""
  if [[ ${TYPE_OK:-0} -eq 1 ]]; then
    echo "Spike 1 PASSED (type + paste) — proceed to Spike 2:"
  else
    echo "Spike 1 PASSED (paste works) — proceed to Spike 2:"
    echo "  Flow Linux will use clipboard paste as the primary injection method."
  fi
  echo "  cargo run --release -p flow-spike-hotkey"
  exit 0
fi

echo ""
echo "Spike 1 INCOMPLETE — fix failures above before continuing."
exit 1
