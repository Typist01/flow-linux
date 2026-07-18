"""Audio recording utilities for linux-wispr."""

from __future__ import annotations

import io
import threading
import wave
from typing import Optional

import numpy as np

try:
    import sounddevice as sd
    _SOUNDDEVICE_AVAILABLE = True
except (ImportError, OSError):
    _SOUNDDEVICE_AVAILABLE = False


class AudioRecorder:
    """Records audio from a microphone while active."""

    def __init__(self, sample_rate: int = 16000, device: Optional[int] = None):
        if not _SOUNDDEVICE_AVAILABLE:
            raise RuntimeError(
                "sounddevice is not installed. Run: pip install sounddevice"
            )
        self.sample_rate = sample_rate
        self.device = device if (device is not None and device >= 0) else None
        self._frames: list[np.ndarray] = []
        self._stream: Optional[sd.InputStream] = None
        self._lock = threading.Lock()
        self._recording = False

    def start(self) -> None:
        """Start capturing audio from the microphone."""
        if self._recording:
            return
        with self._lock:
            self._frames = []
            self._recording = True
        self._stream = sd.InputStream(
            samplerate=self.sample_rate,
            channels=1,
            dtype="int16",
            device=self.device,
            callback=self._callback,
        )
        self._stream.start()

    def stop(self) -> np.ndarray:
        """Stop capturing and return raw PCM audio as a float32 numpy array."""
        if not self._recording:
            return np.array([], dtype=np.float32)
        self._recording = False
        if self._stream is not None:
            self._stream.stop()
            self._stream.close()
            self._stream = None
        with self._lock:
            frames = list(self._frames)
        if not frames:
            return np.array([], dtype=np.float32)
        audio = np.concatenate(frames, axis=0).flatten()
        # Normalise int16 → float32 in [-1, 1]
        return audio.astype(np.float32) / 32768.0

    def stop_as_wav_bytes(self) -> bytes:
        """Stop capturing and return WAV-encoded bytes."""
        audio = self.stop()
        buf = io.BytesIO()
        with wave.open(buf, "wb") as wf:
            wf.setnchannels(1)
            wf.setsampwidth(2)  # int16
            wf.setframerate(self.sample_rate)
            pcm = (audio * 32768).astype(np.int16)
            wf.writeframes(pcm.tobytes())
        return buf.getvalue()

    def _callback(self, indata: np.ndarray, frames: int, time, status) -> None:  # noqa: ANN001
        if self._recording:
            with self._lock:
                self._frames.append(indata.copy())


def list_devices() -> list[dict]:
    """Return a list of available audio input devices."""
    if not _SOUNDDEVICE_AVAILABLE:
        return []
    devices = []
    for i, dev in enumerate(sd.query_devices()):
        if dev["max_input_channels"] > 0:
            devices.append({"index": i, "name": dev["name"]})
    return devices
