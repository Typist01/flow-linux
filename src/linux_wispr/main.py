"""Main application logic for linux-wispr."""

from __future__ import annotations

import sys
import threading
import time
from pathlib import Path
from typing import Optional

from .audio import AudioRecorder
from .config import Config, load_config
from .hotkey import HotkeyListener
from .transcriber import Transcriber
from .typer import type_text


class WisprApp:
    """Orchestrates recording → transcription → text injection."""

    def __init__(self, config: Optional[Config] = None):
        self.config = config or load_config()
        self._recorder: Optional[AudioRecorder] = None
        self._transcriber: Optional[Transcriber] = None
        self._listener: Optional[HotkeyListener] = None
        self._recording = False
        self._lock = threading.Lock()
        self._shutdown = threading.Event()
        self._max_duration_timer: Optional[threading.Timer] = None

    # ------------------------------------------------------------------
    # Lifecycle
    # ------------------------------------------------------------------

    def setup(self) -> None:
        """Initialise all sub-components (load model, etc.)."""
        cfg = self.config

        self._recorder = AudioRecorder(
            sample_rate=cfg.audio.sample_rate,
            device=cfg.audio.device,
        )

        self._transcriber = Transcriber(
            model=cfg.whisper.model,
            device=cfg.whisper.device,
            compute_type=cfg.whisper.compute_type,
            language=cfg.whisper.language,
            translate=cfg.whisper.translate,
        )

        print(f"Loading Whisper model '{cfg.whisper.model}' …", flush=True)
        self._transcriber.load()
        print("Model ready.", flush=True)

        self._listener = HotkeyListener(
            key_combo=cfg.hotkey.key,
            on_press=self._on_hotkey_press,
            on_release=self._on_hotkey_release,
        )

    def run(self) -> None:
        """Start listening. Blocks until Ctrl-C or shutdown() is called."""
        if self._listener is None:
            self.setup()
        assert self._listener is not None

        key = self.config.hotkey.key
        print(f"linux-wispr is ready. Hold [{key}] to record. Ctrl-C to quit.", flush=True)
        self._listener.start()
        try:
            self._shutdown.wait()
        except KeyboardInterrupt:
            pass
        finally:
            self.shutdown()

    def shutdown(self) -> None:
        """Stop all threads and clean up."""
        self._shutdown.set()
        if self._listener is not None:
            self._listener.stop()
        if self._recording and self._recorder is not None:
            self._recorder.stop()
            self._recording = False

    # ------------------------------------------------------------------
    # Hotkey callbacks (called from listener thread)
    # ------------------------------------------------------------------

    def _on_hotkey_press(self) -> None:
        with self._lock:
            if self._recording:
                return
            self._recording = True

        print("● Recording …", flush=True)
        assert self._recorder is not None
        self._recorder.start()

        # Optional max-duration limit
        max_dur = self.config.audio.max_duration
        if max_dur > 0:
            self._max_duration_timer = threading.Timer(max_dur, self._on_hotkey_release)
            self._max_duration_timer.daemon = True
            self._max_duration_timer.start()

    def _on_hotkey_release(self) -> None:
        with self._lock:
            if not self._recording:
                return
            self._recording = False

        if self._max_duration_timer is not None:
            self._max_duration_timer.cancel()
            self._max_duration_timer = None

        assert self._recorder is not None
        print("■ Processing …", flush=True)
        audio = self._recorder.stop()

        # Transcribe in a background thread to avoid blocking hotkey listener
        t = threading.Thread(target=self._transcribe_and_type, args=(audio,), daemon=True)
        t.start()

    def _transcribe_and_type(self, audio) -> None:  # noqa: ANN001
        assert self._transcriber is not None
        try:
            text = self._transcriber.transcribe(audio)
        except Exception as exc:  # pragma: no cover
            print(f"Transcription error: {exc}", file=sys.stderr, flush=True)
            return

        if not text:
            print("(no speech detected)", flush=True)
            return

        print(f"→ {text}", flush=True)
        cfg_out = self.config.output
        try:
            type_text(
                text,
                method=cfg_out.method,
                delay_ms=cfg_out.type_delay_ms,
            )
        except Exception as exc:  # pragma: no cover
            print(f"Typing error: {exc}", file=sys.stderr, flush=True)
