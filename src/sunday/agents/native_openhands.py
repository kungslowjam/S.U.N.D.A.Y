"""NativeOpenHandsAgent -- code-execution-centric agent.

Renamed from ``OpenHandsAgent`` to clarify this is SUNDAY's native
CodeAct-style implementation.  The ``OpenHandsAgent`` name is now used
for the real openhands-sdk integration in ``openhands.py``.
"""

from __future__ import annotations

import json as _json
import re
from typing import Any, List, Optional

from sunday.agents._stubs import AgentContext, AgentResult, ToolUsingAgent
from sunday.agents.prompt_loader import (
    load_few_shot_exemplars,
    load_system_prompt_override,
)
from sunday.core.events import EventBus
from sunday.core.registry import AgentRegistry
from sunday.core.types import Message, Role, ToolCall, ToolResult
from sunday.engine._stubs import InferenceEngine
from sunday.tools._stubs import BaseTool, build_tool_descriptions

OPENHANDS_SYSTEM_PROMPT = (  # noqa: E501
    "You are an AI assistant with access to tools. "
    "You MUST use tools when they would help answer "
    "the user's question.\n\n"
    "## How to use tools\n\n"
    "To call a tool, write on its own lines:\n\n"
    "Action: <tool_name>\n"
    "Action Input: <json_arguments>\n\n"
    "You will receive the result, then continue your "
    "response.\n\n"
    "## How to use skills\n\n"
    "Tools whose names begin with `skill_` are SKILLS. Skills are reusable "
    "playbooks or pipelines for specific tasks. When a user request matches "
    "a skill description, call the matching `skill_*` tool before using "
    "generic tools. If the skill returns markdown instructions, read them "
    "as your playbook, then follow those instructions using the other tools. "
    "Do not call the same instructional skill repeatedly in the same turn.\n\n"
    "Prefer skills for domain workflows such as academic paper search, "
    "research synthesis, code explanation, data analysis, file creation, "
    "and other tasks where a matching skill exists. Use generic tools like "
    "`web_search` only when no matching skill exists or when the skill "
    "instructs you to search.\n\n"
    "## Available tools\n\n"
    "{tool_descriptions}\n\n"
    "{skill_examples}"
    "## Important rules\n\n"
    "- When the user asks you to look up, search, fetch, "
    "or summarize a URL or topic, use a matching `skill_*` first when one "
    "exists; otherwise use web_search. Do NOT say you cannot browse the web.\n"
    "- When the user provides a URL, pass the FULL URL "
    "(including https://) as the query to web_search. "
    "Do NOT rewrite URLs into search keywords.\n"
    "- When the user asks a math question, use calculator.\n"
    "- When the user asks to read a file, use file_read.\n"
    "- You CAN write Python code in ```python blocks and "
    "it will be executed. Use this for computation, data "
    "processing, or when no specific tool fits.\n"
    "- If no tool or code is needed, respond directly "
    "with your answer.\n"
    "- Do NOT include <think> tags or internal reasoning "
    "in your response. Respond directly."
)


