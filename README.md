# linux-wispr

> **Push-to-talk speech-to-text for Linux** — a Linux port of Whisper Flow.

Hold a configurable hotkey, speak, release the key, and the transcribed text is
typed directly into whatever window has focus.  Powered by
[faster-whisper](https://github.com/SYSTRAN/faster-whisper) (a CTranslate2
port of OpenAI Whisper).

---

## Features

- 🎙 **Push-to-talk** — hold a key to record, release to transcribe & type
- ⚡ **Fast** — runs entirely on your machine; no cloud API required
- 🖥 **X11 & Wayland** — types via `xdotool` (X11) or `wtype` (Wayland)
- 🌍 **Multilingual** — auto-detect or pin a language; optional English translation
- 🔧 **Configurable** — TOML config file with sensible defaults
- 🖱 **Any hotkey** — single key or modifier combos (`ctrl+space`, `right_shift`, etc.)
- 🔕 **Silence-aware** — built-in VAD skips silent segments

---

## Requirements

| Category | Requirement |
|---|---|
| OS | Linux (X11 or Wayland) |
| Python | 3.10+ |
| Audio library | PortAudio (`libportaudio2`) |
| Typing tool | `xdotool` (X11) **or** `wtype` / `ydotool` (Wayland) |
| GPU (optional) | NVIDIA GPU + CUDA for faster inference |

---

## Installation

### 1. Install system dependencies

```bash
# Debian / Ubuntu
sudo apt update
sudo apt install portaudio19-dev xdotool   # X11
# or
sudo apt install portaudio19-dev wtype      # Wayland
```

### 2. Install linux-wispr

```bash
pip install linux-wispr
```

Or install from source:

```bash
git clone https://github.com/Typist01/linux-wispr.git
cd linux-wispr
pip install .
```

### 3. (Optional) GPU acceleration

```bash
pip install torch  # pulls in CUDA support automatically
```

---

## Quick start

```bash
# Run with defaults (base model, Right Shift hotkey)
wispr

# Choose a larger / more accurate model
wispr --model small

# Pin the language and hotkey
wispr --model base --key "ctrl+space" --language en
```

Hold your hotkey, speak, release — the text appears in your focused window.

---

## Configuration

Generate a default config file:

```bash
wispr init-config
# → ~/.config/linux-wispr/config.toml
```

Edit `~/.config/linux-wispr/config.toml`:

```toml
[hotkey]
# Key to hold for push-to-talk
# Examples: "right_shift", "ctrl+space", "alt+r", "f9"
key = "right_shift"

[audio]
sample_rate = 16000   # Hz — Whisper expects 16 kHz
device = -1           # -1 = system default; use `wispr list-devices` to find others
max_duration = 30     # seconds; 0 = unlimited

[whisper]
model = "base"        # tiny | base | small | medium | large | large-v2 | large-v3
device = "auto"       # auto | cpu | cuda
compute_type = "auto" # auto | int8 | float16 | float32
language = ""         # "" = auto-detect; or e.g. "en", "de", "fr"
translate = false     # true → translate everything to English

[output]
method = "auto"       # auto | xdotool | wtype | ydotool | clipboard
type_delay_ms = 0     # inter-character delay (xdotool only); increase if chars drop
smart_space = false   # prepend a space before typed text
```

---

## CLI reference

```
wispr [COMMAND] [OPTIONS]

Commands:
  run            Start the listener (default when no command given)
  init-config    Write a default config file
  list-devices   List available audio input devices

Options for `wispr run`:
  --config PATH    Path to a custom config.toml
  --model MODEL    Whisper model size (tiny/base/small/medium/large/large-v2/large-v3)
  --key KEY        Hotkey combo (e.g. right_shift, ctrl+space)
  --method METHOD  Output method (auto/xdotool/wtype/ydotool/clipboard)
  --language LANG  Language code (en, de, fr, …) or empty for auto
  --translate      Translate to English
```

---

## Choosing a model

| Model | Size | Speed | Accuracy |
|---|---|---|---|
| `tiny` | ~75 MB | fastest | lowest |
| `base` | ~145 MB | fast | good (default) |
| `small` | ~465 MB | moderate | better |
| `medium` | ~1.5 GB | slow | high |
| `large-v3` | ~3 GB | slowest | best |

On a modern CPU, `base` transcribes in roughly real-time.  
With a GPU, even `large-v3` is near real-time.

---

## Supported hotkeys

Any key name recognised by [pynput](https://pynput.readthedocs.io/) works,
plus these aliases:

| Config value | Key |
|---|---|
| `right_shift` | Right Shift *(default)* |
| `left_shift` | Left Shift |
| `ctrl+space` | Ctrl + Space |
| `alt+r` | Alt + R |
| `f9` | F9 |
| `super+v` | Super/Windows + V |

---

## Wayland notes

On Wayland, install **wtype** (recommended) or **ydotool**:

```bash
sudo apt install wtype
# or
sudo apt install ydotool
sudo systemctl enable --now ydotoold
```

If neither typing tool is available, linux-wispr falls back to the clipboard
(`clipboard` method) and attempts to paste with Ctrl+V.

---

## Development

```bash
git clone https://github.com/Typist01/linux-wispr.git
cd linux-wispr
pip install -e .
pip install pytest
python -m pytest tests/ -v
```

---

## License

MIT
