"""Tests for project-context discovery (AGENTS.md, CLAUDE.md, SKILL.md)."""

from __future__ import annotations

from pathlib import Path

import pytest

from sunday.core.project_context import (
    ProjectContext,
    ProjectContextLoader,
    get_project_context,
)


class TestProjectContextLoader:
    def test_load_agents_md(self, tmp_path: Path) -> None:
        (tmp_path / "AGENTS.md").write_text("# Root agent\nBe helpful.")
        loader = ProjectContextLoader(cwd=tmp_path)
        ctx = loader.load()
        assert "Be helpful." in ctx.agents_content
        assert ctx.claude_content == ""

    def test_load_agents_override_precedence(self, tmp_path: Path) -> None:
        (tmp_path / "AGENTS.md").write_text("general")
        (tmp_path / "AGENTS.override.md").write_text("override")
        loader = ProjectContextLoader(cwd=tmp_path)
        ctx = loader.load()
        assert ctx.agents_content == "override"

    def test_walk_upward(self, tmp_path: Path) -> None:
        root = tmp_path / "root"
        sub = root / "sub"
        sub.mkdir(parents=True)
        (root / "AGENTS.md").write_text("root-level")
        (sub / "AGENTS.md").write_text("sub-level")
        loader = ProjectContextLoader(cwd=sub)
        ctx = loader.load()
        # Both discovered; root first, then sub (so sub overrides)
        assert ctx.agents_content == "root-level\n\nsub-level"

    def test_load_claude_md(self, tmp_path: Path) -> None:
        (tmp_path / "CLAUDE.md").write_text("Use black.")
        loader = ProjectContextLoader(cwd=tmp_path)
        ctx = loader.load()
        assert "Use black." in ctx.claude_content

    def test_system_prompt_suffix(self, tmp_path: Path) -> None:
        (tmp_path / "AGENTS.md").write_text("Be concise.")
        (tmp_path / "CLAUDE.md").write_text("Use types.")
        loader = ProjectContextLoader(cwd=tmp_path)
        ctx = loader.load()
        suffix = ctx.system_prompt_suffix()
        assert "Be concise." in suffix
        assert "Use types." in suffix
        assert "Project Instructions" in suffix
        assert "Coding Conventions" in suffix

    def test_discover_skills(self, tmp_path: Path) -> None:
        root = tmp_path / "repo"
        root.mkdir()
        (root / ".git").mkdir()  # project-root marker
        skill_dir = root / "skills" / "greet"
        skill_dir.mkdir(parents=True)
        (skill_dir / "SKILL.md").write_text("---\nname: greet\n---\nSay hello.")
        loader = ProjectContextLoader(cwd=root, skill_max_depth=3)
        ctx = loader.load()
        assert len(ctx.skill_paths) == 1
        assert ctx.skill_paths[0].name == "SKILL.md"

    def test_no_project_root_no_skills(self, tmp_path: Path, monkeypatch) -> None:
        # No project markers → skill discovery skipped
        monkeypatch.setattr(
            "sunday.core.project_context.ProjectContextLoader._find_project_root",
            staticmethod(lambda dirs: None),
        )
        loader = ProjectContextLoader(cwd=tmp_path, scan_skills=True)
        ctx = loader.load()
        assert ctx.skill_paths == []

    def test_empty_context_is_falsy(self, tmp_path: Path, monkeypatch) -> None:
        # Restrict walk to tmp_path only so ancestor files don't leak in
        monkeypatch.setattr(
            "sunday.core.project_context._walk_up",
            lambda start: [tmp_path],
        )
        loader = ProjectContextLoader(cwd=tmp_path)
        ctx = loader.load()
        assert not ctx

    def test_truthy_when_content(self, tmp_path: Path) -> None:
        (tmp_path / "AGENTS.md").write_text("x")
        loader = ProjectContextLoader(cwd=tmp_path)
        ctx = loader.load()
        assert ctx


class TestGetProjectContext:
    def test_caching(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        (tmp_path / "AGENTS.md").write_text("cached")
        monkeypatch.chdir(tmp_path)
        ctx1 = get_project_context(force_refresh=True)
        ctx2 = get_project_context()
        assert ctx1.agents_content == ctx2.agents_content

    def test_refresh(self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
        (tmp_path / "AGENTS.md").write_text("v1")
        monkeypatch.chdir(tmp_path)
        ctx1 = get_project_context(force_refresh=True)
        (tmp_path / "AGENTS.md").write_text("v2")
        ctx2 = get_project_context(force_refresh=True)
        assert ctx2.agents_content == "v2"
        assert ctx1.agents_content == "v1"
