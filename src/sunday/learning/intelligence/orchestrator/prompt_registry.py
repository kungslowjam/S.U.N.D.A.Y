"""Prompt registry for orchestrator structured mode.

Adapted from IPW's ``prompt_registry.py``.  Provides the canonical system
prompt template and tool descriptions used by the structured-mode
``OrchestratorAgent`` and by the SFT/GRPO training pipelines.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Dict, List, Optional

if TYPE_CHECKING:
    from sunday.tools._stubs import BaseTool

PROMPT_VERSION = "1.0"

SYSTEM_PROMPT_TEMPLATE = """\
You are SUNDAY, an intelligent orchestrator. You MUST solve tasks using tools. 
NEVER refuse or say you are an AI. ALWAYS be concise. MAX 50 words for greetings.
NEVER output internal "Thinking Process" or reasoning to the user.
ALWAYS respond in the same language as the user.
Structure: THOUGHT (internal), TOOL (if needed), FINAL_ANSWER (what user sees).
MANDATORY: For listing files or directory contents, you MUST use the `shell` tool (e.g., 'ls' or 'dir').
MANDATORY: For ANY mathematical calculation, you MUST use the `calculator` tool. NEVER calculate in your head.
Keep the tone professional, helpful, and extremely brief like a high-end personal assistant.

=== AVAILABLE TOOLS ===
{tools_description}

=== RESPONSE FORMAT (MANDATORY) ===
THOUGHT: <analyze task and pick tool>
TOOL: <tool name>
INPUT: <tool input>

After results:
FINAL_ANSWER: <your answer>

=== EXAMPLES ===
User: Search for Apple stock price.
THOUGHT: I should navigate to Google first.
TOOL: browser_navigate
INPUT: {"url": "https://www.google.com"}

User: (Google loaded)
THOUGHT: I'll find the search box using the accessibility tree.
TOOL: browser_get_accessibility_tree
INPUT: {}

User: Interactive Elements Found: [@1] textarea: "Search"
THOUGHT: I'll type the search query into element @1.
TOOL: browser_type
INPUT: {"selector": "@1", "text": "Apple stock price"}

