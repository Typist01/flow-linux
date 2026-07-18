"""Configuration management for linux-wispr."""

from __future__ import annotations

import os
import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional


DEFAULT_CONFIG_PATH = Path.home() / ".config" / "linux-wispr" / "config.toml"

DEFAULT_CONFIG_CONTENT = """\
[hotkey]
# Key combination to hold for push-to-talk recording.
# Supported modifiers: ctrl, shift, alt, super
# Examples: "ctrl+space", "right_shift", "alt+r"
key = "right_shift"

[audio]
# Audio sample rate in Hz (16000 recommended for Whisper)
sample_rate = 16000
# Microphone device index. -1 = system default
device = -1
# Maximum recording duration in seconds (0 = unlimited)
max_duration = 30

[whisper]
# Model size: tiny, base, small, medium, large, large-v2, large-v3
model = "base"
# Computation device: "cpu", "cuda", "auto"
device = "auto"
# Computation type for faster-whisper: "int8", "float16", "float32", "auto"
compute_type = "auto"
# Language code (e.g. "en", "de", "fr"). Empty string = auto-detect
language = ""
# Translation: if true, translate speech to English
translate = false

[output]
# Typing method: "auto", "xdotool", "wtype", "ydotool", "clipboard"
# "auto" selects xdotool on X11 and wtype on Wayland
method = "auto"
# Delay between typed characters in milliseconds (xdotool only)
# Increase if characters are dropped in fast applications
type_delay_ms = 0
# If true, prepend a space before typed text when cursor is after a non-space char
smart_space = false
"""


@dataclass
class HotkeyConfig:
    key: str = "right_shift"


@dataclass
class AudioConfig:
    sample_rate: int = 16000
    device: int = -1
    max_duration: int = 30


@dataclass
class WhisperConfig:
    model: str = "base"
    device: str = "auto"
    compute_type: str = "auto"
    language: str = ""
    translate: bool = False


@dataclass
class OutputConfig:
    method: str = "auto"
    type_delay_ms: int = 0
    smart_space: bool = False


@dataclass
class Config:
    hotkey: HotkeyConfig = field(default_factory=HotkeyConfig)
    audio: AudioConfig = field(default_factory=AudioConfig)
    whisper: WhisperConfig = field(default_factory=WhisperConfig)
    output: OutputConfig = field(default_factory=OutputConfig)


def load_config(path: Optional[Path] = None) -> Config:
    """Load configuration from TOML file. Falls back to defaults if not found."""
    config_path = path or DEFAULT_CONFIG_PATH

    if not config_path.exists():
        return Config()

    with open(config_path, "rb") as f:
        data = tomllib.load(f)

    cfg = Config()

    if hotkey_data := data.get("hotkey"):
        cfg.hotkey = HotkeyConfig(**{k: v for k, v in hotkey_data.items() if hasattr(cfg.hotkey, k)})

    if audio_data := data.get("audio"):
        cfg.audio = AudioConfig(**{k: v for k, v in audio_data.items() if hasattr(cfg.audio, k)})

    if whisper_data := data.get("whisper"):
        cfg.whisper = WhisperConfig(**{k: v for k, v in whisper_data.items() if hasattr(cfg.whisper, k)})

    if output_data := data.get("output"):
        cfg.output = OutputConfig(**{k: v for k, v in output_data.items() if hasattr(cfg.output, k)})

    return cfg


def write_default_config(path: Optional[Path] = None) -> Path:
    """Write default configuration file and return its path."""
    config_path = path or DEFAULT_CONFIG_PATH
    config_path.parent.mkdir(parents=True, exist_ok=True)
    config_path.write_text(DEFAULT_CONFIG_CONTENT)
    return config_path
