"""Skill source resolvers — Hermes, OpenClaw, generic GitHub."""

from sunday.skills.sources.base import ResolvedSkill, SourceResolver
from sunday.skills.sources.github import GitHubResolver
from sunday.skills.sources.hermes import HERMES_REPO_URL, HermesResolver
from sunday.skills.sources.openclaw import OPENCLAW_REPO_URL, OpenClawResolver

__all__ = [
    "GitHubResolver",
    "HERMES_REPO_URL",
    "HermesResolver",
    "OPENCLAW_REPO_URL",
    "OpenClawResolver",
    "ResolvedSkill",
    "SourceResolver",
]
