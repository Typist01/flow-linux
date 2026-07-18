"""Whisper speech-to-text transcription backend for linux-wispr."""

from __future__ import annotations

from typing import Optional

import numpy as np


class Transcriber:
    """Wraps faster-whisper to transcribe audio."""

    def __init__(
        self,
        model: str = "base",
        device: str = "auto",
        compute_type: str = "auto",
        language: str = "",
        translate: bool = False,
    ):
        self.model_name = model
        self.language: Optional[str] = language or None
        self.translate = translate
        self._model = None

        # Resolve 'auto' device
        if device == "auto":
            try:
                import torch
                self._device = "cuda" if torch.cuda.is_available() else "cpu"
            except ImportError:
                self._device = "cpu"
        else:
            self._device = device

        # Resolve 'auto' compute type
        if compute_type == "auto":
            self._compute_type = "float16" if self._device == "cuda" else "int8"
        else:
            self._compute_type = compute_type

    def load(self) -> None:
        """Load the Whisper model into memory (call once at startup)."""
        try:
            from faster_whisper import WhisperModel
        except ImportError as exc:
            raise RuntimeError(
                "faster-whisper is not installed. Run: pip install faster-whisper"
            ) from exc

        self._model = WhisperModel(
            self.model_name,
            device=self._device,
            compute_type=self._compute_type,
        )

    def transcribe(self, audio: np.ndarray) -> str:
        """Transcribe a float32 numpy array (16 kHz mono) and return the text."""
        if self._model is None:
            self.load()

        if audio.size == 0:
            return ""

        task = "translate" if self.translate else "transcribe"
        segments, _info = self._model.transcribe(
            audio,
            language=self.language,
            task=task,
            vad_filter=True,
        )
        return " ".join(seg.text.strip() for seg in segments).strip()
