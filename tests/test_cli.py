"""Tests for the CLI entry point."""

import pytest
from unittest.mock import patch, MagicMock


def test_help_exits_cleanly():
    from linux_wispr.__main__ import build_parser
    parser = build_parser()
    with pytest.raises(SystemExit) as exc_info:
        parser.parse_args(["--help"])
    assert exc_info.value.code == 0


def test_list_devices_command(capsys):
    fake_devices = [{"index": 0, "name": "Microphone"}, {"index": 1, "name": "USB Audio"}]
    with patch("linux_wispr.audio.list_devices", return_value=fake_devices):
        from linux_wispr.__main__ import main
        ret = main(["list-devices"])
    assert ret == 0
    captured = capsys.readouterr()
    assert "Microphone" in captured.out
    assert "USB Audio" in captured.out


def test_init_config_command(tmp_path, monkeypatch):
    config_file = tmp_path / "config.toml"
    monkeypatch.setattr("linux_wispr.config.DEFAULT_CONFIG_PATH", config_file)

    from linux_wispr.__main__ import main
    ret = main(["init-config"])
    assert ret == 0
    assert config_file.exists()


def test_init_config_already_exists(tmp_path, monkeypatch, capsys):
    config_file = tmp_path / "config.toml"
    config_file.write_text("[hotkey]\nkey = 'right_shift'\n")
    monkeypatch.setattr("linux_wispr.config.DEFAULT_CONFIG_PATH", config_file)

    from linux_wispr.__main__ import main
    ret = main(["init-config"])
    assert ret == 0
    captured = capsys.readouterr()
    assert "already exists" in captured.out


def test_run_command_calls_app(tmp_path):
    with patch("linux_wispr.main.WisprApp") as MockApp:
        instance = MockApp.return_value
        instance.run.return_value = None
        from linux_wispr.__main__ import main
        main(["run"])
        assert MockApp.called
        instance.run.assert_called_once()
