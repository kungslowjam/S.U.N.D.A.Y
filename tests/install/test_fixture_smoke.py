"""Smoke test that the tmp_sunday_home fixture works."""

from __future__ import annotations

from pathlib import Path

from sunday.core import config as config_mod


def test_fixture_redirects_default_config_dir(tmp_sunday_home: Path) -> None:
    assert config_mod.DEFAULT_CONFIG_DIR == tmp_sunday_home
    assert tmp_sunday_home.exists()
    assert (tmp_sunday_home / ".state").exists()
    assert (tmp_sunday_home / ".state" / "models").exists()


def test_fixture_redirects_config_path(tmp_sunday_home: Path) -> None:
    assert config_mod.DEFAULT_CONFIG_PATH == tmp_sunday_home / "config.toml"
