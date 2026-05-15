"""OrchestratorAgent — multi-turn agent with tool-calling loop.

Supports two modes:

- **function_calling** (default): Uses OpenAI-format tool definitions and
  parses ``tool_calls`` from the engine response.
- **structured**: Uses a THOUGHT/TOOL/INPUT/FINAL_ANSWER text format
  (like ReAct) with a canonical system prompt from the orchestrator
  prompt registry.  This is the format used by the SFT/GRPO training
  pipelines, making the Orchestrator a distinctive trainable agent type.
"""

from __future__ import annotations

import concurrent.futures
import hashlib
import re
from typing import Any, Dict, List, Optional

from sunday.agents._stubs import AgentContext, AgentResult, ToolUsingAgent
from sunday.core.events import EventBus
from sunday.core.registry import AgentRegistry
from sunday.core.types import Message, Role, ToolCall, ToolResult
from sunday.engine._stubs import InferenceEngine
from sunday.tools._stubs import BaseTool


@AgentRegistry.register("orchestrator")
class OrchestratorAgent(ToolUsingAgent):
    """Multi-turn agent that routes between tools and the LLM.

    Implements a tool-calling loop:
    1. Send messages with tool definitions to the engine.
    2. If the response contains tool_calls, execute them and loop.
    3. If no tool_calls, return the final answer.
    4. Stop after ``max_turns`` iterations.

    In **structured** mode the agent instead uses a
    ``THOUGHT: / TOOL: / INPUT: / FINAL_ANSWER:`` text protocol
    identical to the format used by the orchestrator SFT/GRPO
    training pipelines.
    """

    agent_id = "orchestrator"
    _default_temperature = 0.7
    _default_max_tokens = 1024
    _default_max_turns = 10

    # --- State-Change Tracker (detects browser loops via URL+content hash) ---
    class _StateTracker:
        """Track browser state changes to detect loops more precisely than tool-name counting."""
        def __init__(self):
            self.states: List[str] = []  # list of "url|content_hash"

        def record(self, tool_name: str, tool_result_content: str):
            """Record a state fingerprint after a browser tool execution."""
            if not tool_name.startswith("browser_"):
                return
            # Extract URL if present in result
            url = ""
            for line in (tool_result_content or "").split("\n")[:5]:
                if "http" in line:
                    url = line.strip()[:120]
                    break
            content_hash = hashlib.md5((tool_result_content or "")[:2000].encode()).hexdigest()[:8]
            self.states.append(f"{url}|{content_hash}")

        def is_stuck(self, window: int = 3) -> bool:
            """True if the last N browser states are identical (same page, same content)."""
            if len(self.states) < window:
                return False
            recent = self.states[-window:]
            return len(set(recent)) == 1

        def reset(self):
            self.states.clear()

    _visual_audit_prompt = (
        "You are a STRICT VISUAL VERIFIER. You are comparing a Proposed Answer to a Screenshot.\n\n"
        "CRITERIA for REJECTION:\n"
        "1. The page shows '404 Not Found', 'Page Not Found', or a blank screen.\n"
        "2. The Proposed Answer mentions a specific PRICE or MODEL that is NOT visible in the screenshot.\n"
        "3. The page is a generic homepage and does not contain the specific information requested.\n\n"
        "OUTPUT FORMAT:\n"
        "REJECT: [Brief reason]\n"
        "ACCEPT: [Brief reason]\n\n"
        "Decision (REJECT or ACCEPT only):"
    )

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
        mode: str = "function_calling",
        system_prompt: Optional[str] = None,
        parallel_tools: bool = True,
        interactive: bool = False,
        confirm_callback=None,
        provider: Optional[str] = None,
        visual_audit: bool = True,
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
        )
        self._mode = mode
        self._system_prompt = system_prompt
        self._parallel_tools = parallel_tools
        self._visual_audit = visual_audit
        self._provider = provider

    def run(
        self,
        input: str,
        context: Optional[AgentContext] = None,
        **kwargs: Any,
    ) -> AgentResult:
        if self._mode == "structured":
            return self._run_structured(input, context, **kwargs)
        return self._run_function_calling(input, context, **kwargs)

    @property
    def _config(self):
        from sunday.core.config import load_config
        return load_config()

    def _generate(self, messages: List[Message], **kwargs: Any) -> tuple[dict, str]:
        """Centralized generation with Smart Hybrid Routing."""
        # Use explicit provider if set, else fallback to config
        provider = self._provider or self._config.intelligence.provider
        
        # 🛡️ STRICT PROTOCOL: Force the model to be concise and hide internal monologues
        protocol_fix = (
            "\n\n[STRICT PROTOCOL]\n"
            "1. IDENTITY OVERRIDE: You are SUNDAY, an autonomous high-performance agent. You HAVE direct access to the local machine and the internet via your TOOLS.\n"
            "2. NEVER say you cannot run tools, cannot browse, or cannot access the system. You MUST use your tools to fulfill requests.\n"
            "3. If you need to think, use <thought> tags. NEVER leak your reasoning process outside these tags.\n"
            "4. Your final response must be ONLY the answer or tool result. No meta-commentary.\n"
            "5. If a test is requested, call 'run_harness_test' immediately. DO NOT explain why you are doing it."
        )
        if messages and messages[0].role == Role.SYSTEM:
            messages[0].content += protocol_fix

        # Determine the correct brain
        if provider == "hybrid":
            # 🧠 SMART ROUTER: 3-tier task classification
            task_tier = self._classify_task(messages)
            
            if task_tier == "cloud":
                use_local = False
                print(f"[🧠 HYBRID] Task requires tools/reasoning. Routing to CLOUD...")
            else:
                use_local = True
                print(f"[⚡ HYBRID] Simple/summarization task. Routing to LOCAL...")

            if use_local:
                # ⚡ LOCAL BRAIN: Fast, free, good for simple chat/extraction
                current_provider = "local"
                model = self._model
                brain_tag = "[⚡ LOCAL]"
            else:
                # 🧠 CLOUD BRAIN: Powerful, used for navigation/reasoning
                current_provider = "openrouter"
                model = self._config.intelligence.default_model
                brain_tag = "[🧠 CLOUD]"
        else:
            current_provider = provider
            # 🛡️ FORCE LOCAL: If provider is local, we MUST use the fallback GGUF model
            if provider == "local":
                model = self._config.intelligence.fallback_model
                brain_tag = "[⚡ LOCAL]"
            else:
                model = self._model
                brain_tag = f"[{provider.upper()}]"

        # 🔑 API KEY RESOLVER: Prioritize .env if config is empty
        import os
        api_key = os.getenv("OPENROUTER_API_KEY")
        if api_key and current_provider == "openrouter":
            print(f"[HYBRID] Routing to Cloud via OpenRouter...")

        try:
            res = self._engine.generate(
                messages,
                model=model,
                temperature=kwargs.pop("temperature", self._temperature),
                max_tokens=kwargs.pop("max_tokens", self._max_tokens),
                provider=current_provider,
                api_key=api_key,
                timeout=kwargs.pop("timeout", 300.0), # ⏱️ Higher timeout for local inference
                **kwargs
            )
        except Exception as e:
            # Re-raise to trigger the resilient fallback in the run loop
            raise e
            
        return res, brain_tag

    def _inject_brain_tag(self, arguments: str, brain_tag: str) -> str:
        """Inject brain source tag into tool arguments for UI observability."""
        import json
        try:
            args_obj = json.loads(arguments)
            args_obj["_brain"] = brain_tag
            return json.dumps(args_obj)
        except (json.JSONDecodeError, TypeError):
            return arguments

    def _classify_task(self, messages: List[Message]) -> str:
        """Classify task complexity for hybrid routing.
        
        Returns 'cloud' or 'local' based on a 3-tier heuristic:
        1. If data is already retrieved → local (summarization)
        2. If task requires tools (browsing/searching) → cloud (reasoning)
        3. Simple chat → local (fast response)
        """
        # 🛡️ SAFE JOIN: Handle messages with None content (e.g. assistant tool calls)
        full_text = " ".join([m.content or "" for m in messages]).lower()
        last_msg = messages[-1].content.lower() if (messages and messages[-1].content) else ""
        
        # Tier 1: If we already have data, use Local for summarization
        if ">>> data retrieved" in full_text:
            for msg in reversed(messages):
                if msg.role == Role.USER and msg.content.startswith("Observation:"):
                    if len(msg.content) > 500:
                        return "local"  # Summarize locally
                    break
        
        # Tier 2: Tool-required tasks need Cloud reasoning
        tool_keywords = [
            "โรงแรม", "hotel", "booking", "จอง", "ค้นหา", "ราคา", "price",
            "research", "paper", "amazon", "browse", "เปิดเว็บ",
            "browser", "เว็บ", "web_search", "search",
        ]
        if any(k in full_text for k in tool_keywords):
            return "cloud"
        
        # Tier 3: Simple chat stays Local
        if len(last_msg) < 100:
            return "local"
        
        # Default: Cloud (better safe than sorry)
        return "cloud"

    # ------------------------------------------------------------------
    # Structured mode (THOUGHT/TOOL/INPUT/FINAL_ANSWER)
    # ------------------------------------------------------------------

    def _run_structured(
        self,
        input: str,
        context: Optional[AgentContext] = None,
        **kwargs: Any,
    ) -> AgentResult:
        self._emit_turn_start(input)

        # Build system prompt (Optimized with Heuristic Filtering)
        relevant_names = self._get_relevant_tools(input)
        filtered_tools = [t for t in self._tools if t.spec.name in relevant_names] if (self._tools and relevant_names) else self._tools

        if self._system_prompt:
            sys_prompt = self._system_prompt
        else:
            from sunday.learning.intelligence.orchestrator.prompt_registry import (
                build_system_prompt,
            )

            sys_prompt = build_system_prompt(tools=filtered_tools)

        messages = self._build_messages(input, context, system_prompt=sys_prompt)
        
        # --- SMART HARNESS: MEMORY RECALL ---
        if self._config.agent.context_from_memory:
            try:
                from sunday.agents._lesson_store import get_lesson_store
                store = get_lesson_store()
                past_lessons = store.search(input, top_k=2)
                if past_lessons:
                    lesson_text = "\n\n[MEMORY RECALL] Lessons from past similar tasks:\n" + "\n".join([f"- {l.content}" for l in past_lessons])
                    messages[0].content += lesson_text
            except Exception as e:
                print(f"[WARN] Memory recall failed: {e}")

        all_tool_results: list[ToolResult] = []
        turns = 0
        tool_usage_history = [] # Track tool calls for loop detection
        state_tracker = self._StateTracker()  # 🥉 State-Change Tracking

        for _turn in range(self._max_turns):
            turns += 1

            # --- SMART HARNESS: LOOP INTUITION ---
            if len(tool_usage_history) >= 3 and len(set(tool_usage_history[-3:])) == 1:
                messages.append(Message(role=Role.USER, content="[SYSTEM WARNING] You have used the same tool 3 times. You might be stuck. Change your strategy (e.g., try a different URL, use web_search, or ask the user for help)."))
                tool_usage_history = [] # Reset after warning

            if self._loop_guard:
                messages = self._loop_guard.compress_context(messages)

            # --- LOCAL OPTIMIZATION: Context Management ---
            # For Local models, we cap at 2048 tokens to keep prefill fast
            limit = 2048 if self._config.intelligence.provider == "local" else 8192
            messages = self._context_manager.optimize(messages, max_tokens=limit)

            try:
                result, brain_tag = self._generate(messages)
            except Exception as e:
                # 🛡️ RESILIENT FALLBACK: If Cloud fails (connection error), try Local!
                print(f"[🛡️ RESILIENT] Cloud generation failed: {e}. Falling back to LOCAL brain...")
                old_provider = self._config.intelligence.provider
                self._config.intelligence.provider = "local"
                try:
                    result, brain_tag = self._generate(messages)
                    brain_tag = "[🛡️ FALLBACK LOCAL]"
                finally:
                    self._config.intelligence.provider = old_provider
            
            content = result.get("content", "")
            parsed = self._parse_structured_response(content)

            # FINAL_ANSWER -> done
            # --- PLAN EXTRACTION ---
            # Extract [ ] or [x] tasks from THOUGHT to show a visual timeline
            plan_match = _re.findall(r'\[([ xX])\]\s*(.+)', result)
            if plan_match:
                plan_items = [{"task": task.strip(), "done": status.lower() == 'x'} for status, task in plan_match]
                # Emit plan as metadata for the UI timeline
                self._emit_metadata({"execution_plan": plan_items})

            if parsed["final_answer"]:
                self._emit_turn_end(turns=turns)
                return AgentResult(
                    content=parsed["final_answer"],
                    tool_results=all_tool_results,
                    turns=turns,
                )

            # TOOL -> execute
            if parsed["tool"]:
                tool_usage_history.append(parsed["tool"])
                messages.append(Message(role=Role.ASSISTANT, content=content))

                # Clean arguments (use raw parsed input, no _brain injection)
                raw_args = parsed["input"] or "{}"

                tool_call = ToolCall(
                    id=f"orch_{turns}",
                    name=parsed["tool"],
                    arguments=raw_args,
                )

                # === INTERCEPT: If model tries to scroll/click on a results page, force extract instead ===
                if tool_call.name in ("browser_scroll", "browser_click") and turns > 5:
                    last_msg = messages[-1].content if messages else ""
                    if "properties found" in last_msg.lower() or "Search results" in last_msg:
                         tool_call.name = "browser_extract"
                         tool_call.arguments = '{"selector": "body", "extract_type": "text"}'

                # Emit tool call event for frontend dashboard + console log
                self._emit_tool_call(tool_call)
                print(f"[🧠 {brain_tag}] Executing: {tool_call.name}")
                tool_result = self._executor.execute(tool_call)
                
                # 🥉 State-Change Tracking: record browser state
                state_tracker.record(tool_call.name, tool_result.content or "")
                if state_tracker.is_stuck():
                    messages.append(Message(role=Role.USER, content=
                        "[🔴 STATE-STUCK DETECTED] The browser page has not changed after 3 actions. "
                        "The URL and content are identical. You MUST try a completely different approach: "
                        "use web_search, change the URL, or provide a FINAL_ANSWER with what you have."
                    ))
                    state_tracker.reset()

                # 🥈 Reflection Pattern: force AI to analyze failures
                if not tool_result.success:
                    reflection = self._reflection_prompt(tool_call.name, tool_result.content)
                    tool_result.content = reflection
                
                all_tool_results.append(tool_result)

                # === AUTO-COMPLETE: If browser_extract returned listing data, finish now ===
                auto_answer = self._check_auto_complete(tool_result, input)
                if auto_answer:
                    self._emit_turn_end(turns=turns)
                    return AgentResult(
                        content=auto_answer,
                        tool_results=all_tool_results,
                        turns=turns,
                    )

                observation = f"Observation: {tool_result.content}"
                messages.append(Message(role=Role.USER, content=observation))
                
                # Auto-compress previous large observations to save memory
                messages = self._context_manager.compress_tool_outputs(messages)
                continue

            # Neither -> treat content as final answer (but strip thinking)
            final_content = self._strip_think_tags(content)
            self._emit_turn_end(turns=turns)
            result = AgentResult(
                content=final_content,
                tool_results=all_tool_results,
                turns=turns,
            )
            # 🏅 Memory Consolidation: save lesson after completion
            self._consolidate_memory(input, result)
            return result

        # Max turns exceeded
        return self._max_turns_result(all_tool_results, turns)

    @staticmethod
    def _parse_structured_response(text: str) -> dict:
        """Parse THOUGHT/TOOL/INPUT/FINAL_ANSWER from model output (supports standard and XML)."""
        result = {
            "thought": "",
            "tool": "",
            "input": "",
            "final_answer": "",
        }

        # 1. Extract and separate THOUGHT/Thinking block
        thought_match = re.search(
            r"(?:THOUGHT|Thinking Process|Reasoning|<thought>):\s*(.+?)(?=\nTOOL:|\nFINAL[_ ]?ANSWER:|</thought>|\Z)",
            text,
            re.DOTALL | re.IGNORECASE,
        )
        if thought_match:
            result["thought"] = thought_match.group(1).strip()
            search_text = text.replace(thought_match.group(0), "")
        else:
            search_text = text

        # 2. FINAL_ANSWER check (priority)
        final_match = re.search(
            r"FINAL[_ ]?ANSWER:\s*(.+)",
            search_text,
            re.DOTALL | re.IGNORECASE,
        )
        if final_match:
            result["final_answer"] = final_match.group(1).strip()
            return result

        # 3. XML Tool Call Format (OpenAI/Claude style fallback)
        xml_match = re.search(r"<tool_call>\s*(\{.*?\})\s*</tool_call>", search_text, re.DOTALL)
        if xml_match:
            try:
                import json
                tc = json.loads(xml_match.group(1))
                result["tool"] = tc.get("name", "")
                result["input"] = json.dumps(tc.get("arguments", {}))
                return result
            except: pass

        # 4. Standard Inline Format TOOL: name({"key": "val"})
        inline_match = re.search(
            r"TOOL:\s*([\w_]+)\s*\((.+?)\)(?=\n||\Z)", 
            search_text, 
            re.DOTALL | re.IGNORECASE
        )
        if inline_match:
            result["tool"] = inline_match.group(1).strip()
            result["input"] = inline_match.group(2).strip()
            return result

        # 5. Standard Format TOOL: name \n INPUT: json
        tool_match = re.search(r"TOOL:\s*([\w_]+)", search_text, re.IGNORECASE)
        if tool_match:
            result["tool"] = tool_match.group(1).strip()

        input_match = re.search(
            r"INPUT:\s*(.+?)(?=\n(?:THOUGHT|Thinking Process|Reasoning):|\nTOOL:|\nFINAL|\Z)",
            search_text,
            re.DOTALL | re.IGNORECASE,
        )
        if input_match:
            result["input"] = input_match.group(1).strip()

        # Validate: If tool name is empty but input is not, it's an invalid parse
        if not result["tool"] and result["input"]:
            result["input"] = "" 
            
        return result

    # ------------------------------------------------------------------
    # Function-calling mode (original behaviour)
    # ------------------------------------------------------------------

    def _run_function_calling(
        self,
        input: str,
        context: Optional[AgentContext] = None,
        **kwargs: Any,
    ) -> AgentResult:
        self._emit_turn_start(input)

        messages = self._build_messages(input, context)

        # Get OpenAI-format tool definitions (Optimized with Heuristic Filtering)
        relevant_tools = self._get_relevant_tools(input)
        openai_tools = self._executor.get_openai_tools(filter_names=relevant_tools) if self._tools else []

        all_tool_results: list[ToolResult] = []
        turns = 0
        total_prompt_tokens = 0
        total_completion_tokens = 0

        # 🥉 State-Change Tracker: detect browser stuck loops
        state_tracker = self._StateTracker()

        for _turn in range(self._max_turns):
            turns += 1

            if self._loop_guard:
                messages = self._loop_guard.compress_context(messages)

            # Build generate kwargs
            gen_kwargs: dict[str, Any] = {}
            if openai_tools:
                gen_kwargs["tools"] = openai_tools

            # --- LOCAL OPTIMIZATION: Context Management ---
            # Aggressive capping for local/Ollama to prevent 100s prefill times
            is_local = self._config.intelligence.provider == "local"
            limit = 2048 if is_local else 8192
            messages = self._context_manager.optimize(messages, max_tokens=limit)

            try:
                result, brain_tag = self._generate(messages, **gen_kwargs)
            except Exception as e:
                # 🛡️ RESILIENT FALLBACK: If Cloud fails, try Local!
                print(f"[🛡️ RESILIENT] Cloud failed: {e}. Falling back to LOCAL...")
                old_provider = self._config.intelligence.provider
                self._config.intelligence.provider = "local"
                try:
                    result, brain_tag = self._generate(messages, **gen_kwargs)
                    brain_tag = "[🛡️ FALLBACK LOCAL]"
                finally:
                    self._config.intelligence.provider = old_provider

            # Accumulate token usage
            usage = result.get("usage", {})
            total_prompt_tokens += usage.get("prompt_tokens", 0)
            total_completion_tokens += usage.get("completion_tokens", 0)

            content = result.get("content", "")
            raw_tool_calls = result.get("tool_calls", [])

            # No tool calls -> check continuation, then final answer
            if not raw_tool_calls:
                content = self._check_continuation(result, messages)
                content = self._strip_think_tags(content)

                # 👁️ Visual Grounding Pattern (Priority 1): Auditor check before final answer
                # Only run if the agent interacted with the browser during this session
                if self._visual_audit and any(tr.tool_name.startswith("browser_") for tr in all_tool_results):
                    audit_result = self._run_visual_audit(input, content)
                    if audit_result.startswith("REJECT:"):
                        print(f"[👁️ VISUAL REJECT] {audit_result}")
                        messages.append(
                            Message(
                                role=Role.USER,
                                content=f"[👁️ VISUAL AUDIT FAILED] {audit_result}\nYou MUST correct your answer based on the visual evidence. If the page was a 404, go back to a search engine.",
                            )
                        )
                        continue  # Re-plan!

                self._emit_turn_end(turns=turns, content_length=len(content))
                return AgentResult(
                    content=content,
                    tool_results=all_tool_results,
                    turns=turns,
                    metadata={
                        "prompt_tokens": total_prompt_tokens,
                        "completion_tokens": total_completion_tokens,
                        "total_tokens": total_prompt_tokens + total_completion_tokens,
                    },
                )

            # Build ToolCall objects from raw dicts (clean, no brain tag injection)
            tool_calls = [
                ToolCall(
                    id=tc.get("id", f"call_{i}"),
                    name=tc.get("name", ""),
                    arguments=tc.get("arguments", "{}"),
                )
                for i, tc in enumerate(raw_tool_calls)
            ]

            # Emit tool call events for frontend dashboard + console log
            for tc in tool_calls:
                self._emit_tool_call(tc)
                print(f"[🧠 {brain_tag}] FC Executing: {tc.name}")

            # Append assistant message with tool calls
            messages.append(
                Message(
                    role=Role.ASSISTANT,
                    content=content,
                    tool_calls=tool_calls,
                )
            )

            # 🥇 Actor-Critic Pattern: Verify plan before execution
            critic_rejection = self._run_critic_check(tool_calls, messages)
            if critic_rejection:
                print(f"[🛑 CRITIC REJECTED] {critic_rejection}")
                for tc in tool_calls:
                    tool_result = ToolResult(
                        tool_name=tc.name,
                        content=f"[CRITIC REJECTED] {critic_rejection}",
                        success=False,
                    )
                    all_tool_results.append(tool_result)
                    self._emit_tool_call_end(tc, tool_result, 0)
                    messages.append(
                        Message(
                            role=Role.TOOL,
                            content=tool_result.content,
                            tool_call_id=tc.id,
                            name=tc.name,
                        )
                    )
                continue  # Skip execution and let the LLM plan again

            # Execute each tool (with loop guard check) and append results
            # Note: We filter again here to ensure security/consistency if needed, 
            # but usually OpenAI-tools filtering above is enough.
            if self._parallel_tools and len(tool_calls) > 1:
                # Parallel execution
                def _exec_tool(tc: ToolCall) -> tuple:
                    import time
                    t0 = time.time()
                    if self._loop_guard:
                        verdict = self._loop_guard.check_call(
                            tc.name,
                            tc.arguments,
                        )
                        if verdict.blocked:
                            res = ToolResult(
                                tool_name=tc.name,
                                content=f"Loop guard: {verdict.reason}",
                                success=False,
                            )
                            return tc, res, (time.time() - t0) * 1000
                    res = self._executor.execute(tc)
                    return tc, res, (time.time() - t0) * 1000

                with concurrent.futures.ThreadPoolExecutor(
                    max_workers=len(tool_calls),
                ) as pool:
                    futures = {pool.submit(_exec_tool, tc): tc for tc in tool_calls}
                    results_map: dict[int, tuple] = {}
                    for future in concurrent.futures.as_completed(futures):
                        tc_orig = futures[future]
                        results_map[id(tc_orig)] = future.result()

                # Append results in original order
                for tc in tool_calls:
                    _, tool_result, latency = results_map[id(tc)]
                    self._emit_tool_call_end(tc, tool_result, latency)
                    all_tool_results.append(tool_result)
                    # 🔍 404 Detection: Prevent "guessing" loops
                    if tc.name == "browser_navigate" and any(x in tool_result.content.lower() for x in ["not found", "404", "ไม่พบหน้า"]):
                        tool_result = ToolResult(
                            tool_name=tc.name,
                            content=(
                                f"[⚠️ 404 ERROR] The page you navigated to returned NOT FOUND.\n"
                                f"Page Content: {tool_result.content[:200]}\n\n"
                                f"STOP guessing URLs. Your current guess is wrong. "
                                f"You MUST use 'web_search' to find the actual current URL for this product "
                                f"or go to the homepage and use the site's own search bar."
                            ),
                            success=False
                        )

                    # 🥉 State-Change Tracker: record browser state fingerprint
                    state_tracker.record(tc.name, tool_result.content)

                    # 🥈 Reflection Pattern: inject structured failure analysis
                    if not tool_result.success:
                        tool_result = ToolResult(
                            tool_name=tc.name,
                            content=self._reflection_prompt(tc.name, tool_result.content),
                            success=False,
                        )

                    messages.append(
                        Message(
                            role=Role.TOOL,
                            content=tool_result.content,
                            tool_call_id=tc.id,
                            name=tc.name,
                        )
                    )
            else:
                # Sequential execution
                for tc in tool_calls:
                    import time
                    t0 = time.time()
                    # Loop guard check before execution
                    if self._loop_guard:
                        verdict = self._loop_guard.check_call(
                            tc.name,
                            tc.arguments,
                        )
                        if verdict.blocked:
                            tool_result = ToolResult(
                                tool_name=tc.name,
                                content=f"Loop guard: {verdict.reason}",
                                success=False,
                            )
                            latency = (time.time() - t0) * 1000
                            self._emit_tool_call_end(tc, tool_result, latency)
                            all_tool_results.append(tool_result)
                            messages.append(
                                Message(
                                    role=Role.TOOL,
                                    content=tool_result.content,
                                    tool_call_id=tc.id,
                                    name=tc.name,
                                )
                            )
                            continue

                    tool_result = self._executor.execute(tc)
                    latency = (time.time() - t0) * 1000
                    self._emit_tool_call_end(tc, tool_result, latency)
                    all_tool_results.append(tool_result)

                    # 🔍 404 Detection: Prevent "guessing" loops
                    if tc.name == "browser_navigate" and any(x in tool_result.content.lower() for x in ["not found", "404", "ไม่พบหน้า"]):
                        tool_result = ToolResult(
                            tool_name=tc.name,
                            content=(
                                f"[⚠️ 404 ERROR] The page you navigated to returned NOT FOUND.\n"
                                f"Page Content: {tool_result.content[:200]}\n\n"
                                f"STOP guessing URLs. Your current guess is wrong. "
                                f"You MUST use 'web_search' to find the actual current URL for this product "
                                f"or go to the homepage and use the site's own search bar."
                            ),
                            success=False
                        )

                    # 🥉 State-Change Tracker: record browser state fingerprint
                    state_tracker.record(tc.name, tool_result.content)

                    # 🥈 Reflection Pattern: inject structured failure analysis
                    if not tool_result.success:
                        tool_result = ToolResult(
                            tool_name=tc.name,
                            content=self._reflection_prompt(tc.name, tool_result.content),
                            success=False,
                        )

                    # 🥉 Stuck Detection: if browser state hasn't changed in 3 turns, force new strategy
                    if state_tracker.is_stuck(window=3):
                        stuck_msg = (
                            "[⚠️ STUCK DETECTED] The browser has been on the same page with the same content "
                            "for 3 consecutive actions. You MUST try a completely different approach:\n"
                            "1. Use web_search to find a direct URL with filters already applied\n"
                            "2. Try browser_navigate to a completely different URL\n"
                            "3. If the site is unresponsive, summarize whatever data you already have\n\n"
                            "Do NOT repeat the same browser actions."
                        )
                        messages.append(Message(role=Role.USER, content=stuck_msg))
                        state_tracker.reset()  # Reset so it can detect the next stuck cycle
                        print(f"[⚠️ STATE] Stuck detected at turn {turns}, injecting recovery prompt")

                    # Append tool response message
                    messages.append(
                        Message(
                            role=Role.TOOL,
                            content=tool_result.content,
                            tool_call_id=tc.id,
                            name=tc.name,
                        )
                    )

        # Max turns exceeded
        final_content = self._strip_think_tags(content) if content else ""
        self._emit_turn_end(turns=turns)
        result = AgentResult(
            content=final_content or "Maximum turns reached without a final answer.",
            tool_results=all_tool_results,
            turns=turns,
            metadata={
                "max_turns_exceeded": True,
                "prompt_tokens": total_prompt_tokens,
                "completion_tokens": total_completion_tokens,
                "total_tokens": total_prompt_tokens + total_completion_tokens,
            },
        )
        # 🏅 Memory Consolidation: especially important for failed tasks
        self._consolidate_memory(input, result)
        return result
    

    # ------------------------------------------------------------------
    # 🥈 Reflection Pattern: structured failure analysis
    # ------------------------------------------------------------------
    @staticmethod
    def _reflection_prompt(tool_name: str, error_content: str) -> str:
        """Build a structured reflection prompt when a tool fails.

        Instead of a generic 'TRY AGAIN', this forces the model to
        analyze the failure and choose a different strategy.
        """
        return (
            f"[🛑 REFLECTION REQUIRED] Tool '{tool_name}' failed.\n"
            f"Error: {(error_content or 'Unknown error')[:300]}\n\n"
            f"Before you are allowed to try again, you MUST use the `think` tool to write a failure log. "
            f"In your `think` tool call, you MUST answer:\n"
            f"1. WHY did it fail?\n"
            f"2. What will you do DIFFERENTLY?\n"
            f"3. What is your completely new approach?\n\n"
            f"Do NOT execute any other tools until you have successfully called the `think` tool."
        )

    # ------------------------------------------------------------------
    # 🏅 Memory Consolidation: auto-save lessons after task completion
    # ------------------------------------------------------------------
    def _consolidate_memory(self, task_input: str, result: AgentResult):
        """After a task completes, summarize the experience as a reusable lesson.

        Stores a compact 'rule' in SQLite memory so the agent avoids
        repeating mistakes and reuses successful strategies.
        """
        if not result.tool_results:
            return  # No tools used, nothing to learn from

        try:
            from sunday.agents._lesson_store import get_lesson_store
            store = get_lesson_store()

            # Build a compact summary of what happened
            tools_used = [tr.tool_name for tr in result.tool_results if tr.tool_name]
            failed_tools = [tr.tool_name for tr in result.tool_results if not tr.success]
            success_rate = sum(1 for tr in result.tool_results if tr.success) / max(len(result.tool_results), 1)

            lesson = (
                f"Task: {task_input[:100]}\n"
                f"Tools: {', '.join(set(tools_used))}\n"
                f"Failed: {', '.join(set(failed_tools)) if failed_tools else 'none'}\n"
                f"Success rate: {success_rate:.0%}\n"
                f"Turns: {result.turns}\n"
                f"Result: {'completed' if success_rate > 0.5 else 'struggled'}"
            )

            store.store(lesson, source="auto_lesson", metadata={"type": "lesson", "auto": True})
            print(f"[🏅 MEMORY] Consolidated lesson: {task_input[:50]}... ({len(result.tool_results)} tools, {success_rate:.0%} success)")
        except Exception as e:
            print(f"[WARN] Memory consolidation failed: {e}")




    def _strip_think_tags(self, text: str) -> str:
        """Forcefully remove internal thinking/reasoning blocks."""
        if not text:
            return ""
        # Remove XML-style tags
        text = re.sub(r"<thought>.*?</thought>", "", text, flags=re.DOTALL | re.IGNORECASE)
        # Remove structured headers
        text = re.sub(r"(?:THOUGHT|Thinking Process|Reasoning|THOUGHTS):\s*.*?(?=\nTOOL:|\nFINAL|\Z)", "", text, flags=re.DOTALL | re.IGNORECASE)
        # Remove patterns like "(Wait, ...)" or "Revised: ..." or "Note: ..."
        text = re.sub(r"\(Wait,.*?\)", "", text, flags=re.DOTALL | re.IGNORECASE)
        text = re.sub(r"(?:Revised|Correction|Choice|Final Choice|Wait).*?:.*", "", text, flags=re.IGNORECASE)
        # Remove common assistant preambles
        text = re.sub(r"^(?:Certainly|Sure|I can help with that|Of course|Okay|Alright)[^.!?]*[.!?]\s*", "", text, flags=re.IGNORECASE)
        return text.strip()

    def _run_visual_audit(self, query: str, content: str) -> str:
        """Takes a screenshot and asks a VLM to verify the final answer."""
        print(f"[👁️ VISUAL AUDIT] Verifying final answer with screenshot...")
        try:
            # 1. Take Screenshot using the existing tool registry
            from sunday.tools.browser import BrowserScreenshotTool
            screenshot_tool = BrowserScreenshotTool()
            res = screenshot_tool.execute()
            if not res.success:
                return "ACCEPT (Screenshot tool failed)"

            b64_img = res.metadata.get("screenshot_base64")
            if not b64_img:
                return "ACCEPT (No image data captured)"

            # 2. Call VLM (Local or Cloud)
            audit_msg = (
                f"User Query: {query}\n"
                f"Proposed Answer: {content}\n\n"
                "Does this screenshot support the answer? "
                "Check for 404 pages or incorrect prices/models."
            )

            # Build multi-modal message
            messages = [
                Message(role=Role.SYSTEM, content=self._visual_audit_prompt),
                Message(role=Role.USER, content=audit_msg, images=[b64_img]),
            ]

            # Use local or cloud based on current config
            # Note: We use 0.0 temperature for maximum verification accuracy
            res_vlm, _ = self._generate(
                messages,
                max_tokens=250,
                temperature=0.0,
            )
            return res_vlm.get("content", "").strip()

        except Exception as e:
            print(f"[⚠️ AUDIT ERROR] {e}")
            import traceback

            traceback.print_exc()
            return "ACCEPT (Audit logic crashed)"

    def _run_critic_check(self, tool_calls: List[ToolCall], history: List[Message]) -> Optional[str]:
        """Actor-Critic Pattern: Verify the plan before executing high-risk browser tools."""
        has_browser_tools = any(tc.name.startswith("browser_") for tc in tool_calls)
        if not has_browser_tools:
            return None
            
        plan_str = "\n".join([f"- {tc.name}({tc.arguments})" for tc in tool_calls])
        
        critic_prompt = (
            f"You are the SYSTEM WATCHDOG. The Agent has proposed this plan:\n"
            f"{plan_str}\n\n"
            f"Is this plan efficient? REJECT if:\n"
            f"1. It navigates to a URL that has already failed.\n"
            f"2. It tries to scroll/click without extracting data first.\n"
            f"3. It could be solved faster with 'web_search'.\n\n"
            f"Output 'APPROVED' if it's the best next step.\n"
            f"Output 'REJECTED: [Reason]' if it's a mistake."
        )
        
        # We only pass recent context + critic prompt to save tokens
        recent_context = [m for m in history if m.role != Role.SYSTEM][-3:]
        critic_messages = recent_context + [Message(role=Role.USER, content=critic_prompt)]
        
        try:
            # Force local provider for fast verification, don't pass tools
            provider = self._config.intelligence.provider
            old_model = self._model
            if provider == "hybrid":
                self._model = self._config.intelligence.fallback_model
                
            # Temporarily set provider to local if it's hybrid
            old_provider = provider
            if provider == "hybrid":
                self._config.intelligence.provider = "local"

            res, _ = self._generate(critic_messages, max_tokens=150, temperature=0.0)
            
            # Restore state
            self._config.intelligence.provider = old_provider
            self._model = old_model
                
            verdict = res.get("content", "").strip()
            
            if verdict.startswith("REJECTED"):
                return verdict.replace("REJECTED:", "").strip()
            return None
        except Exception as e:
            print(f"[WARN] Critic check failed: {e}")
            return None



    # ------------------------------------------------------------------
    # UI / Observability
    # ------------------------------------------------------------------

    def _emit_turn_start(self, input_text: str):
        """Emit start of a reasoning turn."""
        if self._bus:
            from sunday.core.events import EventType
            self._bus.publish(EventType.AGENT_TURN_START, {"input": input_text, "agent_id": self.agent_id})

    def _emit_turn_end(self, turns: int, content_length: int = 0, **kwargs: Any):
        """Emit end of a reasoning turn."""
        if self._bus:
            from sunday.core.events import EventBus, EventType
            payload = {"turns": turns, "agent_id": self.agent_id, "content_length": content_length}
            payload.update(kwargs)
            self._bus.publish(EventType.AGENT_TURN_END, payload)

    def _emit_tool_call(self, tool_call: ToolCall):
        """Emit a tool call event for UI visibility."""
        if self._bus:
            from sunday.core.events import EventType
            # Use TOOL_CALL_START to notify UI of execution
            self._bus.publish(EventType.TOOL_CALL_START, {
                "id": tool_call.id,
                "tool": tool_call.name,        # Frontend reads "tool"
                "name": tool_call.name,        # Backend compat
                "arguments": tool_call.arguments,
                "agent_id": self.agent_id
            })

    def _emit_tool_call_end(self, tool_call: ToolCall, tool_result: ToolResult, latency_ms: float = 0):
        """Emit a tool call completion event for UI visibility."""
        if self._bus:
            from sunday.core.events import EventType
            self._bus.publish(EventType.TOOL_CALL_END, {
                "id": tool_call.id,
                "tool": tool_call.name,
                "name": tool_call.name,
                "success": tool_result.success,
                "result": (tool_result.content or "")[:500],  # Truncate for UI
                "latency": latency_ms,
                "agent_id": self.agent_id,
            })

    def _emit_metadata(self, metadata: dict):
        """Emit arbitrary metadata for UI enrichment (e.g. Execution Plan)."""
        if self._bus:
            from sunday.core.events import EventType
            # Use TELEMETRY_RECORD as a catch-all for metadata updates
            self._bus.publish(EventType.TELEMETRY_RECORD, {
                "metadata": metadata,
                "agent_id": self.agent_id
            })

    def _inject_brain_tag(self, arguments_json: str, brain_tag: str) -> str:
        """Inject the [🧠 CLOUD] or [⚡ LOCAL] tag into tool arguments for UI display."""
        if not brain_tag: return arguments_json
        import json
        try:
            args = json.loads(arguments_json)
            args["_brain"] = brain_tag
            return json.dumps(args, ensure_ascii=False)
        except:
            return arguments_json


    def _get_relevant_tools(self, user_input: str) -> Optional[List[str]]:
        """Heuristically select tools relevant to the current user input to reduce prompt bloat."""
        if not self._tools:
            return None
            
        all_names = [t.spec.name for t in self._tools]
        
        # ALWAYS keep these core tools for the Orchestrator
        core_tools = {
            "think", "delegate_browser", "delegate_coding", 
            "list_tools", "reload_tools", "system_health"
        }
        
        # CATEGORIES based on keywords
        categories = {
            "coding": ["file_read", "file_write", "create_tool_scaffold", "shell_exec", "graphify", "code_interpreter"],
            "browser": ["browser_navigate", "browser_click", "browser_type", "browser_screenshot", "browser_extract", "browser_axtree", "e2e_browser_test"],
            "search": ["web_search", "google_search"],
            "testing": ["run_harness_test", "e2e_browser_test"],
            "system": ["system_health"],
            "meta": ["list_tools", "reload_tools"]
        }
        
        query = user_input.lower()
        selected = set(core_tools)
        
        # Match keywords (Thai + English)
        if any(k in query for k in ["file", "code", "write", "read", "python", "script", "tool", "scaffold", "สร้าง", "เขียน", "ไฟล์"]):
            selected.update(categories["coding"])
        
        if any(k in query for k in ["web", "search", "browse", "go to", "url", "site", "find", "ค้นหา", "เว็บ", "เช็ค", "ราคา"]):
            selected.update(categories["browser"])
            selected.update(categories["search"])
            
        if any(k in query for k in ["system", "health", "cpu", "ram", "disk", "เครื่อง", "สุขภาพ"]):
            selected.add("system_health")
            
        if any(k in query for k in ["test", "verify", "harness", "check", "run", "ทดสอบ", "รัน", "ตรวจ"]):
            selected.update(categories["testing"])

        # Intersection with what's actually available
        final_list = [name for name in all_names if name in selected]
        
        # If we matched nothing or it's too short, return everything to be safe
        if len(final_list) <= len(core_tools) and len(query) > 50:
             return None
             
        return final_list

__all__ = ["OrchestratorAgent"]
