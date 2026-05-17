---
name: code-companion
description: Assist with coding tasks including debugging, review, and test generation
author: sunday
tags: [coding, development, debug, review, test]
required_capabilities: ["file:read", "file:write", "shell:exec"]
---

# Code Companion Skill

This skill helps developers write, review, and debug code.

## When to Use
- The user asks for help with a bug or error
- The user wants code review or refactoring suggestions
- The user needs test cases written for a function or module

## Workflow
1. **Read** the relevant source files using `file_read`
2. **Analyze** the code for issues or improvements
3. **Write** fixes or new code using `file_write` or `apply_patch`
4. **Run** tests using `shell_exec` to verify changes

## Guidelines
- Always read the existing code before proposing changes
- Follow the project's CLAUDE.md and AGENTS.md conventions
- Prefer minimal, focused changes over large rewrites
- Run the test suite after any modification
- Use `git diff` to show the user exactly what changed

## Example
```
User: "Fix the off-by-one error in pagination"
Agent: file_read → analyze → apply_patch → shell_exec(tests)
```
