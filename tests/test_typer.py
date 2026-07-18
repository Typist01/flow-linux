"""Tests for the typer module (no actual typing tool required)."""

import os
import subprocess
from unittest.mock import MagicMock, patch

import pytest

from linux_wispr import typer as typer_mod


# ---------------------------------------------------------------------------
# _detect_method
# ---------------------------------------------------------------------------

class TestDetectMethod:
    def test_wayland_prefers_wtype(self, monkeypatch):
        monkeypatch.setenv("XDG_SESSION_TYPE", "wayland")
        with patch("shutil.which", side_effect=lambda x: "/usr/bin/" + x if x == "wtype" else None):
            result = typer_mod._detect_method()
        assert result == "wtype"

    def test_wayland_falls_back_to_ydotool(self, monkeypatch):
        monkeypatch.setenv("XDG_SESSION_TYPE", "wayland")
        def which(cmd):
            return "/usr/bin/ydotool" if cmd == "ydotool" else None
        with patch("shutil.which", side_effect=which):
            result = typer_mod._detect_method()
        assert result == "ydotool"

    def test_wayland_falls_back_to_clipboard(self, monkeypatch):
        monkeypatch.setenv("XDG_SESSION_TYPE", "wayland")
        with patch("shutil.which", return_value=None):
            result = typer_mod._detect_method()
        assert result == "clipboard"

    def test_x11_prefers_xdotool(self, monkeypatch):
        monkeypatch.setenv("XDG_SESSION_TYPE", "x11")
        with patch("shutil.which", side_effect=lambda x: "/usr/bin/xdotool" if x == "xdotool" else None):
            result = typer_mod._detect_method()
        assert result == "xdotool"

    def test_x11_no_xdotool_falls_back_to_clipboard(self, monkeypatch):
        monkeypatch.setenv("XDG_SESSION_TYPE", "x11")
        with patch("shutil.which", return_value=None):
            result = typer_mod._detect_method()
        assert result == "clipboard"


# ---------------------------------------------------------------------------
# type_text dispatch
# ---------------------------------------------------------------------------

class TestTypeText:
    def test_empty_text_does_nothing(self):
        """type_text with empty string must not call any subprocess."""
        with patch("subprocess.run") as mock_run:
            typer_mod.type_text("")
            mock_run.assert_not_called()

    def test_xdotool_method_called(self, monkeypatch):
        monkeypatch.setenv("XDG_SESSION_TYPE", "x11")
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            typer_mod.type_text("hello", method="xdotool")
            assert mock_run.called
            cmd = mock_run.call_args[0][0]
            assert "xdotool" in cmd
            assert "hello" in cmd

    def test_wtype_method_called(self, monkeypatch):
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            typer_mod.type_text("hello", method="wtype")
            assert mock_run.called
            cmd = mock_run.call_args[0][0]
            assert "wtype" in cmd

    def test_ydotool_method_called(self):
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            typer_mod.type_text("hello", method="ydotool")
            assert mock_run.called
            cmd = mock_run.call_args[0][0]
            assert "ydotool" in cmd

    def test_xdotool_not_found_raises(self):
        with patch("subprocess.run", side_effect=FileNotFoundError):
            with pytest.raises(RuntimeError, match="xdotool not found"):
                typer_mod._type_xdotool("hello")

    def test_wtype_not_found_raises(self):
        with patch("subprocess.run", side_effect=FileNotFoundError):
            with pytest.raises(RuntimeError, match="wtype not found"):
                typer_mod._type_wtype("hello")

    def test_ydotool_not_found_raises(self):
        with patch("subprocess.run", side_effect=FileNotFoundError):
            with pytest.raises(RuntimeError, match="ydotool not found"):
                typer_mod._type_ydotool("hello")

    def test_xdotool_with_delay(self):
        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(returncode=0)
            typer_mod._type_xdotool("hello", delay_ms=20)
            cmd = mock_run.call_args[0][0]
            assert "--delay" in cmd
            assert "20" in cmd
