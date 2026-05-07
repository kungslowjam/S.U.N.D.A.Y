"""OfficialSkillsResolver — resolves skills from officialskills.sh catalog."""

from __future__ import annotations

from pathlib import Path

from sunday.skills.sources.openclaw import OpenClawResolver

OFFICIALSKILLS_REPO_URL = "https://github.com/VoltAgent/awesome-agent-skills.git"


class OfficialSkillsResolver(OpenClawResolver):
    """Resolves skills from VoltAgent/awesome-agent-skills.

    The officialskills.sh site is generated from this repository. Its README
    is a curated catalog of official and community Agent Skills.
    """

    name = "officialskills"

    def __init__(self, cache_root: Path | None = None) -> None:
        super().__init__(
            cache_root=cache_root
            or Path("~/.sunday/skill-cache/officialskills/").expanduser()
        )
        self._repo_url = OFFICIALSKILLS_REPO_URL


__all__ = ["OfficialSkillsResolver", "OFFICIALSKILLS_REPO_URL"]
