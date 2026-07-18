"""Tests for the transcriber module."""

import numpy as np
import pytest
from unittest.mock import MagicMock, patch


class TestTranscriber:
    def _make_transcriber(self, **kwargs):
        from linux_wispr.transcriber import Transcriber
        return Transcriber(**kwargs)

    def test_default_attrs(self):
        t = self._make_transcriber()
        assert t.model_name == "base"
        assert t.language is None
        assert t.translate is False

    def test_language_empty_string_becomes_none(self):
        t = self._make_transcriber(language="")
        assert t.language is None

    def test_language_set(self):
        t = self._make_transcriber(language="en")
        assert t.language == "en"

    def test_transcribe_empty_audio_returns_empty(self):
        t = self._make_transcriber()
        # Provide a fake loaded model so load() isn't called
        fake_model = MagicMock()
        fake_model.transcribe.return_value = (iter([]), MagicMock())
        t._model = fake_model

        result = t.transcribe(np.array([], dtype=np.float32))
        assert result == ""
        # transcribe() should not be called for empty audio
        fake_model.transcribe.assert_not_called()

    def test_transcribe_returns_joined_segments(self):
        t = self._make_transcriber()

        seg1 = MagicMock()
        seg1.text = "  Hello"
        seg2 = MagicMock()
        seg2.text = "world.  "

        fake_model = MagicMock()
        fake_model.transcribe.return_value = (iter([seg1, seg2]), MagicMock())
        t._model = fake_model

        audio = np.zeros(16000, dtype=np.float32)
        result = t.transcribe(audio)
        assert result == "Hello world."

    def test_load_raises_if_faster_whisper_missing(self, monkeypatch):
        import builtins
        real_import = builtins.__import__

        def mock_import(name, *args, **kwargs):
            if name == "faster_whisper":
                raise ImportError("no module")
            return real_import(name, *args, **kwargs)

        from linux_wispr.transcriber import Transcriber
        t = Transcriber()

        monkeypatch.setattr(builtins, "__import__", mock_import)
        with pytest.raises(RuntimeError, match="faster-whisper is not installed"):
            t.load()

    def test_translate_task(self):
        t = self._make_transcriber(translate=True)
        seg = MagicMock()
        seg.text = "bonjour"
        fake_model = MagicMock()
        fake_model.transcribe.return_value = (iter([seg]), MagicMock())
        t._model = fake_model

        audio = np.zeros(16000, dtype=np.float32)
        t.transcribe(audio)

        call_kwargs = fake_model.transcribe.call_args[1]
        assert call_kwargs.get("task") == "translate"

    def test_device_auto_resolves_to_cpu_without_torch(self, monkeypatch):
        import builtins
        real_import = builtins.__import__

        def mock_import(name, *args, **kwargs):
            if name == "torch":
                raise ImportError("no torch")
            return real_import(name, *args, **kwargs)

        monkeypatch.setattr(builtins, "__import__", mock_import)
        from linux_wispr.transcriber import Transcriber
        t = Transcriber(device="auto")
        assert t._device == "cpu"
