#!/usr/bin/env bash
# Install Flow Linux daemon: desktop file, systemd user unit, optional autostart.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BINARY="${FLOW_DESKTOP_BINARY:-$PROJECT_ROOT/target/release/flow-daemon}"
SYSTEMD_UNIT_SRC="$PROJECT_ROOT/packaging/systemd/flow-daemon.service"
SYSTEMD_UNIT_DEST="$HOME/.config/systemd/user/flow-daemon.service"
AUTOSTART_SRC="$PROJECT_ROOT/assets/autostart/flow-linux.desktop"
AUTOSTART_DEST="$HOME/.config/autostart/flow-linux.desktop"
DESKTOP_SRC="$PROJECT_ROOT/assets/io.github.Typist01.FlowLinux.desktop"
DESKTOP_DEST="$HOME/.local/share/applications/io.github.Typist01.FlowLinux.desktop"
LEGACY_DESKTOP="$HOME/.local/share/applications/io.flowlinux.SpikeHotkey.desktop"

if [[ ! -x "$BINARY" ]]; then
  echo "Binary not found: $BINARY"
  echo "Build first:"
  echo "  export PATH=\"\$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:\$PATH\""
  echo "  export CARGO_TARGET_DIR=\"$PROJECT_ROOT/target\""
  echo "  cargo build --release -p flow-daemon"
  exit 1
fi

mkdir -p "$HOME/.local/share/applications"
mkdir -p "$HOME/.config/systemd/user"
mkdir -p "$HOME/.config/autostart"

# KDE portal registration (required for global hotkey)
sed -E "s|^Exec=.*|Exec=$BINARY|" "$DESKTOP_SRC" > "$DESKTOP_DEST"
rm -f "$LEGACY_DESKTOP"

# systemd user service with absolute binary path
sed -E "s|^ExecStart=.*|ExecStart=$BINARY|" "$SYSTEMD_UNIT_SRC" > "$SYSTEMD_UNIT_DEST"

# autostart entry
sed -E "s|^Exec=.*|Exec=$BINARY|" "$AUTOSTART_SRC" > "$AUTOSTART_DEST"

if command -v update-desktop-database &>/dev/null; then
  update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
fi

systemctl --user daemon-reload

echo "Installed:"
echo "  Portal desktop: $DESKTOP_DEST"
echo "  systemd unit:   $SYSTEMD_UNIT_DEST"
echo "  Autostart:      $AUTOSTART_DEST"
echo ""
echo "Enable and start:"
echo "  systemctl --user enable --now flow-daemon.service"
echo ""
echo "Or run manually:"
echo "  $BINARY"
