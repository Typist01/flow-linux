#!/usr/bin/env bash
# Stage Flow Linux into an AppDir and optionally build an AppImage.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
APPDIR="$ROOT/packaging/appimage/AppDir"
OUT_DIR="$ROOT/packaging/appimage/out"
APPIMAGE_NAME="Flow_Linux-x86_64.AppImage"

export PATH="${HOME}/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:${PATH:-}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
# Allows the downloaded appimagetool AppImage to run on systems without usable FUSE.
export APPIMAGE_EXTRACT_AND_RUN="${APPIMAGE_EXTRACT_AND_RUN:-1}"

echo "==> Building flow-daemon (release)"
cargo build --release -p flow-daemon --manifest-path "$ROOT/Cargo.toml"

echo "==> Staging AppDir"
mkdir -p "$APPDIR/usr/bin" \
  "$APPDIR/usr/share/applications" \
  "$APPDIR/usr/share/icons/hicolor/256x256/apps"

cp "$CARGO_TARGET_DIR/release/flow-daemon" "$APPDIR/usr/bin/flow-daemon"
chmod +x "$APPDIR/usr/bin/flow-daemon"
# Desktop entry is checked into AppDir; refresh icon from assets each build.
cp "$ROOT/assets/icons/flow-linux-256.png" \
  "$APPDIR/usr/share/icons/hicolor/256x256/apps/flow-linux.png"

# AppImage runtime expects a top-level desktop + .DirIcon sometimes
cp -f "$APPDIR/usr/share/applications/flow-linux.desktop" "$APPDIR/flow-linux.desktop"
cp -f "$APPDIR/usr/share/icons/hicolor/256x256/apps/flow-linux.png" "$APPDIR/.DirIcon"
cp -f "$APPDIR/usr/share/icons/hicolor/256x256/apps/flow-linux.png" "$APPDIR/flow-linux.png"
rm -f "$APPDIR/AppRun"
install -m 0755 "$ROOT/packaging/appimage/AppRun" "$APPDIR/AppRun"

echo "==> Secret scan"
"$ROOT/scripts/check-no-secrets.sh"

echo ""
echo "Host runtime dependencies (not bundled):"
echo "  - PipeWire (mic)"
echo "  - wl-clipboard"
echo "  - ydotool (+ user service)"
echo "  - libsecret / KDE Wallet"
echo "  - xdg-desktop-portal (GlobalShortcuts)"
echo ""
echo "BYOK: no API keys are included. Configure via Settings or OPENAI_API_KEY."
echo ""

if command -v appimagetool >/dev/null 2>&1; then
  mkdir -p "$OUT_DIR"
  if command -v linuxdeploy >/dev/null 2>&1; then
    echo "==> Running linuxdeploy"
    linuxdeploy --appdir="$APPDIR" --executable="$APPDIR/usr/bin/flow-daemon" \
      --desktop-file="$APPDIR/usr/share/applications/flow-linux.desktop" \
      --icon-file="$APPDIR/usr/share/icons/hicolor/256x256/apps/flow-linux.png"
  else
    echo "==> linuxdeploy not found; using staged AppDir directly"
  fi
  echo "==> Running appimagetool"
  appimagetool "$APPDIR" "$OUT_DIR/$APPIMAGE_NAME"
  (cd "$OUT_DIR" && sha256sum "$APPIMAGE_NAME" > sha256sums.txt)
  echo "Wrote $OUT_DIR/$APPIMAGE_NAME"
  echo "Wrote $OUT_DIR/sha256sums.txt"
else
  echo "AppDir staged at: $APPDIR"
  echo "Install appimagetool to produce a .AppImage, then re-run this script."
fi