=== CRITICAL RULES ===
1. NEVER answer directly. ALWAYS use at least one tool.
2. NEVER say you are an AI model who cannot browse. You have tools!
3. If user mentions a specific website (Google, Amazon, etc.) or says 'Go to', YOU MUST use `browser_navigate`.
4. If user says 'see', 'look', or 'screenshot', use `browser_screenshot` after navigating.
5. Tip: Amazon `#twotabsearchtextbox`, Google `textarea[name='q']`.
"""

TOOL_DESCRIPTIONS: Dict[str, dict] = {
    "calculator": {"category": "utility", "description": "Instant math computation."},
    "think": {"category": "utility", "description": "Reasoning scratchpad."},
    "code_interpreter": {"category": "utility", "description": "Python execution sandbox."},
    "web_search": {"category": "utility", "description": "Search info ONLY if no specific website is mentioned. Prefer browser for specific sites."},
    "file_read": {"category": "utility", "description": "Read file contents."},
    "memory_search": {"category": "memory", "description": "Search indexed docs."},
    "memory_store": {"category": "memory", "description": "Store info in memory."},
    "browser_drag": {"category": "browser", "description": "Complex mouse drag using coordinates (AntiGravity-style)."},
    "browser_scroll": {"category": "browser", "description": "Precise pixel-based scrolling."},
    "browser_get_accessibility_tree": {"category": "browser", "description": "Get clean list of interactive elements with @IDs for stable targeting."},
    "llm": {"category": "llm", "description": "Natural language generation/analysis."},
}


# Category labels for tool selection guide auto-generation
_CAT_LABELS: Dict[str, str] = {
    "math": "MATH PROBLEMS",
    "utility": "UTILITY / CODING TASKS",
    "memory": "GENERAL Q&A / FACTUAL",
    "llm": "REASONING/LOGIC",
}


def build_system_prompt(
    tool_names: Optional[List[str]] = None,
    *,
    tools: Optional[List["BaseTool"]] = None,
) -> str:
    """Build the complete system prompt for the given tools.

    Args:
        tool_names: Tool names to include.  If ``None``, uses all
            tools from :data:`TOOL_DESCRIPTIONS`.  This path is kept for
            backward compatibility with training pipelines.
        tools: Optional list of ``BaseTool`` instances.  When provided,
            rich descriptions are auto-generated from ``ToolSpec``,
            replacing the hardcoded :data:`TOOL_DESCRIPTIONS` lookup.
            Unknown / MCP tools get full descriptions instead of
            ``"Tool: {name}"``.

    Returns:
        Complete system prompt string.
    """
    # When BaseTool instances are provided, generate descriptions from spec
    if tools is not None:
        from sunday.tools._stubs import build_tool_descriptions

        desc_text = build_tool_descriptions(tools, include_cost=True)

        # Auto-generate tool selection guide by grouping tools by category
        by_cat: Dict[str, List[str]] = {}
        for t in tools:
            cat = t.spec.category or "llm"
            by_cat.setdefault(cat, []).append(t.spec.name)

        guide: list[str] = ["Choose tools based on task type:\n"]
        for cat, names in by_cat.items():
            label = _CAT_LABELS.get(cat, cat.upper())
            guide.append(f"{label}:")
            for n in names:
                guide.append(f"- {n}")
            guide.append("")

        return SYSTEM_PROMPT_TEMPLATE.format(
            tools_description=desc_text,
            tool_selection_guide="\n".join(guide),
        )

    # Backward-compat: tool_names-only path (used by training pipelines)
    if tool_names is None:
        tool_names = list(TOOL_DESCRIPTIONS)

    # Tool descriptions
    desc_lines: list[str] = []
    for name in tool_names:
        if name in TOOL_DESCRIPTIONS:
            desc = TOOL_DESCRIPTIONS[name]["description"]
        else:
            desc = f"Tool: {name}"
        desc_lines.append(f"- {name}: {desc}")

    # Group tools by category
    by_cat_names: Dict[str, List[str]] = {}
    for name in tool_names:
        cat = (
            TOOL_DESCRIPTIONS[name]["category"] if name in TOOL_DESCRIPTIONS else "llm"
        )
        by_cat_names.setdefault(cat, []).append(name)

    guide = [
        "Choose tools based on task type:\n",
    ]

    # Math
    math_lines: list[str] = []
    if "calculator" in tool_names:
        math_lines.append(
            "- Simple arithmetic/algebra -> calculator (instant, accurate)"
        )
    if "code_interpreter" in tool_names:
        math_lines.append("- Numerical algorithms -> code_interpreter (programmable)")
    if math_lines:
        guide.append("MATH PROBLEMS:")
        guide.extend(math_lines)
        guide.append("")

    # Coding
    code_lines: list[str] = []
    if "code_interpreter" in tool_names:
        code_lines.append("- Algorithm implementation/execution -> code_interpreter")
    if code_lines:
        guide.append("CODING TASKS:")
        guide.extend(code_lines)
        guide.append("")

    # Reasoning
    reasoning_lines: list[str] = []
    if "think" in tool_names:
        reasoning_lines.append(
            "- Step-by-step analysis -> think (organize thoughts first)"
        )
    llm_tools = by_cat_names.get("llm", [])
    if llm_tools:
        reasoning_lines.append(f"- Complex reasoning -> {', '.join(llm_tools)}")
    if reasoning_lines:
        guide.append("REASONING/LOGIC:")
        guide.extend(reasoning_lines)
        guide.append("")

    # General Q&A
    general_lines: list[str] = []
    if "web_search" in tool_names:
        general_lines.append("- Current events/recent info -> web_search")
    memory_tools = by_cat_names.get("memory", [])
    if memory_tools:
        general_lines.append(f"- Stored knowledge -> {', '.join(memory_tools)}")
    if general_lines:
        guide.append("GENERAL Q&A / FACTUAL:")
        guide.extend(general_lines)
        guide.append("")

    return SYSTEM_PROMPT_TEMPLATE.format(
        tools_description="\n".join(desc_lines),
        tool_selection_guide="\n".join(guide),
    )


__all__ = [
    "TOOL_DESCRIPTIONS",
    "build_system_prompt",
]
