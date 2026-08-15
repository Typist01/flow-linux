# Launch posts (v0.2.0)

Record a 30–45s KDE Wayland clip first: focus a text field, hold Ctrl+Win+Space, speak, release, text appears. Attach that video everywhere. Do not call this a Wispr product or “Wispr for Linux.”

## Title

OSS push-to-talk dictation for KDE Wayland (local Whisper or BYOK)

## Short post (KDE Discuss, r/kde, r/cachyos)

Flow Linux is an open-source hold-to-talk dictation daemon for KDE Wayland.

Hold Ctrl+Win+Space, speak, release — text is pasted into the focused app. You can run fully offline with a local Whisper model (download from Settings) or bring your own OpenAI key for streaming.

It is KDE-first. Other desktops are experimental. Host deps (PipeWire, ydotool, wl-clipboard, portal) are not bundled.

AppImage: https://github.com/Typist01/flow-linux/releases
Source: https://github.com/Typist01/flow-linux

I want bug reports from KDE/Arch/CachyOS installs more than feature requests right now.

## Show HN

**Show HN: Flow Linux – hold-to-talk dictation for KDE Wayland (local Whisper or BYOK)**

I built a small Rust daemon that does push-to-talk on KDE Wayland: global hotkey via the desktop portal, PipeWire capture, local Whisper or OpenAI streaming, then clipboard + ydotool paste.

No accounts and no bundled API keys. Settings has a Ready page that tells you if the mic is muted, the hotkey is unbound, ydotool is down, or the Whisper model is missing.

KDE only for now. Looking for people who already live on Plasma to break it.

https://github.com/Typist01/flow-linux

## Screencast checklist

1. Unmute the laptop mic.
2. Confirm Settings → Ready is all green (or download `base.en` first).
3. Open Kate or a browser text field.
4. Hold Ctrl+Win+Space, say one sentence, release.
5. Keep the clip under 45 seconds. No wallpaper with personal data.
