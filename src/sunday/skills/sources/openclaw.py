"""OpenClawResolver — resolves skills from the OpenClaw skill index.

Layout:
    skills/<owner>/<skill-name>/SKILL.md
    skills/<owner>/<skill-name>/_meta.json  (optional sidecar registry data)
"""

from __future__ import annotations

import json
import logging
import re
import subprocess
from pathlib import Path
from typing import List

import yaml

from sunday.skills.sources.base import ResolvedSkill, SourceResolver

LOGGER = logging.getLogger(__name__)

OPENCLAW_REPO_URL = "https://github.com/VoltAgent/awesome-openclaw-skills.git"


class OpenClawResolver(SourceResolver):
    """Resolves skills from the OpenClaw skill index."""

    name = "openclaw"

    def __init__(self, cache_root: Path | None = None) -> None:
        if cache_root is None:
            cache_root = Path("~/.sunday/skill-cache/openclaw/").expanduser()
        self._cache_root = Path(cache_root)
        self._repo_url = OPENCLAW_REPO_URL

    def cache_dir(self) -> Path:
        return self._cache_root

    def sync(self) -> None:
        if self._cache_root.exists() and (self._cache_root / ".git").exists():
            subprocess.run(
                ["git", "-C", str(self._cache_root), "pull", "--ff-only"],
                check=True,
            )
        else:
            self._cache_root.parent.mkdir(parents=True, exist_ok=True)
            subprocess.run(
                ["git", "clone", "--depth", "1", self._repo_url, str(self._cache_root)],
                check=True,
            )

    def list_skills(self) -> List[ResolvedSkill]:
        skills_root = self._cache_root / "skills"
        if not skills_root.exists():
            return self._list_from_readme()

        results: List[ResolvedSkill] = []
        commit = self._read_commit()

        for owner_dir in sorted(skills_root.iterdir()):
            if not owner_dir.is_dir():
                continue
            for skill_dir in sorted(owner_dir.iterdir()):
                if not skill_dir.is_dir():
                    continue
                skill_md = skill_dir / "SKILL.md"
                if not skill_md.exists():
                    continue

                name, description = self._read_preview(
                    skill_md, default_name=skill_dir.name
                )
                sidecar = self._read_sidecar(skill_dir / "_meta.json")
                results.append(
                    ResolvedSkill(
                        name=name,
                        source=self.name,
                        path=skill_dir,
                        category=owner_dir.name,
                        description=description,
                        commit=commit,
                        sidecar_data=sidecar,
                    )
                )

        return results

    def _list_from_readme(self) -> List[ResolvedSkill]:
        """Parse curated awesome-list markdown when SKILL.md files are absent."""
        catalog_files = [self._cache_root / "README.md"]
        categories_root = self._cache_root / "categories"
        if categories_root.exists():
            catalog_files.extend(sorted(categories_root.glob("*.md")))
        catalog_files = [p for p in catalog_files if p.exists()]
        if not catalog_files:
            return []

        results: List[ResolvedSkill] = []
        commit = self._read_commit()
        seen: set[str] = set()

        for catalog_file in catalog_files:
            try:
                raw = catalog_file.read_text(encoding="utf-8")
            except OSError:
                continue
            category = self._category_from_catalog_file(catalog_file)
            current_category = category
            for line in raw.splitlines():
                next_category = self._parse_catalog_category(line)
                if next_category:
                    current_category = next_category
                    continue

                parsed = self._parse_catalog_line(
                    line, current_category, catalog_file, commit
                )
                if parsed is None or parsed.name in seen:
                    continue
                seen.add(parsed.name)
                results.append(parsed)

        return results

    def _parse_catalog_line(
        self,
        line: str,
        category: str,
        catalog_file: Path,
        commit: str,
    ) -> ResolvedSkill | None:
        stripped = line.strip()

        bullet_match = re.match(
            r"-\s+(?:\*\*)?\[([^\]]+)\]\(([^)]+)\)(?:\*\*)?\s*-\s*(.*)",
            stripped,
        )
        if not bullet_match:
            return None

        name = self._clean_markdown_cell(bullet_match.group(1))
        if not name:
            return None

        url = bullet_match.group(2).strip()
        description = self._clean_markdown_cell(bullet_match.group(3))
        return ResolvedSkill(
            name=name,
            source=self.name,
            path=catalog_file,
            category=category,
            description=description,
            commit=commit,
            sidecar_data={"catalog_only": True, "url": url},
        )

    def _parse_catalog_category(self, line: str) -> str:
        stripped = line.strip()
        detail_match = re.match(r"<summary><h3[^>]*>(.*?)</h3></summary>", stripped)
        if detail_match:
            return self._clean_markdown_cell(detail_match.group(1))
        heading_match = re.match(r"#{2,4}\s+(.+)", stripped)
        if heading_match:
            return self._clean_markdown_cell(heading_match.group(1))
        return ""

    def _clean_markdown_cell(self, value: str) -> str:
        value = value.replace("`", "").strip()
        if "](" in value and value.startswith("["):
            label_end = value.find("]")
            if label_end > 1:
                value = value[1:label_end]
        return value.strip()

    def _category_from_catalog_file(self, catalog_file: Path) -> str:
        if catalog_file.name == "README.md":
            return ""
        try:
            raw = catalog_file.read_text(encoding="utf-8")
        except OSError:
            return ""
        for line in raw.splitlines():
            if line.startswith("# "):
                return self._clean_markdown_cell(line.lstrip("#").strip())
        return catalog_file.stem.replace("-", " ").title()

    def _read_preview(self, skill_md: Path, default_name: str) -> tuple[str, str]:
        try:
            raw = skill_md.read_text(encoding="utf-8")
        except Exception:
            return default_name, ""
        if not raw.startswith("---"):
            return default_name, ""
        rest = raw[3:].lstrip("\n")
        end = rest.find("\n---")
        if end == -1:
            return default_name, ""
        try:
            fm = yaml.safe_load(rest[:end])
        except yaml.YAMLError:
            return default_name, ""
        if not isinstance(fm, dict):
            return default_name, ""
        return str(fm.get("name", default_name)), str(fm.get("description", ""))

    def _read_sidecar(self, sidecar_path: Path) -> dict:
        if not sidecar_path.exists():
            return {}
        try:
            return json.loads(sidecar_path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            return {}

    def _read_commit(self) -> str:
        if not (self._cache_root / ".git").exists():
            return ""
        try:
            result = subprocess.run(
                ["git", "-C", str(self._cache_root), "rev-parse", "HEAD"],
                capture_output=True,
                text=True,
                check=True,
            )
            return result.stdout.strip()
        except subprocess.CalledProcessError:
            return ""


__all__ = ["OpenClawResolver", "OPENCLAW_REPO_URL"]
