"""Tests for the hotkey parsing helpers (no pynput required for parse tests)."""

import pytest
from unittest.mock import MagicMock, patch


# We patch pynput at import time so the module loads even in headless CI
import sys


def test_parse_key_simple():
    from linux_wispr.hotkey import _parse_key
    mods, main = _parse_key("right_shift")
    assert mods == frozenset()
    assert main == "right_shift"


def test_parse_key_modifier_combo():
    from linux_wispr.hotkey import _parse_key
    mods, main = _parse_key("ctrl+space")
    assert mods == frozenset({"ctrl"})
    assert main == "space"


def test_parse_key_multi_modifier():
    from linux_wispr.hotkey import _parse_key
    mods, main = _parse_key("ctrl+shift+r")
    assert mods == frozenset({"ctrl", "shift"})
    assert main == "r"


def test_parse_key_alt():
    from linux_wispr.hotkey import _parse_key
    mods, main = _parse_key("alt+r")
    assert mods == frozenset({"alt"})
    assert main == "r"


def test_parse_key_case_insensitive():
    from linux_wispr.hotkey import _parse_key
    mods, main = _parse_key("CTRL+Space")
    assert "ctrl" in mods
    assert main == "space"
