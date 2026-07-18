"""Tests for config module."""

import tempfile
from pathlib import Path

import pytest

from linux_wispr.config import (
    Config,
    HotkeyConfig,
    AudioConfig,
    WhisperConfig,
    OutputConfig,
    load_config,
    write_default_config,
    DEFAULT_CONFIG_CONTENT,
)


def test_default_config_values():
    cfg = Config()
    assert cfg.hotkey.key == "right_shift"
    assert cfg.audio.sample_rate == 16000
    assert cfg.audio.device == -1
    assert cfg.audio.max_duration == 30
    assert cfg.whisper.model == "base"
    assert cfg.whisper.device == "auto"
    assert cfg.whisper.compute_type == "auto"
    assert cfg.whisper.language == ""
    assert cfg.whisper.translate is False
    assert cfg.output.method == "auto"
    assert cfg.output.type_delay_ms == 0
    assert cfg.output.smart_space is False


def test_load_config_returns_defaults_when_file_missing():
    cfg = load_config(Path("/nonexistent/path/config.toml"))
    assert isinstance(cfg, Config)
    assert cfg.hotkey.key == "right_shift"


def test_load_config_from_toml(tmp_path):
    config_file = tmp_path / "config.toml"
    config_file.write_text(
        """
[hotkey]
key = "ctrl+space"

[audio]
sample_rate = 44100
device = 2
max_duration = 60

[whisper]
model = "small"
device = "cpu"
compute_type = "float32"
language = "en"
translate = true

[output]
method = "xdotool"
type_delay_ms = 10
smart_space = true
"""
    )
    cfg = load_config(config_file)
    assert cfg.hotkey.key == "ctrl+space"
    assert cfg.audio.sample_rate == 44100
    assert cfg.audio.device == 2
    assert cfg.audio.max_duration == 60
    assert cfg.whisper.model == "small"
    assert cfg.whisper.device == "cpu"
    assert cfg.whisper.compute_type == "float32"
    assert cfg.whisper.language == "en"
    assert cfg.whisper.translate is True
    assert cfg.output.method == "xdotool"
    assert cfg.output.type_delay_ms == 10
    assert cfg.output.smart_space is True


def test_load_config_partial_override(tmp_path):
    """Only specified keys are overridden; others use defaults."""
    config_file = tmp_path / "config.toml"
    config_file.write_text(
        """
[whisper]
model = "large-v3"
"""
    )
    cfg = load_config(config_file)
    assert cfg.whisper.model == "large-v3"
    # Non-overridden values keep defaults
    assert cfg.hotkey.key == "right_shift"
    assert cfg.audio.sample_rate == 16000


def test_write_default_config(tmp_path):
    config_file = tmp_path / "config.toml"
    returned_path = write_default_config(config_file)
    assert returned_path == config_file
    assert config_file.exists()
    content = config_file.read_text()
    assert "[hotkey]" in content
    assert "[audio]" in content
    assert "[whisper]" in content
    assert "[output]" in content


def test_write_default_config_creates_parent_dirs(tmp_path):
    nested = tmp_path / "a" / "b" / "c" / "config.toml"
    write_default_config(nested)
    assert nested.exists()