@AgentRegistry.register("native_openhands")
class NativeOpenHandsAgent(ToolUsingAgent):
    """Native CodeAct agent -- generates and executes Python code."""

    agent_id = "native_openhands"
    _default_temperature = 0.7
    _default_max_tokens = 2048
    _default_max_turns = 3

    def __init__(
        self,
        engine: InferenceEngine,
        model: str,
        *,
        tools: Optional[List[BaseTool]] = None,
        bus: Optional[EventBus] = None,
        max_turns: Optional[int] = None,
        temperature: Optional[float] = None,
        max_tokens: Optional[int] = None,
        interactive: bool = False,
        confirm_callback=None,
        skill_few_shot_examples: Optional[List[str]] = None,
        skill_playbooks: Optional[List[dict[str, str]]] = None,
    ) -> None:
        super().__init__(
            engine,
            model,
            tools=tools,
            bus=bus,
            max_turns=max_turns,
            temperature=temperature,
            max_tokens=max_tokens,
            interactive=interactive,
            confirm_callback=confirm_callback,
            skill_few_shot_examples=skill_few_shot_examples,
        )
        self._skill_playbooks = list(skill_playbooks or [])

    @staticmethod
    def _expand_urls(text: str) -> tuple[str, bool]:
        """If the user message contains a URL, fetch it and inline the content.

        Returns (possibly_expanded_text, was_expanded).
        """
        import re as _re

        url_match = _re.search(r"https?://[^\s,;\"'<>]+", text)
        if not url_match:
            return text, False
        url = url_match.group(0).rstrip(".,;)")
        try:
            from sunday.tools.web_search import WebSearchTool

            content = WebSearchTool._fetch_url(url, max_chars=4000)
            header = f"\n\n--- Content from {url} ---\n"
            footer = "\n--- End of content ---\n"
            expanded = text.replace(url, f"{header}{content}{footer}")
            return expanded, True
        except Exception:
            return text, False

    def _truncate_if_needed(
        self,
        messages: list[Message],
        max_prompt_tokens: int = 3000,
    ) -> list[Message]:
        """Truncate messages if estimated token count exceeds limit."""
        total_chars = sum(len(m.content) for m in messages)
        estimated_tokens = total_chars // 4
        if estimated_tokens <= max_prompt_tokens:
            return messages
        # Find the last user message and truncate its content
        for i in range(len(messages) - 1, -1, -1):
            if messages[i].role == Role.USER:
                excess_tokens = estimated_tokens - max_prompt_tokens
                excess_chars = excess_tokens * 4
                original = messages[i].content
                if len(original) > excess_chars + 200:
                    truncated = original[: len(original) - excess_chars]
                    messages[i] = Message(
                        role=Role.USER,
                        content=(
                            truncated + "\n\n[Input truncated to fit context window]"
                        ),
                    )
                break
        return messages

    def _build_relevant_skill_hints(self, user_input: str, limit: int = 6) -> str:
        """Surface likely matching skills near the top of the prompt.

        The complete tool list can be long. A compact relevance hint makes it
        much easier for small local models to pick a `skill_*` tool before
        falling back to generic tools such as web_search.
        """
        skill_specs = [
            tool.spec
            for tool in self._tools
            if tool.spec.category == "skill" or tool.spec.name.startswith("skill_")
        ]
        if not skill_specs:
            return ""

        query = user_input.lower()
        query_terms = {
            term
            for term in re.findall(r"[a-zA-Z0-9_\-]+", query)
            if len(term) >= 3
        }
        scored: list[tuple[int, str, str]] = []

        for spec in skill_specs:
            haystack = f"{spec.name} {spec.description}".lower()
            score = 0
            for term in query_terms:
                if term in haystack:
                    score += 2 if term in spec.name.lower() else 1

            # Common academic/research requests should strongly consider
            # academic skills before generic web search.
            if any(
                phrase in query
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
                    score += 6

            if score > 0:
                scored.append((score, spec.name, spec.description))

        if not scored:
            return ""

        scored.sort(key=lambda item: (-item[0], item[1]))
        lines = [
            "## Relevant skills for this request",
            "Prefer these skills before generic tools when they match:",
        ]
        for _, name, desc in scored[:limit]:
            lines.append(f"- {name}: {desc}")
        return "\n".join(lines) + "\n\n"

    def _build_skill_playbooks_block(self) -> str:
        if not self._skill_playbooks:
            return ""

        lines = [
            "## Active Skill Playbooks",
            "The following skills were selected for this request. Treat them as "
            "task-specific instructions. Follow the matching playbook using your "
            "normal tools. Only call `skill_*` directly when you need a pipeline "
            "result or the playbook explicitly tells you to.",
        ]
        for skill in self._skill_playbooks:
            name = skill.get("name", "unknown-skill")
            desc = skill.get("description", name)
            content = skill.get("content", "").strip()
            lines.extend(
                [
                    "",
                    f"### {name}",
                    desc,
                    "",
                    content,
                ]
            )
        return "\n".join(lines) + "\n\n"

    @staticmethod
    def _strip_tool_call_text(text: str) -> str:
        """Remove raw tool call artifacts from final output."""
        # Remove Action: ... Action Input: ... blocks
        text = re.sub(
            r"Action:\s*.+?(?:Action Input:\s*.+?)?(?=\n\n|\Z)",
            "",
            text,
            flags=re.DOTALL | re.IGNORECASE,
        )
        # Remove <tool_call>...</tool_call> or </tool_name> blocks
        text = re.sub(r"<tool_call>.*?</\w+>", "", text, flags=re.DOTALL)
        return text.strip()

    @staticmethod
    def _tool_observation_text(tool_result: ToolResult) -> str:
        """Return compact text to feed back into the model after a tool call.

        Instruction-only skills often return a whole ``SKILL.md``. In
        Codex-first mode that playbook is already injected into the system
        prompt, so repeating it as an observation can overflow small local
        GGUF context windows before the next turn.
        """
        metadata = tool_result.metadata or {}
        if (
            metadata.get("skill_kind") == "instructional"
            or str(tool_result.tool_name).startswith("skill_")
            and metadata.get("steps", 0) == 0
        ):
            skill_name = metadata.get("skill") or tool_result.tool_name.replace(
                "skill_",
                "",
                1,
            )
            task = ""
            args = metadata.get("arguments")
            if isinstance(args, dict):
                task = str(args.get("task") or "").strip()
            suffix = f" Task: {task}" if task else ""
            search_hint = ""
            if skill_name == "arxiv":
                search_query = task or "academic research papers"
                search_hint = (
                    " Next, call exactly one academic search tool first, "
                    "preferably OpenAlex because it has broad scholarly "
                    "coverage and no API key requirement:\n"
                    "Action: openalex_search\n"
                    f"Action Input: {{\"query\": \"{search_query}\", "
                    "\"limit\": 5, \"start_year\": 2024, "
                    "\"end_year\": 2026}}\n"
                    "Use arxiv_search as fallback if OpenAlex finds nothing. "
                    "Use generic web_search only as the last fallback. "
                    "After searching, summarize 5 papers with years and links."
                )
            return (
                f"Skill `{skill_name}` is active. Use its playbook already "
                f"provided in the system prompt, then continue with the "
                f"appropriate concrete tools.{suffix}{search_hint}"
            )

        obs_text = tool_result.content
        if tool_result.tool_name in {
            "web_search",
            "openalex_search",
            "semantic_scholar_search",
            "arxiv_search",
        }:
            if len(obs_text) > 1200:
                obs_text = obs_text[:1200] + "\n\n[Search results truncated]"
            return (
                f"{obs_text}\n\n"
                "Now answer the user directly in Thai. Do not describe your "
                "reasoning process or mention what you plan to do. Include "
                "only useful results, dates, and links from the search output. "
                "If results are weak, say that clearly and suggest better "
                "academic search terms."
            )
        if len(obs_text) > 2000:
            obs_text = obs_text[:2000] + "\n\n[Output truncated]"
        return obs_text

    def _extract_code(self, text: str) -> str | None:
        """Extract Python code from markdown code blocks."""
        match = re.search(r"```python\n(.*?)```", text, re.DOTALL)
        if match:
            return match.group(1).strip()
        return None

    def _extract_tool_call(self, text: str) -> tuple[str, str] | None:
        """Extract tool call from structured output.

        Supports two formats:
        1. Action: tool_name / Action Input: {"key": "value"}
        2. <tool_call>tool_name\\n$key=value</tool_call> (XML-style)
        """
        # Format 1: Action / Action Input
        action_match = re.search(r"Action:\s*(.+)", text, re.IGNORECASE)
        input_match = re.search(
            r"Action Input:\s*(.+?)(?=\n\n|\Z)", text, re.DOTALL | re.IGNORECASE
        )
        if action_match:
            return (
                action_match.group(1).strip(),
                input_match.group(1).strip() if input_match else "{}",
            )

        # Format 2: <tool_call>tool_name ... </tool_call> or </tool_name>
        xml_match = re.search(
            r"<tool_call>\s*(\w+)\s*(.*?)</\w+>",
            text,
            re.DOTALL,
        )
        if xml_match:
            tool_name = xml_match.group(1).strip()
            raw_params = xml_match.group(2).strip()
            # Parse $key=value or <key>value</key> params into JSON
            params: dict[str, Any] = {}
            # $key=value format
            pat = r"\$(\w+)=(.+?)(?=\$|\n<|</|$)"
            for m in re.finditer(pat, raw_params, re.DOTALL):
                params[m.group(1)] = m.group(2).strip().rstrip("</>\n")
            # <key>value</key> format
            for m in re.finditer(r"<(\w+)>(.*?)</\1>", raw_params, re.DOTALL):
                key, val = m.group(1), m.group(2).strip()
                # Try to parse as int
                try:
                    params[key] = int(val)
                except ValueError:
                    params[key] = val
            # key: value format (common in GLM models)
            if not params:
                for m in re.finditer(
                    r"(\w+)\s*:\s*(.+?)(?=\n\w+\s*:|$)", raw_params, re.DOTALL
                ):
                    key, val = m.group(1), m.group(2).strip().strip("\"'")
                    try:
                        params[key] = int(val)
                    except ValueError:
                        params[key] = val
            if params:
                return (tool_name, _json.dumps(params))
            return (tool_name, "{}")

        return None

    def _supports_native_tool_schema(self) -> bool:
        """Return whether this backend should receive OpenAI tool schemas.

        llama.cpp accepts parts of the OpenAI-compatible API, but local GGUF
        models frequently return 400 for large or unsupported `tools` schemas.
        NativeOpenHands already includes text tool instructions in the prompt,
        so local llama.cpp should use the Action/Action Input fallback instead.
        """
        engine_id = getattr(self._engine, "engine_id", "")
        inner = self._engine
        while hasattr(inner, "_inner"):
            inner = getattr(inner, "_inner")
            engine_id = getattr(inner, "engine_id", engine_id)
        if engine_id in {"llamacpp", "mlx", "lmstudio"}:
            return False
        if str(self._model).lower().endswith(".gguf"):
            return False
        return True

    def _is_local_gguf_backend(self) -> bool:
        return not self._supports_native_tool_schema()

    def _select_prompt_tools(self, playbooks: list[dict[str, str]]) -> list[BaseTool]:
        """Keep local prompts small by listing only useful skill tools.

        The full installed skill catalog can be dozens or hundreds of entries.
        Local GGUF models with small context windows can reject that prompt with
        HTTP 400 before generation starts. Codex-first mode already injects the
        selected SKILL.md playbooks, so we only need to list normal tools plus
        skill tools matching those selected playbooks.
        """
        if not self._is_local_gguf_backend():
            return self._tools

        selected_skill_tools = {
            f"skill_{skill.get('name', '').strip()}"
            for skill in playbooks
            if skill.get("name")
        }
        prompt_tools: list[BaseTool] = []
        for tool in self._tools:
            spec = tool.spec
            is_skill = spec.category == "skill" or spec.name.startswith("skill_")
            if not is_skill or spec.name in selected_skill_tools:
                prompt_tools.append(tool)
        return prompt_tools

    def run(
        self,
        input: str,
        context: Optional[AgentContext] = None,
        **kwargs: Any,
    ) -> AgentResult:
        self._emit_turn_start(input)

        playbooks = list(self._skill_playbooks)
        prompt_tools = self._select_prompt_tools(playbooks)
        tool_descriptions = build_tool_descriptions(prompt_tools)
        if self._skill_few_shot_examples:
            skill_examples_block = (
                "## Skill Examples\n\n"
                + "\n\n".join(self._skill_few_shot_examples)
                + "\n\n"
            )
        else:
            skill_examples_block = ""
        prompt_template = (
            load_system_prompt_override("native_openhands") or OPENHANDS_SYSTEM_PROMPT
        )
        try:
            system_prompt = prompt_template.format(
                tool_descriptions=tool_descriptions,
                skill_examples=skill_examples_block,
            )
        except KeyError:
            system_prompt = prompt_template.format(
                tool_descriptions=tool_descriptions,
            )
            if skill_examples_block:
                system_prompt += "\n\n" + skill_examples_block

        skill_hints = self._build_relevant_skill_hints(input)
        if skill_hints:
            system_prompt = system_prompt + "\n\n" + skill_hints
        skill_playbooks = self._build_skill_playbooks_block()
        if skill_playbooks:
            system_prompt = system_prompt + "\n\n" + skill_playbooks

        # Pre-fetch any URLs in the input so the LLM gets the content directly
        input, url_expanded = self._expand_urls(input)

        # If URL content was inlined, skip the tool loop -- just summarize directly
        if url_expanded:
            direct_messages: list[Message] = [
                Message(
                    role=Role.SYSTEM,
                    content=(
                        "You are a helpful assistant. "
                        "Respond directly to the user's "
                        "request using the provided content."
                        " Do NOT include <think> tags."
                    ),
                ),
                Message(role=Role.USER, content=input),
            ]
            direct_messages = self._truncate_if_needed(direct_messages)
            try:
                result = self._generate(direct_messages)
            except Exception:
                # Propagate to the eval runner / server bridge so the failure
                # is recorded as an error instead of a fake "input too long"
                # answer that silently scores as 0%. Telemetry boundary is
                # still emitted before re-raising.
                self._emit_turn_end(turns=1, error=True)
                raise
            content = self._strip_think_tags(result.get("content", ""))
            usage = result.get("usage", {})
            self._emit_turn_end(turns=1)
            return AgentResult(
                content=content,
                tool_results=[],
                turns=1,
                metadata={
                    "prompt_tokens": usage.get("prompt_tokens", 0),
                    "completion_tokens": usage.get("completion_tokens", 0),
                    "total_tokens": usage.get("total_tokens", 0),
                },
            )

        messages = self._build_messages(input, context, system_prompt=system_prompt)

        # Inject few-shot exemplars before the user input
        for ex in load_few_shot_exemplars("native_openhands"):
            if ex.get("input") and ex.get("output"):
                messages.insert(-1, Message(role=Role.USER, content=ex["input"]))
                messages.insert(-1, Message(role=Role.ASSISTANT, content=ex["output"]))

        messages = self._truncate_if_needed(messages)

        all_tool_results: list[ToolResult] = []
        turns = 0
        last_content = ""
        total_usage = {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}

        # Build OpenAI-format tool schemas for native function calling
        openai_tools = (
            self._executor.get_openai_tools()
            if self._tools and self._supports_native_tool_schema()
            else []
        )
        # Side dict for Gemini thought_signatures (ToolCall uses slots)
        _thought_sigs: dict[str, bytes] = {}

        for _turn in range(self._max_turns):
            turns += 1
            # Truncate before every generate call -- tool results may have
            # expanded the context beyond what the model supports.
            messages = self._truncate_if_needed(messages)

            gen_kwargs: dict[str, Any] = {}
            if openai_tools:
                gen_kwargs["tools"] = openai_tools

            try:
                result = self._generate(messages, **gen_kwargs)
            except Exception:
                # Propagate so the eval runner records a real error rather
                # than a fake "input too long" string that silently scores 0.
                self._emit_turn_end(turns=turns, error=True)
                raise

            # Accumulate usage from this generate call
            usage = result.get("usage", {})
            for k in total_usage:
                total_usage[k] += usage.get(k, 0)

            content = result.get("content", "")
            # Strip think tags so they don't interfere with parsing
            content = self._strip_think_tags(content)
            last_content = content

            # --- Native function-calling path (OpenAI, Anthropic, etc.) ---
            raw_tool_calls = result.get("tool_calls", [])
            if raw_tool_calls:
                native_calls = []
                for i, tc in enumerate(raw_tool_calls):
                    call = ToolCall(
                        id=tc.get("id", f"call_{turns}_{i}"),
                        name=tc.get("name", ""),
                        arguments=tc.get("arguments", "{}"),
                    )
                    # Preserve thought_signature for Gemini reasoning
                    sig = tc.get("thought_signature")
                    if sig is not None:
                        _thought_sigs[call.id] = sig
                    native_calls.append(call)
                messages.append(
                    Message(
                        role=Role.ASSISTANT,
                        content=content,
                        tool_calls=native_calls,
                    )
                )
                for tc in native_calls:
                    tool_result = self._executor.execute(tc)
                    all_tool_results.append(tool_result)
                    obs_text = self._tool_observation_text(tool_result)
                    messages.append(
                        Message(
                            role=Role.TOOL,
                            content=obs_text,
                            tool_call_id=tc.id,
                            name=tc.name,
                        )
                    )
                continue

            # --- Text-based fallback (CodeAct / Action-Input format) ---

            # Try to extract code
            code = self._extract_code(content)
            if code:
                messages.append(Message(role=Role.ASSISTANT, content=content))

                # Execute via code_interpreter tool if available
                tool_call = ToolCall(
                    id=f"code_{turns}",
                    name="code_interpreter",
                    arguments=_json.dumps({"code": code}),
                )
                tool_result = self._executor.execute(tool_call)
                all_tool_results.append(tool_result)

                obs_text = self._tool_observation_text(tool_result)
                observation = f"Output:\n{obs_text}"
                messages.append(Message(role=Role.USER, content=observation))
                continue

            # Try tool call
            tool_info = self._extract_tool_call(content)
            if tool_info:
                action, action_input = tool_info
                messages.append(Message(role=Role.ASSISTANT, content=content))

                tool_call = ToolCall(
                    id=f"tool_{turns}", name=action, arguments=action_input
                )
                tool_result = self._executor.execute(tool_call)
                all_tool_results.append(tool_result)

                obs_text = self._tool_observation_text(tool_result)
                observation = f"Result: {obs_text}"
                messages.append(Message(role=Role.USER, content=observation))
                continue

            # No code or tool call -- this is the final answer
            content = self._strip_think_tags(content)
            content = self._strip_tool_call_text(content)
            self._emit_turn_end(turns=turns)
            return AgentResult(
                content=content,
                tool_results=all_tool_results,
                turns=turns,
                metadata=total_usage,
            )

        # Max turns
        final = self._strip_think_tags(last_content) or "Maximum turns reached."
        final = self._strip_tool_call_text(final)
        result = self._max_turns_result(all_tool_results, turns, content=final)
        result.metadata.update(total_usage)
        return result


__all__ = ["NativeOpenHandsAgent"]
