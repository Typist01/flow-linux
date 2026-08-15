# Flow Linux

Hold a global hotkey, speak, release — text appears in the focused app.

**KDE Wayland first.** Other Linux desktops are experimental. Local Whisper (offline) or your own OpenAI key (BYOK). No API keys are shipped.

## Install (AppImage)

You need a KDE Wayland session plus host packages that the AppImage does not bundle: PipeWire, `wl-clipboard`, `ydotool` (user service), `libsecret`, and `xdg-desktop-portal-kde`. Package names: [docs/install.md](docs/install.md).

```bash
chmod +x Flow_Linux-x86_64.AppImage
./Flow_Linux-x86_64.AppImage --install
systemctl --user enable --now flow-daemon.service
systemctl --user enable --now ydotool.service
```

Then tray → **Settings…**

1. Fix anything red on the Ready page (muted mic, unbound hotkey, missing model).
2. For offline dictation: Dictation → Batch → Local, then **Download Whisper model**.
3. For live streaming: paste an OpenAI API key under Voice (stored in the system keyring).

**Usage:** focus a text field → hold **Ctrl+Win+Space** → speak → release.

The first start after this release uses a new portal app id (`io.github.Typist01.FlowLinux`). Re-assign the shortcut once if the key does nothing.

## Build from source (CachyOS / Arch)

```bash
./scripts/install-deps.sh
./scripts/install-hotkey-desktop.sh

export PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"
export CARGO_TARGET_DIR="$PWD/target"
cargo build --release -p flow-daemon

./scripts/install-daemon.sh
systemctl --user enable --now flow-daemon.service
```

Or run `./target/release/flow-daemon` directly.

## Configuration

`~/.config/flow-linux/config.toml` is created on first run.

```toml
[stt]
provider = "local"    # local | openai
mode = "batch"        # batch | streaming (streaming requires OpenAI)
model = "base.en"

[polish]
enabled = false
```

**BYOK:** tray → Settings → Voice, or `OPENAI_API_KEY`. Nothing is bundled in builds or AppImages.

Optional OpenAI-compatible proxy hooks (for a future hosted backend):

```toml
[stt]
openai_api_base = "https://api.openai.com/v1"
openai_realtime_url = "wss://api.openai.com/v1/realtime?intent=transcription"
```

## Local Whisper

Required only for `stt.provider = "local"`. Prefer **Download Whisper model** in Settings. Manual:

```bash
mkdir -p ~/.cache/flow-linux/models
curl -L -o ~/.cache/flow-linux/models/ggml-base.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
```

## System requirements

| Role | Arch / CachyOS | Fedora | Debian / Ubuntu |
|------|----------------|--------|-----------------|
| Microphone | `pipewire pipewire-pulse` | `pipewire pipewire-pulseaudio` | `pipewire pipewire-pulse` |
| Clipboard | `wl-clipboard` | `wl-clipboard` | `wl-clipboard` |
| Injection | `ydotool` | `ydotool` | `ydotool` |
| Keyring | `libsecret` | `libsecret` | `libsecret-1-0` |
| KDE portal | `xdg-desktop-portal-kde` | `xdg-desktop-portal-kde` | `xdg-desktop-portal-kde` |

On Arch the user service is **`ydotool.service`**:

```bash
systemctl --user enable --now ydotool.service
```

If dictation records silence, check the hardware mic-mute key and Settings → Ready.

## Project layout

```
crates/
  flow-daemon    # orchestration + state machine
  flow-config    # TOML config + Ready-bar health checks
  flow-hotkey    # KDE GlobalShortcuts portal
  flow-audio     # cpal capture
  flow-stt       # Whisper + OpenAI STT backends
  flow-polish    # optional LLM cleanup
  flow-inject    # clipboard paste / ydotool
  flow-ui        # system tray
  flow-settings  # egui settings window
  flow-secrets   # keyring + API key validation
scripts/         # install-deps, install-daemon, install-hotkey-desktop
```

## Build a release AppImage

```bash
./scripts/check-no-secrets.sh
./packaging/appimage/build-appimage.sh
```

Writes `packaging/appimage/out/Flow_Linux-x86_64.AppImage` when `appimagetool` is on `PATH`.

Font/icon licenses: [`assets/ATTRIBUTIONS.md`](assets/ATTRIBUTIONS.md).

## License

Flow Linux source is licensed under either of:

- Apache License 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT license ([`LICENSE-MIT`](LICENSE-MIT))
