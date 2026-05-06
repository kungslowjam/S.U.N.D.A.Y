"""Tests for speech configuration."""

from sunday.core.config import JarvisConfig, SpeechConfig


def test_speech_config_defaults():
    cfg = SpeechConfig()
    assert cfg.backend == "auto"
    assert cfg.model == "base"
    assert cfg.language == ""
    assert cfg.device == "auto"
    assert cfg.compute_type == "float16"


def test_sunday_config_has_speech():
    cfg = JarvisConfig()
    assert hasattr(cfg, "speech")
    assert isinstance(cfg.speech, SpeechConfig)
    assert cfg.speech.backend == "auto"


def test_sunday_system_has_speech_backend():
    """JarvisSystem has a speech_backend attribute."""
    from sunday.system import JarvisSystem

    assert "speech_backend" in JarvisSystem.__dataclass_fields__
