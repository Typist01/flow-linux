"""Global hotkey listener for linux-wispr.

Uses pynput to listen for a configurable key combination. The callback
``on_press`` is called when the key is first pressed and ``on_release``
is called when it is released.
"""

from __future__ import annotations

import threading
from typing import Callable, Optional

try:
    from pynput import keyboard as _kb
    _PYNPUT_AVAILABLE = True
except ImportError:
    _PYNPUT_AVAILABLE = False


def _parse_key(key_str: str) -> tuple[frozenset[str], str]:
    """Split ``"ctrl+shift+r"`` into (modifiers frozenset, main_key str)."""
    parts = [p.strip().lower() for p in key_str.split("+")]
    modifier_names = {"ctrl", "shift", "alt", "super", "cmd", "meta", "win"}
    mods: set[str] = set()
    keys: list[str] = []
    for part in parts:
        if part in modifier_names:
            mods.add(part)
        else:
            keys.append(part)
    main = keys[-1] if keys else ""
    return frozenset(mods), main


def _pynput_key(name: str):  # noqa: ANN001
    """Convert a key name string to a pynput Key or KeyCode."""
    if not _PYNPUT_AVAILABLE:
        return None
    _ALIASES = {
        "ctrl": _kb.Key.ctrl,
        "shift": _kb.Key.shift,
        "alt": _kb.Key.alt,
        "super": _kb.Key.cmd,
        "cmd": _kb.Key.cmd,
        "meta": _kb.Key.cmd,
        "win": _kb.Key.cmd,
        "space": _kb.Key.space,
        "tab": _kb.Key.tab,
        "enter": _kb.Key.enter,
        "esc": _kb.Key.esc,
        "escape": _kb.Key.esc,
        "right_shift": _kb.Key.shift_r,
        "left_shift": _kb.Key.shift_l,
        "right_ctrl": _kb.Key.ctrl_r,
        "left_ctrl": _kb.Key.ctrl_l,
        "right_alt": _kb.Key.alt_r,
        "left_alt": _kb.Key.alt_l,
        "f1": _kb.Key.f1,
        "f2": _kb.Key.f2,
        "f3": _kb.Key.f3,
        "f4": _kb.Key.f4,
        "f5": _kb.Key.f5,
        "f6": _kb.Key.f6,
        "f7": _kb.Key.f7,
        "f8": _kb.Key.f8,
        "f9": _kb.Key.f9,
        "f10": _kb.Key.f10,
        "f11": _kb.Key.f11,
        "f12": _kb.Key.f12,
    }
    if name in _ALIASES:
        return _ALIASES[name]
    if len(name) == 1:
        return _kb.KeyCode.from_char(name)
    try:
        return getattr(_kb.Key, name)
    except AttributeError:
        return _kb.KeyCode.from_char(name)


class HotkeyListener:
    """Listens for a global hotkey and fires callbacks on press/release."""

    def __init__(
        self,
        key_combo: str,
        on_press: Optional[Callable[[], None]] = None,
        on_release: Optional[Callable[[], None]] = None,
    ):
        if not _PYNPUT_AVAILABLE:
            raise RuntimeError(
                "pynput is not installed. Run: pip install pynput"
            )
        self._key_combo = key_combo
        self._on_press_cb = on_press
        self._on_release_cb = on_release
        self._listener: Optional[_kb.Listener] = None
        self._active = False
        self._lock = threading.Lock()

        self._mods, self._main_key = _parse_key(key_combo)

        # Track currently held modifier keys
        self._held_mods: set[str] = set()
        self._main_held = False

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def start(self) -> None:
        """Start listening for the hotkey in a background thread."""
        self._listener = _kb.Listener(
            on_press=self._handle_press,
            on_release=self._handle_release,
        )
        self._listener.daemon = True
        self._listener.start()

    def stop(self) -> None:
        """Stop the background listener thread."""
        if self._listener is not None:
            self._listener.stop()
            self._listener = None

    def join(self) -> None:
        """Block until the listener thread exits."""
        if self._listener is not None:
            self._listener.join()

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    _MOD_MAP = {
        _kb.Key.ctrl: "ctrl",
        _kb.Key.ctrl_l: "ctrl",
        _kb.Key.ctrl_r: "ctrl",
        _kb.Key.shift: "shift",
        _kb.Key.shift_l: "shift",
        _kb.Key.shift_r: "shift",
        _kb.Key.alt: "alt",
        _kb.Key.alt_l: "alt",
        _kb.Key.alt_r: "alt",
        _kb.Key.cmd: "super",
        _kb.Key.cmd_l: "super",
        _kb.Key.cmd_r: "super",
    } if _PYNPUT_AVAILABLE else {}

    def _is_main_key(self, key) -> bool:  # noqa: ANN001
        """Return True if *key* matches the configured main key."""
        target = _pynput_key(self._main_key)
        if target is None:
            return False
        if isinstance(target, _kb.Key):
            return key == target
        # KeyCode comparison — compare by char or vk
        if isinstance(key, _kb.KeyCode) and isinstance(target, _kb.KeyCode):
            if target.char and key.char:
                return key.char.lower() == target.char.lower()
            return key == target
        return key == target

    def _mods_satisfied(self) -> bool:
        return self._mods.issubset(self._held_mods)

    def _handle_press(self, key) -> None:  # noqa: ANN001
        # Track modifier keys
        mod_name = self._MOD_MAP.get(key)
        if mod_name:
            with self._lock:
                self._held_mods.add(mod_name)

        # Check main key
        if self._is_main_key(key):
            with self._lock:
                if self._mods_satisfied() and not self._main_held:
                    self._main_held = True
                    should_fire = True
                else:
                    should_fire = False
            if should_fire and self._on_press_cb:
                self._on_press_cb()

    def _handle_release(self, key) -> None:  # noqa: ANN001
        fired = False
        if self._is_main_key(key):
            with self._lock:
                if self._main_held:
                    self._main_held = False
                    fired = True
            if fired and self._on_release_cb:
                self._on_release_cb()

        # Track modifier releases
        mod_name = self._MOD_MAP.get(key)
        if mod_name:
            with self._lock:
                self._held_mods.discard(mod_name)
                # Also release main key state if a required modifier was released
                if mod_name in self._mods and self._main_held:
                    self._main_held = False
                    fired = True
            if fired and not self._is_main_key(key) and self._on_release_cb:
                self._on_release_cb()
