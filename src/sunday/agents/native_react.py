"""NativeReActAgent -- Thought-Action-Observation loop agent.

Renamed from ``ReActAgent`` to clarify this is SUNDAY's native
implementation, not an integration with an external project.
"""

from __future__ import annotations

import re
from typing import Any, List, Optional

from sunday.agents._stubs import AgentContext, AgentResult, ToolUsingAgent
from sunday.agents.prompt_loader import (
    load_few_shot_exemplars,
    load_system_prompt_override,
)
from sunday.core.events import EventBus
from sunday.core.registry import AgentRegistry
from sunday.core.types import Message, Role, ToolCall, ToolResult, _message_to_dict
from sunday.engine._stubs import InferenceEngine
from sunday.tools._stubs import BaseTool, build_tool_descriptions

REACT_SYSTEM_PROMPT = """\
You are a ReAct agent. For each step, respond with exactly one of:

1. To think and act:
Thought: <your reasoning>
Action: <tool_name>
Action Input: <json arguments>

2. To give a final answer:
Thought: <your reasoning>
Final Answer: <your answer>

# Using Skills

Tools whose names begin with `skill_` are SKILLS. When you call a skill tool,
the response can take one of two forms:

- **Computed result**: The skill ran a deterministic pipeline and returned a
  value (number, string, JSON, etc.). Use the value directly in your answer.

- **Procedural instructions**: The skill returned markdown text describing
  HOW to accomplish a task. Recognize this when the response starts with
  `#` headings, contains bullet lists, or uses phrases like "When asked
  to...", "First...", "Steps:". When you receive instructions:
  1. READ the instructions carefully — they are your playbook
  2. FOLLOW the steps using your OTHER tools (e.g. calculator, web_search,
     shell_exec, file_read) — not the same skill
  3. DO NOT call the same skill again — you already have its instructions
  4. Synthesize a Final Answer from what you learned

{skill_examples}{tool_descriptions}"""


@AgentRegistry.register("native_react")
class NativeReActAgent(ToolUsingAgent):
    """ReAct agent: Thought -> Action -> Observation loop."""

    agent_id = "native_react"
    _default_temperature = 0.7
    _default_max_tokens = 1024
    _default_max_turns = 10

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

    def _parse_response(self, text: str) -> dict:
        """Parse ReAct structured output."""
        result = {"thought": "", "action": "", "action_input": "", "final_answer": ""}

        # Extract Thought
        thought_match = re.search(
            r"Thought:\s*(.+?)(?=\nAction:|\nFinal Answer:|\Z)",
            text,
            re.DOTALL | re.IGNORECASE,
        )
        if thought_match:
            result["thought"] = thought_match.group(1).strip()

        # Check for Final Answer
        final_match = re.search(
            r"Final Answer:\s*(.+)", text, re.DOTALL | re.IGNORECASE
        )
        if final_match:
            result["final_answer"] = final_match.group(1).strip()
            return result

        # Extract Action and Action Input
        action_match = re.search(r"Action:\s*(.+)", text, re.IGNORECASE)
        if action_match:
            result["action"] = action_match.group(1).strip()

        input_match = re.search(
            r"Action Input:\s*(.+?)(?=\n\n|\nThought:|\Z)",
            text,
            re.DOTALL | re.IGNORECASE,
        )
        if input_match:
            result["action_input"] = input_match.group(1).strip()

        return result

    def run(
        self,
        input: str,
        context: Optional[AgentContext] = None,
        **kwargs: Any,
    ) -> AgentResult:
        self._emit_turn_start(input)

        try:
            from sunday._rust_bridge import get_rust_module
            rust_mod = get_rust_module()
            
            # Since the rust NativeReActAgent uses its own system prompt and tools,
            # we just instantiate and run it here.
            # We map the engine key (e.g. 'ollama' or 'llama_cpp') based on the Python engine type.
            engine_key = getattr(self._engine, "engine_id", "ollama")
            
            agent = rust_mod.NativeReActAgent(
                engine_key=engine_key,
                host="http://localhost:11434",
                model=self._model,
                max_turns=self._max_turns,
                temperature=self._temperature,
            )
            
            # Run the rust agent
            result = agent.run(input)
            
            self._emit_turn_end(turns=result.turns)
            return AgentResult(
                content=result.content,
                tool_results=[],  # Tool results are managed by Rust internally for now
                turns=result.turns,
                metadata={"rust_native": True},
            )

        except Exception as e:
            # Fallback to a failure message if rust agent crashes or is not available
            self._emit_turn_end(turns=1)
            return AgentResult(
                content=f"Error running native Rust ReAct agent: {e}",
                tool_results=[],
                turns=1,
                metadata={"error": str(e)},
            )

__all__ = ["NativeReActAgent", "REACT_SYSTEM_PROMPT"]
