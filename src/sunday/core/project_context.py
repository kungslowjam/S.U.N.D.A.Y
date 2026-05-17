"""Project-context discovery — AGENTS.md, CLAUDE.md, and SKILL.md.

Implements the emerging industry convention used by OpenAI Codex CLI,
Anthropic Claude Code, OpenClaw, and Hermes Agent:

- ``AGENTS.md`` — project-level instructions discovered by walking upward
  from the current working directory.
- ``AGENTS.override.md`` — per-directory override checked before ``AGENTS.md``.
- ``CLAUDE.md`` — coding-specific project context (Claude Code convention).
- ``SKILL.md`` — skill definitions found inside the project tree.

The loader walks **upward** from ``cwd`` toward the filesystem root.
Instructions from deeper (more specific) directories are appended after
instructions from higher (more general) directories, so the specific
overrides take precedence in the prompt.
"""

from __future__ import annotations

import logging
from pathlib import Path
from typing import List, Optional

LOGGER = logging.getLogger(__name__)

# Filenames we recognise, in precedence order within a single directory.
_AGENT_FILES = ["AGENTS.override.md", "AGENTS.md"]
_CODING_FILES = ["CLAUDE.md", "claude.md"]


def _walk_up(start: Path) -> List[Path]:
    """Yield *start* and every parent up to the filesystem root."""
    dirs: List[Path] = []
    current = start.resolve()
    while True:
        dirs.append(current)
        parent = current.parent
        if parent == current:
            break
        current = parent
    return dirs


class ProjectContext:
    """Aggregated project instructions discovered from the filesystem."""

    def __init__(
        self,
        agents_content: str = "",
        claude_content: str = "",
        skill_paths: List[Path] | None = None,
    ) -> None:
        self.agents_content = agents_content
        self.claude_content = claude_content
        self.skill_paths = skill_paths or []

    def system_prompt_suffix(self) -> str:
        """Return a single string suitable for appending to a system prompt.

        Order:
        1. General AGENTS.md instructions (root → cwd, so cwd overrides)
        2. CLAUDE.md coding conventions
        """
        parts: List[str] = []
        if self.agents_content:
            parts.append("# Project Instructions\n\n" + self.agents_content)
        if self.claude_content:
            parts.append("# Coding Conventions\n\n" + self.claude_content)
        return "\n\n".join(parts)

    def __bool__(self) -> bool:
        return bool(self.agents_content or self.claude_content or self.skill_paths)


class ProjectContextLoader:
    """Discover and load AGENTS.md / CLAUDE.md / SKILL.md from a project tree."""

    def __init__(
        self,
        cwd: Optional[Path] = None,
        *,
        max_depth: int = 32,
        scan_skills: bool = True,
        skill_max_depth: int = 3,
    ) -> None:
        self._cwd = (cwd or Path.cwd()).resolve()
        self._max_depth = max_depth
        self._scan_skills = scan_skills
        self._skill_max_depth = skill_max_depth

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def load(self) -> ProjectContext:
        """Walk upward from *cwd* and aggregate all discovered context."""
        agents_parts: List[str] = []
        claude_parts: List[str] = []
        skill_paths: List[Path] = []

        dirs = _walk_up(self._cwd)[: self._max_depth]

        # AGENTS.md / CLAUDE.md — walk from root downward so that deeper
        # (more specific) instructions come *last* and override earlier ones.
        for directory in reversed(dirs):
            agents_text = self._try_load(directory, _AGENT_FILES)
            if agents_text:
                agents_parts.append(agents_text)

            claude_text = self._try_load(directory, _CODING_FILES)
            if claude_text:
                claude_parts.append(claude_text)

        # SKILL.md — scan the project tree starting at the nearest root
        # that looks like a project (contains .git, pyproject.toml, etc.)
        if self._scan_skills:
            project_root = self._find_project_root(dirs)
            if project_root:
                skill_paths = self._discover_skills(project_root)

        return ProjectContext(
            agents_content="\n\n".join(agents_parts),
            claude_content="\n\n".join(claude_parts),
            skill_paths=skill_paths,
        )

    # ------------------------------------------------------------------
    # Helpers
    # ------------------------------------------------------------------

    @staticmethod
    def _try_load(directory: Path, filenames: List[str]) -> str:
        """Return the text of the first existing *filenames* in *directory*."""
        for name in filenames:
            path = directory / name
            if path.exists() and path.is_file():
                try:
                    return path.read_text(encoding="utf-8")
                except Exception as exc:
                    LOGGER.warning("Failed to read %s: %s", path, exc)
        return ""

    @staticmethod
    def _find_project_root(dirs: List[Path]) -> Optional[Path]:
        """Return the deepest directory that looks like a project root."""
        markers = {
            ".git",
            ".hg",
            "pyproject.toml",
            "package.json",
            "Cargo.toml",
            "go.mod",
            "pom.xml",
            "build.gradle",
            "CMakeLists.txt",
            "Makefile",
            "AGENTS.md",
            "CLAUDE.md",
        }
        for directory in dirs:
            if any((directory / m).exists() for m in markers):
                return directory
        return None

    def _discover_skills(self, root: Path) -> List[Path]:
        """Find SKILL.md files under *root* up to *skill_max_depth*."""
        found: List[Path] = []
        try:
            for depth in range(1, self._skill_max_depth + 1):
                pattern = "/".join(["*"] * depth) + "/SKILL.md"
                for path in root.glob(pattern):
                    if path.is_file():
                        found.append(path)
        except Exception as exc:
            LOGGER.warning("Skill discovery failed under %s: %s", root, exc)
        return found


# Singleton cache so we don't re-read the filesystem on every agent turn.
_cached_context: Optional[ProjectContext] = None
_cached_cwd: Optional[Path] = None


def get_project_context(
    cwd: Optional[Path] = None,
    *,
    force_refresh: bool = False,
) -> ProjectContext:
    """Return the cached (or freshly loaded) project context for *cwd*.

    The result is memoised; call with ``force_refresh=True`` after a
    working-directory change or when you know files have been edited.
    """
    global _cached_context, _cached_cwd

    effective_cwd = (cwd or Path.cwd()).resolve()
    if not force_refresh and _cached_context is not None and _cached_cwd == effective_cwd:
        return _cached_context

    loader = ProjectContextLoader(cwd=effective_cwd)
    _cached_context = loader.load()
    _cached_cwd = effective_cwd
    return _cached_context


__all__ = [
    "ProjectContext",
    "ProjectContextLoader",
    "get_project_context",
]
