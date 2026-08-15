#!/usr/bin/env bash
# =============================================================================
# install-deps.sh — Install system packages Flow Linux needs on Arch/CachyOS
# =============================================================================
#
# Run this ONCE before Spike 1. It installs the tools that type/paste text
# into whatever app you have focused (the hardest part of a dictation app).
#
# Usage:
#   ./scripts/install-deps.sh
#
# Requires: sudo (pacman + udev rules + usermod)
# =============================================================================

# -e  exit immediately if any command fails
# -u  treat unset variables as errors
# -o pipefail  fail a pipeline if any step fails (not just the last one)
set -euo pipefail

echo "Installing Flow Linux system dependencies..."

# -----------------------------------------------------------------------------
# Core packages
# -----------------------------------------------------------------------------
# wtype        — types Unicode text into the focused Wayland window (primary)
# ydotool      — simulates key presses via Linux uinput (paste fallback)
# wl-clipboard — copies text to Wayland clipboard (wl-copy / wl-paste)
# pipewire     — audio stack (needed later for Spike 3 microphone capture)
# cmake        — required to compile whisper.cpp (Spike 4 / whisper-rs)
sudo pacman -S --needed wtype ydotool wl-clipboard pipewire pipewire-pulse cmake gtk3 xdotool libappindicator-gtk3 libsecret

# -----------------------------------------------------------------------------
# uinput device permissions (required by ydotool)
# -----------------------------------------------------------------------------
# ydotool talks to /dev/uinput to fake keyboard input at the kernel level.
# Without this udev rule, only root can use uinput.
if [[ ! -f /etc/udev/rules.d/80-uinput.rules ]]; then
  echo 'KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput"' \
    | sudo tee /etc/udev/rules.d/80-uinput.rules
  sudo udevadm control --reload-rules   # reload udev rule set
  sudo udevadm trigger                  # apply rules to existing devices
fi

# -----------------------------------------------------------------------------
# input group membership (optional fallback for evdev global hotkeys)
# -----------------------------------------------------------------------------
# Reading raw keyboard events from /dev/input/event* requires the input group.
# We use the XDG GlobalShortcuts portal on KDE (Spike 2), but evdev is the
# fallback for compositors without portal support — so we set this up now.
if ! id -nG "$USER" | grep -qw input; then
  sudo usermod -aG input "$USER"
  echo "Added $USER to input group — log out and back in for this to take effect."
fi

# -----------------------------------------------------------------------------
# ydotool user service (Arch/CachyOS)
# -----------------------------------------------------------------------------
# ydotool REQUIRES a background daemon that holds open the virtual keyboard
# device. Without it, `ydotool key` / `ydotool type` silently fail.
#
# On Arch the systemd UNIT is ydotool.service — NOT ydotoold.service.
# The daemon binary inside that unit is still /usr/bin/ydotoold.
systemctl --user enable --now ydotool.service

echo ""
echo "Verifying installation..."
systemctl --user is-active ydotool.service
command -v wtype wl-copy ydotool

echo ""
echo "Done. Next step:"
echo "  ./scripts/spike-inject.sh"
