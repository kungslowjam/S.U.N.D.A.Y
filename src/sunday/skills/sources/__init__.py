"""Skill source resolvers — Hermes, OpenClaw, Official Skills, generic GitHub."""

from sunday.skills.sources.base import ResolvedSkill, SourceResolver
from sunday.skills.sources.github import GitHubResolver
from sunday.skills.sources.hermes import HERMES_REPO_URL, HermesResolver
from sunday.skills.sources.officialskills import (
    OFFICIALSKILLS_REPO_URL,
    OfficialSkillsResolver,
)
from sunday.skills.sources.openclaw import OPENCLAW_REPO_URL, OpenClawResolver

__all__ = [
    "GitHubResolver",
    "HERMES_REPO_URL",
    "HermesResolver",
    "OFFICIALSKILLS_REPO_URL",
    "OPENCLAW_REPO_URL",
    "OfficialSkillsResolver",
    "OpenClawResolver",
    "ResolvedSkill",
    "SourceResolver",
]
