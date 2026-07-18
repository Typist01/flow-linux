# Flow Linux

Wispr Flow-like voice dictation for Linux — hold a global hotkey, speak, release, and text appears in the focused app.

**Stack:** Rust daemon, KDE Wayland-first, local Whisper or OpenAI cloud STT, optional OpenAI polish, clipboard/ydotool injection.

## Features

- Push-to-talk global hotkey (KDE GlobalShortcuts portal)
- **STT:** batch (local Whisper / OpenAI file) or **streaming** (`gpt-realtime-whisper`)
- **Polish (optional):** OpenAI chat models (`gpt-4.1-nano`, `gpt-4o-mini`, …) on final transcript only
- Signal tray icon + Flow Capsule listening overlay (live partials in streaming mode)
- Settings instrument panel (Ready status, mode, models, BYOK key storage)
- **BYOK:** API keys are never shipped — Settings/keyring or `OPENAI_API_KEY` only
- Single-instance daemon lock, systemd user service, autostart

## Quick start

```bash
# System dependencies (CachyOS/Arch)
./scripts/install-deps.sh

# KDE portal registration (required for global hotkey)
./scripts/install-spike-hotkey-desktop.sh

# Build
export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
export CARGO_TARGET_DIR="$PWD/target"
cargo build --release -p flow-daemon

# Install desktop file, systemd unit, autostart
./scripts/install-daemon.sh

# Run manually
./target/release/flow-daemon

# Or via systemd
systemctl --user enable --now flow-daemon.service
```

**Usage:** focus a text field → hold **Ctrl+Win+Space** → speak → release.

**Settings:** tray → **Settings…** → configure STT/polish providers, models, and OpenAI API key.

## Configuration

`~/.config/flow-linux/config.toml` — created on first run.

```toml
[stt]
provider = "openai"   # local | openai
openai_model = "gpt-4o-mini-transcribe"

[polish]
enabled = true
provider = "openai"
openai_model = "gpt-4.1-nano"
```

**BYOK — no API keys are included in builds or AppImages.**  
Configure via tray → **Settings…** → Voice (system keyring) or export `OPENAI_API_KEY`.

## Local Whisper model

Required only when `stt.provider = "local"`:

```bash
mkdir -p ~/.cache/flow-linux/models
curl -L -o ~/.cache/flow-linux/models/ggml-base.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
```

## System requirements (CachyOS/Arch)

```bash
./scripts/install-deps.sh
```

Packages: `wtype`, `ydotool`, `wl-clipboard`, `pipewire`, `cmake`, `gtk3`, `libappindicator-gtk3`, `libsecret`

### ydotool service

On Arch the user service is **`ydotool.service`** (not `ydotoold.service`):

```bash
systemctl --user enable --now ydotool.service
```

## Project layout

```
crates/
  flow-daemon    # orchestration + state machine
  flow-config    # TOML config + provider catalogs
  flow-hotkey    # KDE GlobalShortcuts portal
  flow-audio     # cpal capture
  flow-stt       # Whisper + OpenAI STT backends
  flow-polish    # optional LLM cleanup
  flow-inject    # clipboard paste / ydotool
  flow-ui        # system tray
  flow-settings  # egui settings window
  flow-secrets   # keyring + API key validation
spikes/          # Phase 1 de-risking binaries
scripts/         # install-deps, install-daemon, spike-inject
```

## Development spikes

Phase 1 validation binaries (run in order during initial bring-up):

| Spike | Command |
|-------|---------|
| 1 — Injection | `./scripts/spike-inject.sh` |
| 2 — Hotkey | `cargo run --release -p flow-spike-hotkey` |
| 3 — Audio | `cargo run -p flow-spike-audio` |
| 4 — STT | `cargo run -p flow-spike-stt` |
| 5+6 — Daemon | `cargo run --release -p flow-daemon` |

## Build note (Cursor / rustup)

If `cargo` fails with a proxy error:

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
export CARGO_TARGET_DIR="$PWD/target"
```

## Distribute (AppImage scaffold)

```bash
./scripts/check-no-secrets.sh
./packaging/appimage/build-appimage.sh
```

Stages `packaging/appimage/AppDir`. If `linuxdeploy` and `appimagetool` are on `PATH`, also writes  
`packaging/appimage/out/Flow_Linux-x86_64.AppImage`.

**Host packages still required:** PipeWire, `wl-clipboard`, `ydotool` (+ user service), `libsecret`, desktop portal (GlobalShortcuts).

Font/icon licenses: see [`assets/ATTRIBUTIONS.md`](assets/ATTRIBUTIONS.md).

## License

MIT OR Apache-2.0 (see individual crate manifests). Fonts under SIL OFL (see attributions).
