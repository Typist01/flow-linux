# Install Flow Linux

Supported target: **KDE Wayland**. The same host packages are listed for other distros so you can try them; GNOME and other compositors are experimental.

## Host packages

The AppImage and Flatpak do **not** bundle these.

| Role | Arch / CachyOS | Fedora | Debian / Ubuntu |
|------|----------------|--------|-----------------|
| Microphone | `pipewire` `pipewire-pulse` | `pipewire` `pipewire-pulseaudio` | `pipewire` `pipewire-pulse` |
| Clipboard | `wl-clipboard` | `wl-clipboard` | `wl-clipboard` |
| Injection | `ydotool` | `ydotool` | `ydotool` |
| Keyring | `libsecret` | `libsecret` | `libsecret-1-0` |
| Portal | `xdg-desktop-portal` `xdg-desktop-portal-kde` | `xdg-desktop-portal` `xdg-desktop-portal-kde` | `xdg-desktop-portal` `xdg-desktop-portal-kde` |

Enable injection:

```bash
systemctl --user enable --now ydotool.service
```

On some distros the unit is `ydotoold.service`. Settings → Ready shows which one is missing.

## AppImage

```bash
chmod +x Flow_Linux-x86_64.AppImage
./Flow_Linux-x86_64.AppImage --install
systemctl --user enable --now flow-daemon.service
```

`--install` writes the portal desktop file, user systemd unit, and autostart entry. It does not install host packages.

## First-run checks

Open tray → Settings → Ready. Typical red rows:

- **Microphone mute** — hardware mic-mute key or System Settings → Audio
- **Hotkey** — assign Ctrl+Win+Space in the portal dialog (needed once after the `io.github.Typist01.FlowLinux` id change)
- **ydotool** — install the package and enable the user service
- **Local Whisper model** — use **Download Whisper model** (batch + local only)
- **OpenAI API key** — required only for streaming / cloud STT

## From source

See the README “Build from source” section. After `cargo build --release -p flow-daemon`, run `./scripts/install-daemon.sh`.
