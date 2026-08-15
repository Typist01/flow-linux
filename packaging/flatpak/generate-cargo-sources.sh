#!/usr/bin/env bash
# Generate cargo-sources.json for an offline Flathub build.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="$ROOT/packaging/flatpak/cargo-sources.json"
GENERATOR="${FLATPAK_CARGO_GENERATOR:-$HOME/.local/share/flatpak-builder-tools/cargo/flatpak-cargo-generator.py}"

if [[ ! -f "$GENERATOR" ]]; then
  echo "Need flatpak-cargo-generator.py from flatpak-builder-tools."
  echo "  git clone https://github.com/flatpak/flatpak-builder-tools.git"
  echo "  export FLATPAK_CARGO_GENERATOR=/path/to/cargo/flatpak-cargo-generator.py"
  exit 1
fi

python3 "$GENERATOR" "$ROOT/Cargo.lock" -o "$OUT"
echo "Wrote $OUT"
