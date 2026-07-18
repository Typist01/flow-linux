"""Text injection into the active window for linux-wispr.

Supports X11 via xdotool and Wayland via wtype or ydotool.
Falls back to clipboard-paste when no typing tool is available.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import time


def _detect_method() -> str:
    """Auto-detect best typing method for the current environment."""
    session_type = os.environ.get("XDG_SESSION_TYPE", "").lower()

    if session_type == "wayland":
        if shutil.which("wtype"):
            return "wtype"
        if shutil.which("ydotool"):
            return "ydotool"
        # Fall back to clipboard on Wayland if no typer found
        return "clipboard"

    # X11 or unknown
    if shutil.which("xdotool"):
        return "xdotool"
    return "clipboard"


def type_text(
    text: str,
    method: str = "auto",
    delay_ms: int = 0,
) -> None:
    """Type *text* into the currently focused window.

    Parameters
    ----------
    text:
        The string to type.
    method:
        One of ``"auto"``, ``"xdotool"``, ``"wtype"``, ``"ydotool"``,
        or ``"clipboard"``.
    delay_ms:
        Inter-character delay in milliseconds (xdotool only).
    """
    if not text:
        return

    resolved = _detect_method() if method == "auto" else method

    if resolved == "xdotool":
        _type_xdotool(text, delay_ms=delay_ms)
    elif resolved == "wtype":
        _type_wtype(text)
    elif resolved == "ydotool":
        _type_ydotool(text)
    else:
        _type_clipboard(text)


# ---------------------------------------------------------------------------
# Back-end implementations
# ---------------------------------------------------------------------------

def _type_xdotool(text: str, delay_ms: int = 0) -> None:
    cmd = ["xdotool", "type", "--clearmodifiers"]
    if delay_ms > 0:
        cmd += ["--delay", str(delay_ms)]
    cmd += ["--", text]
    try:
        subprocess.run(cmd, check=True)
    except FileNotFoundError:
        raise RuntimeError(
            "xdotool not found. Install it with: sudo apt install xdotool"
        )
    except subprocess.CalledProcessError as exc:
        raise RuntimeError(f"xdotool failed: {exc}") from exc


def _type_wtype(text: str) -> None:
    try:
        subprocess.run(["wtype", "--", text], check=True)
    except FileNotFoundError:
        raise RuntimeError(
            "wtype not found. Install it with: sudo apt install wtype"
        )
    except subprocess.CalledProcessError as exc:
        raise RuntimeError(f"wtype failed: {exc}") from exc


def _type_ydotool(text: str) -> None:
    try:
        subprocess.run(["ydotool", "type", "--", text], check=True)
    except FileNotFoundError:
        raise RuntimeError(
            "ydotool not found. Install it with: sudo apt install ydotool"
        )
    except subprocess.CalledProcessError as exc:
        raise RuntimeError(f"ydotool failed: {exc}") from exc


def _type_clipboard(text: str) -> None:
    """Copy text to the clipboard then paste via Ctrl+V (X11/Wayland fallback)."""
    session_type = os.environ.get("XDG_SESSION_TYPE", "").lower()

    if session_type == "wayland":
        # wl-clipboard
        if shutil.which("wl-copy"):
            subprocess.run(["wl-copy", "--", text], check=True)
            time.sleep(0.05)
            subprocess.run(["wtype", "-k", "ctrl+v"], check=False)
            return

    # X11 or fallback
    if shutil.which("xclip"):
        p = subprocess.run(
            ["xclip", "-selection", "clipboard"],
            input=text.encode(),
            check=True,
        )
        time.sleep(0.05)
        if shutil.which("xdotool"):
            subprocess.run(["xdotool", "key", "--clearmodifiers", "ctrl+v"], check=False)
        return

    if shutil.which("xsel"):
        subprocess.run(
            ["xsel", "--clipboard", "--input"],
            input=text.encode(),
            check=True,
        )
        time.sleep(0.05)
        if shutil.which("xdotool"):
            subprocess.run(["xdotool", "key", "--clearmodifiers", "ctrl+v"], check=False)
        return

    # Last resort: try pyperclip
    try:
        import pyperclip  # type: ignore[import-untyped]
        pyperclip.copy(text)
    except Exception as exc:
        raise RuntimeError(
            "No clipboard tool found. Install xclip, xsel, or wl-clipboard."
        ) from exc
