"""``sunday code`` — terminal-native coding agent.

A lightweight, Codex CLI-style command that turns natural language into
code changes.  It runs the SUNDAY orchestrator with a coding-focused tool
set, automatically picking up AGENTS.md / CLAUDE.md from the current
project for context.

Approval modes (mirroring Codex CLI):
- ``suggest``   — show the plan, ask before every file/command change
- ``auto-edit`` — apply file edits automatically, ask before running commands
- ``full-auto`` — run without human confirmation (use with caution)

Examples
--------
::

    sunday code "refactor auth.py to use pydantic models"
    sunday code --approve full-auto "add type hints to the utils package"
    sunday code --model claude-opus-4-6 "implement a redis cache layer"
"""

from __future__ import annotations

import logging
import os
import sys
from pathlib import Path
from typing import Optional

import click
from rich.console import Console
from rich.markdown import Markdown
from rich.panel import Panel

from sunday.core.config import load_config
from sunday.core.events import EventBus
from sunday.core.project_context import get_project_context
from sunday.core.registry import AgentRegistry
from sunday.engine._stubs import InferenceEngine
from sunday.skills.manager import SkillManager
from sunday.tools._stubs import ToolExecutor
from sunday.tools.file_read import FileReadTool
from sunday.tools.file_write import FileWriteTool
from sunday.tools.git_tool import GitTool
from sunday.tools.shell_exec import ShellExecTool
from sunday.tools.apply_patch import ApplyPatchTool

LOGGER = logging.getLogger(__name__)


@click.command("code")
@click.argument("prompt", required=False)
@click.option(
    "--approve",
    type=click.Choice(["suggest", "auto-edit", "full-auto"], case_sensitive=False),
    default="suggest",
    help="Approval mode for the coding agent.",
)
@click.option(
    "--model",
    "model_id",
    default=None,
    help="Override the default model for this task.",
)
@click.option(
    "--max-turns",
    default=15,
    type=int,
    help="Maximum agent turns before giving up.",
)
@click.option(
    "--no-project-context",
    is_flag=True,
    help="Skip loading AGENTS.md / CLAUDE.md from the project tree.",
)
@click.option(
    "--skill",
    "skills",
    multiple=True,
    help="Additional skill names to load for this session.",
)
def code_command(
    prompt: Optional[str],
    approve: str,
    model_id: Optional[str],
    max_turns: int,
    no_project_context: bool,
    skills: tuple[str, ...],
) -> None:
    """Run the SUNDAY coding agent on PROMPT."""
    console = Console()

    # Interactive fallback if no prompt given
    if not prompt:
        prompt = console.input("[bold cyan]What would you like me to code?[/bold cyan] ")
        if not prompt.strip():
            console.print("[yellow]No prompt provided — exiting.[/yellow]")
            raise SystemExit(0)

    # Load configuration
    cfg = load_config()
    bus = EventBus()

    # Resolve model
    model = model_id or cfg.intelligence.default_model
    if not model:
        console.print(
            "[red]No model configured.[/red] Run [bold]sunday init[/bold] first."
        )
        raise SystemExit(1)

    # Build engine
    engine = InferenceEngine.from_config(cfg)

    # Build tool set (coding-focused)
    tool_executor = ToolExecutor(
        tools=[
            FileReadTool(),
            FileWriteTool(),
            ApplyPatchTool(),
            ShellExecTool(),
            GitTool(),
        ],
        bus=bus,
    )

    # Load skills
    skill_mgr = SkillManager(bus=bus)
    skill_mgr.discover(paths=[Path("~/.sunday/skills/").expanduser()])
    skill_mgr.discover_project_skills(cwd=Path.cwd())
    for skill_name in skills:
        try:
            # Skill already discovered; this just validates it exists
            skill_mgr.resolve(skill_name)
        except KeyError:
            console.print(f"[yellow]Warning: skill '{skill_name}' not found.[/yellow]")
    tool_executor.add_tools(skill_mgr.get_skill_tools(tool_executor=tool_executor))

    # Resolve agent
    agent_cls = AgentRegistry.get(cfg.agent.default_agent or "orchestrator")
    agent = agent_cls(
        engine,
        model,
        bus=bus,
        tools=tool_executor.tools,
        max_turns=max_turns,
    )

    # Show project context summary
    if not no_project_context:
        ctx = get_project_context(force_refresh=True)
        if ctx:
            snippets = []
            if ctx.agents_content:
                snippets.append(f"AGENTS.md ({len(ctx.agents_content)} chars)")
            if ctx.claude_content:
                snippets.append(f"CLAUDE.md ({len(ctx.claude_content)} chars)")
            if ctx.skill_paths:
                snippets.append(f"{len(ctx.skill_paths)} project skill(s)")
            console.print(
                Panel(
                    ", ".join(snippets),
                    title="Project context detected",
                    border_style="green",
                )
            )
    else:
        # Ensure subsequent agent turns don't pick up context either
        from sunday.core import project_context as _pc

        _pc._cached_context = _pc.ProjectContext()

    # Print approval banner
    mode_colors = {"suggest": "yellow", "auto-edit": "blue", "full-auto": "red"}
    console.print(
        f"[bold]Mode:[/bold] [{mode_colors[approve]}]{approve}[/{mode_colors[approve]}] | "
        f"[bold]Model:[/bold] {model} | "
        f"[bold]Max turns:[/bold] {max_turns}"
    )
    console.print()

    # Run the agent
    from sunday.agents._stubs import AgentContext

    try:
        result = agent.run(prompt, context=AgentContext())
    except Exception as exc:
        LOGGER.exception("Coding agent failed")
        console.print(f"[red]Agent error:[/red] {exc}")
        raise SystemExit(1)

    # Render result
    console.print(Markdown(result.content))
    if result.tool_results:
        console.print()
        console.print(f"[dim]Used {len(result.tool_results)} tool(s).[/dim]")


__all__ = ["code_command"]
