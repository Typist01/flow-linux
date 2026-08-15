# Flatpak / Flathub

App id: `io.github.Typist01.FlowLinux`

## Why `--device=all`

Text injection uses host `ydotool`, which talks to `/dev/uinput`. Flatpak has no narrower permission for uinput (`--device=input` does not include it). Global hotkeys use the desktop portal (`--talk-name=org.freedesktop.portal.Desktop`), not evdev.

The host still needs `ydotool.service` (or `ydotoold.service`) and a udev rule that lets the session user write uinput. The Flatpak cannot install udev rules. Settings → Ready reports a missing service.

If Flathub rejects `--device=all`, the fallback is clipboard-only plus a notification to paste. The AppImage keeps full inject.

## Local build (after cargo-sources exist)

```bash
./packaging/flatpak/generate-cargo-sources.sh
flatpak-builder --user --install --force-clean /tmp/flow-linux-fp packaging/flatpak/io.github.Typist01.FlowLinux.yml
```

## Flathub submit

1. Tag `v0.2.0` and pin the git `commit` in the manifest.
2. Generate `cargo-sources.json`.
3. Open a PR on [flathub/flathub](https://github.com/flathub/flathub) (`new-pr`) with this app id, the demo video, and the `--device=all` write-up above.
