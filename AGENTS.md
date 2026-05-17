# SUNDAY Project Instructions

## Overview
SUNDAY (Self-improving Unified Network for Desktop Automation & Yardsticking) is a high-performance autonomous agent runtime. The core is built in Rust for speed, with Python glue for agent logic, and a Tauri/React desktop dashboard.

## Architecture
- **Rust core** (`rust/crates/`): `sunday-core`, `sunday-tools`, `sunday-engine`, `sunday-sessions`, `sunday-mining`
- **Python runtime** (`src/sunday/`): agents, channels, connectors, tools, workflows, skills
- **Frontend** (`frontend/`): React + TypeScript + Tailwind, Tauri desktop wrapper
- **Examples** (`examples/`): reference implementations (browser_assistant, code_companion, etc.)

## Coding Conventions
- Use type hints everywhere (`from __future__ import annotations`)
- Follow PEP 8; run `ruff check .` and `ruff format .` before committing
- Rust code uses `cargo fmt` and `cargo clippy`
- Keep imports sorted; use absolute imports inside `src/sunday/`
- Write tests for new features in `tests/<module>/`

## Key Files
- `src/sunday/agents/_stubs.py` — BaseAgent / ToolUsingAgent ABCs
- `src/sunday/core/project_context.py` — AGENTS.md / CLAUDE.md discovery (new!)
- `src/sunday/skills/` — Skill loading, parsing, and execution
- `src/sunday/tools/` — Tool implementations (file I/O, shell, git, web, etc.)
- `pyproject.toml` — Python project metadata and dependencies
- `rust/Cargo.toml` — Rust workspace definition

## When Modifying Agents
- Register new agents with `@AgentRegistry.register("name")`
- Subclass `BaseAgent` (simple) or `ToolUsingAgent` (tool-calling)
- Respect `_max_turns` limits to prevent runaway loops
- Emit lifecycle events via `self._bus` when available

## When Modifying Tools
- Register with `@ToolRegistry.register("tool_name")`
- Subclass `BaseTool` and implement `spec` + `execute`
- Return `ToolResult` with `success=True/False`
- Never swallow exceptions; return them in `ToolResult.content`

## When Modifying Skills
- Skills live in `~/.sunday/skills/` or project-local `SKILL.md` files
- Use YAML frontmatter + markdown body format
- Keep skill names kebab-case, lowercase, max 64 chars
- Validate dependencies via `sunday.skills.dependency.validate_dependencies`

## Testing
- Run Python tests: `pytest tests/`
- Run Rust tests: `cargo test --workspace`
- Always run both when touching the Rust/Python bridge

## Commit Style
- Use conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`
- Reference issue numbers when applicable
- Keep commits atomic and reviewable
