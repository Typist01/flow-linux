# Flow Linux

Wispr Flow-like voice dictation for Linux — hold a global hotkey, speak, release, and text appears in the focused app.

**Stack:** Rust daemon, KDE Wayland-first, local Whisper or OpenAI cloud STT, optional OpenAI polish, clipboard/ydotool injection.

Flow Linux is the open-source, community/BYOK client. A hosted paid service can be built as a separate product or private fork without putting billing, auth, or cloud backend code in this repository.

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

Advanced OpenAI-compatible proxy hooks are available for development and future forks:

```toml
[stt]
openai_api_base = "https://api.openai.com/v1"
openai_realtime_url = "wss://api.openai.com/v1/realtime?intent=transcription"

[polish]
openai_api_base = "https://api.openai.com/v1"
```

## Local Whisper model

Required only when `stt.provider = "local"`:

```bash
mkdir -p ~/.cache/flow-linux/models
curl -L -o ~/.cache/flow-linux/models/ggml-base.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
```

## System requirements

Flow Linux targets **KDE Wayland first**. Other Linux desktops and compositors are experimental because global shortcuts and text injection differ across desktop stacks.

Host services are not bundled in AppImages:

| Role | Arch / CachyOS | Fedora | Debian / Ubuntu |
|------|----------------|--------|-----------------|
| Microphone | `pipewire pipewire-pulse` | `pipewire pipewire-pulseaudio` | `pipewire pipewire-pulse` |
| Clipboard | `wl-clipboard` | `wl-clipboard` | `wl-clipboard` |
| Injection | `ydotool` | `ydotool` | `ydotool` |
| Keyring | `libsecret` | `libsecret` | `libsecret-1-0` |
| UI / tray build deps | `gtk3 libappindicator-gtk3` | `gtk3 libappindicator-gtk3` | `libgtk-3-0 libappindicator3-1` |
| KDE portal | `xdg-desktop-portal xdg-desktop-portal-kde` | `xdg-desktop-portal xdg-desktop-portal-kde` | `xdg-desktop-portal xdg-desktop-portal-kde` |

On Arch/CachyOS, the helper script installs the development and runtime packages:

```bash
./scripts/install-deps.sh
```

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

## Install from AppImage

Download `Flow_Linux-x86_64.AppImage` from a release, then:

```bash
chmod +x Flow_Linux-x86_64.AppImage
./Flow_Linux-x86_64.AppImage --install
systemctl --user enable --now flow-daemon.service
```

Then open the tray Settings panel and configure your OpenAI API key or local Whisper model.

The AppImage `--install` step installs:

- KDE portal desktop file for the stable app id
- user systemd service
- autostart desktop entry

It does **not** install host packages such as PipeWire, `ydotool`, `wl-clipboard`, `libsecret`, or desktop portals.

## Build a Release AppImage

```bash
./scripts/check-no-secrets.sh
./packaging/appimage/build-appimage.sh
```

Stages `packaging/appimage/AppDir`. If `linuxdeploy` and `appimagetool` are on `PATH`, also writes:

- `packaging/appimage/out/Flow_Linux-x86_64.AppImage`
- `packaging/appimage/out/sha256sums.txt`

Font/icon licenses: see [`assets/ATTRIBUTIONS.md`](assets/ATTRIBUTIONS.md).

## License

Flow Linux source is licensed under either of:

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT license ([`LICENSE-MIT`](LICENSE-MIT))

Fonts are under SIL OFL (see [`assets/ATTRIBUTIONS.md`](assets/ATTRIBUTIONS.md)).
