"""SkillManager — coordinates skill discovery, catalog, tool wrapping, and execution."""

from __future__ import annotations

import html
import logging
import re
from pathlib import Path
from typing import Any, Dict, List, Optional

LOGGER = logging.getLogger(__name__)

from sunday.core.events import EventBus
from sunday.core.project_context import ProjectContextLoader
from sunday.skills.dependency import validate_dependencies
from sunday.skills.executor import SkillExecutor, SkillResult
from sunday.skills.loader import discover_skills, load_skill_markdown
from sunday.skills.tool_adapter import SkillTool
from sunday.skills.types import SkillManifest
from sunday.tools._stubs import BaseTool, ToolExecutor


class SkillManager:
    """Coordinate skill discovery, resolution, catalog generation, and execution.

    Parameters
    ----------
    bus:
        Event bus for publishing lifecycle events.
    capability_policy:
        Optional capability policy passed through to tool executors.
    """

    def __init__(
        self,
        bus: EventBus,
        *,
        capability_policy: Optional[Any] = None,
        overlay_dir: Optional[Path] = None,
    ) -> None:
        self._bus = bus
        self._capability_policy = capability_policy
        self._skills: Dict[str, SkillManifest] = {}
        self._tool_executor: Optional[ToolExecutor] = None
        if overlay_dir is None:
            # Try to read from config first; fall back to the default
            # ~/.sunday/learning/skills/ if config can't be loaded.
            try:
                from sunday.core.config import load_config

                cfg = load_config()
                cfg_dir = getattr(
                    getattr(cfg.learning, "skills", None),
                    "overlay_dir",
                    None,
                )
                if cfg_dir:
                    overlay_dir = Path(cfg_dir).expanduser()
            except Exception:
                pass
            if overlay_dir is None:
                overlay_dir = Path("~/.sunday/learning/skills/").expanduser()
        self._overlay_dir = Path(overlay_dir).expanduser()

    # ------------------------------------------------------------------
    # Discovery
    # ------------------------------------------------------------------

    def discover(self, paths: Optional[List[Path]] = None) -> None:
        """Scan directories in order and register skills.

        First-seen name wins (workspace path listed first = highest precedence).
        After loading, the full dependency graph is validated and any
        sidecar overlays in ``overlay_dir`` are applied to discovered skills.

        Parameters
        ----------
        paths:
            Directories to scan.  If *None* or empty, no skills are loaded
            from disk — but ``_load_overlays()`` still runs (in case the
            caller had previously seeded ``self._skills`` directly).
        """
        if paths:
            for directory in paths:
                manifests = discover_skills(directory)
                for manifest in manifests:
                    # First-seen wins: do not overwrite an already-registered skill
                    if manifest.name not in self._skills:
                        self._skills[manifest.name] = manifest

            # Validate the dependency graph after loading skills
            if self._skills:
                validate_dependencies(self._skills)

        # Load optimization overlays (Plan 2A) — always runs, even when no
        # paths are provided, so callers can apply overlays to skills loaded
        # via other means.
        self._load_overlays()

    def discover_project_skills(self, cwd: Optional[Path] = None) -> List[str]:
        """Scan the current project tree for SKILL.md files and register them.

        Uses :class:`ProjectContextLoader` to find the project root and
        discover ``SKILL.md`` files up to a shallow depth.  Returns the list
        of skill names that were registered.
        """
        loader = ProjectContextLoader(cwd=cwd, scan_skills=True)
        ctx = loader.load()
        registered: List[str] = []

        for skill_path in ctx.skill_paths:
            try:
                manifest = load_skill_markdown(skill_path)
            except Exception as exc:
                LOGGER.warning("Failed to load project skill %s: %s", skill_path, exc)
                continue

            if manifest.name not in self._skills:
                self._skills[manifest.name] = manifest
                registered.append(manifest.name)

        if registered:
            LOGGER.info("Registered %d project skill(s): %s", len(registered), registered)

        return registered

    def _load_overlays(self) -> None:
        """Apply optimization overlays to discovered skills.

        For each skill, look for ``<overlay_dir>/<skill-name>/optimized.toml``.
        If present, override the manifest description and stash few-shot
        examples under ``manifest.metadata.sunday.few_shot``.

        Bad overlays are silently ignored — they should not break discovery.
        """
        from sunday.skills.overlay import SkillOverlayLoader

        loader = SkillOverlayLoader(self._overlay_dir)
        for name, manifest in self._skills.items():
            overlay = loader.load(name)
            if overlay is None:
                continue
            if overlay.description:
                manifest.description = overlay.description
            if overlay.few_shot:
                new_metadata = dict(manifest.metadata) if manifest.metadata else {}
                oj = dict(new_metadata.get("sunday", {}) or {})
                oj["few_shot"] = list(overlay.few_shot)
                new_metadata["sunday"] = oj
                manifest.metadata = new_metadata

    # ------------------------------------------------------------------
    # Resolve / introspect
    # ------------------------------------------------------------------

    def resolve(self, name: str) -> SkillManifest:
        """Return the manifest for a skill by name.

        Raises
        ------
        KeyError
            If *name* is not registered.
        """
        if name not in self._skills:
            raise KeyError(f"Skill '{name}' not found")
        return self._skills[name]

    def skill_names(self) -> List[str]:
        """Return the names of all registered skills."""
        return list(self._skills.keys())

    # ------------------------------------------------------------------
    # Tool wrapping
    # ------------------------------------------------------------------

    def get_skill_tools(
        self, *, tool_executor: Optional[ToolExecutor] = None
    ) -> List[BaseTool]:
        """Wrap each registered skill as a :class:`SkillTool` (a :class:`BaseTool`).

        Parameters
        ----------
        tool_executor:
            Tool executor to use when running skill pipelines.  Falls back to
            the one set via :meth:`set_tool_executor` if not provided here.

        Returns
        -------
        list[BaseTool]
            One :class:`SkillTool` per registered skill.
        """
        executor = tool_executor or self._tool_executor
        tools: List[BaseTool] = []

        for manifest in self._skills.values():
            real_executor = executor or _NullToolExecutor()
            skill_exec = SkillExecutor(real_executor, bus=self._bus)

            # Wire sub-skill resolver so nested skill_name steps can delegate back
            skill_exec.set_skill_resolver(self._make_resolver())

            skill_tool = SkillTool(manifest, skill_exec, skill_manager=self)
            tools.append(skill_tool)

        return tools

    def _make_resolver(self):
        """Return a resolver callback that delegates sub-skill execution."""

        def _resolver(name: str, context: Dict[str, Any]) -> SkillResult:
            manifest = self.resolve(name)
            skill_exec = SkillExecutor(
                self._tool_executor or _NullToolExecutor(),
                bus=self._bus,
            )
            skill_exec.set_skill_resolver(_resolver)
            return skill_exec.run(manifest, initial_context=context)

        return _resolver

    # ------------------------------------------------------------------
    # Catalog
    # ------------------------------------------------------------------

    def get_catalog_xml(self) -> str:
        """Generate an ``<available_skills>`` XML catalog.

        Skills with ``disable_model_invocation=True`` are excluded so that
        internal or automation-only skills are not surfaced to the model.
        """
        lines: List[str] = ["<available_skills>"]

        for manifest in self._skills.values():
            if manifest.disable_model_invocation:
                continue
            escaped_name = html.escape(manifest.name)
            escaped_desc = html.escape(manifest.description or manifest.name)
            lines.append(
                f"  <skill name={escaped_name!r} description={escaped_desc!r} />"
            )

        lines.append("</available_skills>")
        return "\n".join(lines)

    def get_few_shot_examples(self) -> List[str]:
        """Return formatted few-shot example strings ready for system prompt.

        Pulls from ``manifest.metadata.sunday.few_shot`` for every
        registered skill.  Returns one formatted string per example.
        """
        examples: List[str] = []
        for name, manifest in self._skills.items():
            oj = manifest.metadata.get("sunday", {}) if manifest.metadata else {}
            few_shot = oj.get("few_shot", []) or []
            for ex in few_shot:
                if not isinstance(ex, dict):
                    continue
                inp = str(ex.get("input", ""))
                out = str(ex.get("output", ""))
                if inp or out:
                    examples.append(f"### {name}\nInput: {inp}\nOutput: {out}")
        return examples

    def select_relevant_playbooks(
        self,
        query: str,
        *,
        limit: int = 1,
        max_chars_per_skill: int = 1400,
    ) -> List[Dict[str, str]]:
        """Return likely matching SKILL.md playbooks for Codex-style injection.

        This keeps the OpenJarvis contract (every skill is still a tool) while
        adding Codex-style behavior for instruction-only skills: the most
        relevant playbooks are loaded into the agent prompt before the model
        decides which tools to use.
        """
        query_l = query.lower()
        query_terms = {
            term
            for term in re.findall(r"[a-zA-Z0-9_\-]+", query_l)
            if len(term) >= 3
        }
        scored: List[tuple[int, str, SkillManifest]] = []

        for manifest in self._skills.values():
            if manifest.disable_model_invocation or not manifest.markdown_content:
                continue

            haystack = " ".join(
                [
                    manifest.name,
                    manifest.description or "",
                    " ".join(manifest.tags or []),
                    manifest.markdown_content[:1200],
                ]
            ).lower()
            score = 0
            for term in query_terms:
                if term in haystack:
                    score += 3 if term in manifest.name.lower() else 1

            if any(
                phrase in query_l
                for phrase in (
                    "research paper",
                    "reseach paper",
                    "paper",
                    "arxiv",
                    "academic",
                    "literature",
                    "วิจัย",
                    "บทความ",
                    "เปเปอร์",
                )
            ):
                if any(k in haystack for k in ("arxiv", "research", "paper")):
                    score += 8

            if any(
                phrase in query_l
                for phrase in (
                    "find",
                    "search",
                    "lookup",
                    "หา",
                    "ค้น",
                    "ค้นหา",
                )
            ) and any(
                phrase in query_l
                for phrase in ("paper", "research", "วิจัย", "บทความ", "เปเปอร์")
            ):
                if "arxiv" in haystack:
                    score += 12
                if "writing" in haystack or "write " in haystack:
                    score -= 10

            if score > 0:
                scored.append((score, manifest.name, manifest))

        scored.sort(key=lambda item: (-item[0], item[1]))
        playbooks: List[Dict[str, str]] = []
        for _, _, manifest in scored[:limit]:
            content = manifest.markdown_content.strip()
            if len(content) > max_chars_per_skill:
                content = content[:max_chars_per_skill].rstrip() + "\n\n[Skill truncated]"
            playbooks.append(
                {
                    "name": manifest.name,
                    "description": manifest.description or manifest.name,
                    "content": content,
                }
            )
        return playbooks

    # ------------------------------------------------------------------
    # Trace-driven skill discovery (Plan 2A)
    # ------------------------------------------------------------------

    def discover_from_traces(
        self,
        trace_store: Any,
        *,
        min_frequency: int = 3,
        min_outcome: float = 0.5,
        output_dir: Optional[Path] = None,
    ) -> List[Dict[str, Any]]:
        """Mine the trace store for recurring tool sequences.

        For each recurring sequence found by :class:`SkillDiscovery`, write
        a TOML skill manifest into *output_dir* (default
        ``~/.sunday/skills/discovered/``).  Returns a list of dicts with
        ``name`` and ``path`` for each manifest written.

        Names are normalized to spec-compliant kebab-case (lowercase with
        hyphens, no underscores) so the resulting manifests load cleanly
        through the discovery walker.
        """
        from sunday.learning.agents.skill_discovery import SkillDiscovery

        traces = trace_store.list_traces(limit=10000)
        discovery = SkillDiscovery(
            min_frequency=min_frequency,
            min_outcome=min_outcome,
        )
        discovered = discovery.analyze_traces(traces)

        if output_dir is None:
            output_dir = Path("~/.sunday/skills/discovered/").expanduser()
        output_dir = Path(output_dir).expanduser()
        output_dir.mkdir(parents=True, exist_ok=True)

        written: List[Dict[str, Any]] = []
        for skill in discovered:
            name = self._normalize_skill_name(skill.name)
            skill_subdir = output_dir / name
            skill_subdir.mkdir(parents=True, exist_ok=True)
            toml_path = skill_subdir / "skill.toml"
            toml_path.write_text(
                self._serialize_discovered_skill(name, skill),
                encoding="utf-8",
            )
            written.append({"name": name, "path": str(toml_path)})

        return written

    @staticmethod
    def _normalize_skill_name(raw_name: str) -> str:
        """Convert an arbitrary discovered name to a spec-compliant kebab name.

        - Lowercase
        - Replace underscores and whitespace with hyphens
        - Collapse runs of hyphens
        - Strip leading/trailing hyphens
        """
        import re

        normalized = raw_name.lower()
        normalized = re.sub(r"[_\s]+", "-", normalized)
        normalized = re.sub(r"-+", "-", normalized)
        normalized = normalized.strip("-")
        if not normalized:
            normalized = "discovered-skill"
        return normalized

    @staticmethod
    def _serialize_discovered_skill(name: str, skill: Any) -> str:
        """Serialize a DiscoveredSkill into a spec-compliant skill.toml."""
        lines: List[str] = ["[skill]"]
        lines.append(f'name = "{name}"')
        lines.append('version = "0.1.0"')
        # Truncate description to spec max 1024 chars
        description = (skill.description or f"Discovered skill: {name}")[:1024]
        # Escape backslashes and double quotes for basic TOML strings
        description = description.replace("\\", "\\\\").replace('"', '\\"')
        lines.append(f'description = "{description}"')
        lines.append('author = "sunday (auto-discovered)"')
        lines.append('tags = ["auto-discovered"]')
        lines.append("")

        for tool_name in skill.tool_sequence:
            lines.append("[[skill.steps]]")
            lines.append(f'tool_name = "{tool_name}"')
            lines.append('arguments_template = "{}"')
            lines.append('output_key = ""')
            lines.append("")

        return "\n".join(lines)

    # ------------------------------------------------------------------
    # Execution
    # ------------------------------------------------------------------

    def execute(
        self,
        name: str,
        context: Optional[Dict[str, Any]] = None,
        *,
        trace_id: Optional[str] = None,
    ) -> SkillResult:
        """Resolve and execute a skill by name.

        Parameters
        ----------
        name:
            Skill name to execute.
        context:
            Initial context dict passed to the executor.
        trace_id:
            Optional trace identifier forwarded to the skill-evolution engine.

        Returns
        -------
        SkillResult
        """
        import uuid

        manifest = self.resolve(name)
        executor = SkillExecutor(
            self._tool_executor or _NullToolExecutor(),
            bus=self._bus,
        )
        executor.set_skill_resolver(self._make_resolver())
        return executor.run(
            manifest, initial_context=context, trace_id=trace_id or str(uuid.uuid4())
        )

    # ------------------------------------------------------------------
    # Configuration
    # ------------------------------------------------------------------

    def set_tool_executor(self, tool_executor: ToolExecutor) -> None:
        """Attach a :class:`ToolExecutor` for running tool steps in skill pipelines."""
        self._tool_executor = tool_executor

    # ------------------------------------------------------------------
    # Skill evolution (Hermes-style closed loop)
    # ------------------------------------------------------------------

    def process_skills_batch(self, limit: int = 50) -> List[Dict[str, Any]]:
        """Run the Rust skill-evolution engine over recent traces.

        Discovers recurring tool patterns, writes new SKILL.md files to
        ``~/.sunday/skills/discovered/``, and returns metadata for each
        discovered or updated skill.

        Parameters
        ----------
        limit:
            Maximum number of new traces to process in this batch.

        Returns
        -------
        list[dict]
            Discovered skill records with ``name``, ``confidence``, etc.
        """
        import json as _json

        from sunday._rust_bridge import get_skill_evolution_engine

        engine = get_skill_evolution_engine()
        if engine is None:
            LOGGER.warning("SkillEvolutionEngine not available in Rust backend")
            return []
        try:
            raw = engine.process_batch(limit)
            skills = _json.loads(raw)
            LOGGER.info(
                "Skill evolution batch complete: %d skill(s) discovered/updated",
                len(skills),
            )
            return skills if isinstance(skills, list) else []
        except Exception as exc:
            LOGGER.warning("Skill evolution batch failed: %s", exc)
            return []

    def iterate_skills(self) -> List[Dict[str, Any]]:
        """Re-evaluate existing discovered skills and upgrade their confidence.

        Returns a list of ``{"name": str, "confidence": float}`` dicts.
        """
        import json as _json

        from sunday._rust_bridge import get_skill_evolution_engine

        engine = get_skill_evolution_engine()
        if engine is None:
            LOGGER.warning("SkillEvolutionEngine not available in Rust backend")
            return []
        try:
            raw = engine.iterate_skills()
            result = _json.loads(raw)
            return result if isinstance(result, list) else []
        except Exception as exc:
            LOGGER.warning("Skill iteration failed: %s", exc)
            return []

    def get_skill_stats(self, name: str) -> Dict[str, Any]:
        """Return performance stats for a skill from the evolution engine."""
        import json as _json

        from sunday._rust_bridge import get_skill_evolution_engine

        engine = get_skill_evolution_engine()
        if engine is None:
            return {}
        try:
            raw = engine.get_skill_stats(name)
            return _json.loads(raw)
        except Exception:
            return {}

    # ------------------------------------------------------------------
    # Lifecycle
    # ------------------------------------------------------------------

    def find_installed_paths(
        self, name: str, *, roots: Optional[List[Path]] = None
    ) -> List[Path]:
        """Return on-disk skill directories matching ``name``.

        A directory matches when it contains ``skill.toml`` or ``SKILL.md``
        and either the directory name equals ``name`` or its parsed
        manifest's ``name`` field equals ``name``.
        """
        if roots is None:
            roots = [Path("~/.sunday/skills/").expanduser(), Path("./skills")]

        matches: List[Path] = []
        for root in roots:
            if not root.exists():
                continue
            for candidate in root.rglob("*"):
                if not candidate.is_dir():
                    continue
                toml = candidate / "skill.toml"
                md = candidate / "SKILL.md"
                if not (toml.exists() or md.exists()):
                    continue
                if candidate.name == name:
                    matches.append(candidate)
                    continue
                # Fall back to parsed manifest name
                try:
                    from sunday.skills.loader import load_skill_directory

                    manifest = load_skill_directory(candidate)
                    if manifest is not None and manifest.name == name:
                        matches.append(candidate)
                except Exception:
                    continue
        return matches

    def remove(self, name: str, *, roots: Optional[List[Path]] = None) -> List[Path]:
        """Remove an installed skill by name.

        Returns the list of directories that were removed.  Raises
        :class:`FileNotFoundError` when no matching skill exists on disk.
        """
        import shutil

        paths = self.find_installed_paths(name, roots=roots)
        if not paths:
            raise FileNotFoundError(f"No installed skill named {name!r}")
        for p in paths:
            shutil.rmtree(p)
        # Drop from in-memory catalog
        self._skills.pop(name, None)
        return paths


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------


class _NullToolExecutor(ToolExecutor):
    """A no-op ToolExecutor used when no real executor is available.

    Allows SkillTool/SkillExecutor construction to succeed even before a
    real tool executor is wired in; any actual tool call will produce an
    error ToolResult rather than raising.
    """

    def __init__(self) -> None:
        super().__init__(tools=[], bus=None)


__all__ = ["SkillManager"]
