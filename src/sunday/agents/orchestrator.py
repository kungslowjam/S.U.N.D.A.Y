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
import re
from typing import Any, List, Optional

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

    def _generate(self, messages: List[Message]) -> dict:
        """Centralized generation with Hybrid Routing.
        
        Logic:
        1. If provider is 'hybrid':
           - Use 'openrouter' for the first steps (Planning & Research).
           - Switch to 'local' for summarization if raw data (>>> DATA RETRIEVED) is found.
        2. Otherwise, use the configured provider.
        """
        provider = self._config.intelligence.provider
        
        if provider == "hybrid":
            # Rule: Use Cloud for research, Local for summary
            last_msg_content = messages[-1].content if messages else ""
            
            # If we just extracted data, or have been chatting a lot, use Local to summarize
            if ">>> DATA RETRIEVED" in last_msg_content or len(messages) > 10:
                current_provider = "local"
                model = self._config.intelligence.fallback_model
            else:
                current_provider = "openrouter"
                model = self._config.intelligence.default_model
        else:
            current_provider = provider
            model = self._model # Use the model assigned to this agent

        return self._engine.generate(
            messages,
            model=model,
            temperature=self._temperature,
            max_tokens=self._max_tokens,
            # We add a hint for the engine to know which provider to route to
            provider=current_provider 
        )

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

        # Build system prompt
        if self._system_prompt:
            sys_prompt = self._system_prompt
        else:
            from sunday.learning.intelligence.orchestrator.prompt_registry import (
                build_system_prompt,
            )

            sys_prompt = build_system_prompt(tools=self._tools)

        messages = self._build_messages(input, context, system_prompt=sys_prompt)

        # === SMART HARNESS: Pre-calculate optimized URLs for known domains ===
        smart_hint = self._generate_smart_url(input)
        if smart_hint:
            messages.append(Message(role=Role.USER, content=f"HINT: To be faster, use this direct URL immediately: {smart_hint}"))

        all_tool_results: list[ToolResult] = []
        turns = 0

        for _turn in range(self._max_turns):
            turns += 1

            if self._loop_guard:
                messages = self._loop_guard.compress_context(messages)

            result = self._generate(messages)
            content = result.get("content", "")

            parsed = self._parse_structured_response(content)

            # FINAL_ANSWER -> done
            if parsed["final_answer"]:
                self._emit_turn_end(turns=turns)
                return AgentResult(
                    content=parsed["final_answer"],
                    tool_results=all_tool_results,
                    turns=turns,
                )

            # TOOL -> execute
            if parsed["tool"]:
                messages.append(Message(role=Role.ASSISTANT, content=content))

                tool_call = ToolCall(
                    id=f"orch_{turns}",
                    name=parsed["tool"],
                    arguments=parsed["input"] or "{}",
                )

                # === INTERCEPT: If model tries to scroll/click on a results page, force extract instead ===
                if tool_call.name in ("browser_scroll", "browser_click") and turns > 5:
                    # Check if we should have extracted already
                    last_msg = messages[-1].content if messages else ""
                    if "properties found" in last_msg.lower() or "Search results" in last_msg:
                         tool_call.name = "browser_extract"
                         tool_call.arguments = '{"selector": "body", "extract_type": "text"}'

                tool_result = self._executor.execute(tool_call)
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
                messages = self._compress_tool_outputs(messages)
                continue

            # Neither -> treat content as final answer (but strip thinking)
            final_content = self._strip_think_tags(content)
            self._emit_turn_end(turns=turns)
            return AgentResult(
                content=final_content,
                tool_results=all_tool_results,
                turns=turns,
            )

        # Max turns exceeded
        return self._max_turns_result(all_tool_results, turns)

    @staticmethod
    def _parse_structured_response(text: str) -> dict:
        """Parse THOUGHT/TOOL/INPUT/FINAL_ANSWER from model output."""
        result = {
            "thought": "",
            "tool": "",
            "input": "",
            "final_answer": "",
        }

        # Match THOUGHT: or Thinking Process: or Reasoning:
        thought_match = re.search(
            r"(?:THOUGHT|Thinking Process|Reasoning):\s*(.+?)(?=\nTOOL:|\nFINAL[_ ]?ANSWER:|\Z)",
            text,
            re.DOTALL | re.IGNORECASE,
        )
        if thought_match:
            result["thought"] = thought_match.group(1).strip()

        final_match = re.search(
            r"FINAL[_ ]?ANSWER:\s*(.+)",
            text,
            re.DOTALL | re.IGNORECASE,
        )
        if final_match:
            result["final_answer"] = final_match.group(1).strip()
            return result

        # 1. Match Inline Format TOOL: name({"key": "val"})
        inline_match = re.search(
            r"TOOL:\s*(\w+)\s*\((.+?)\)(?=\n||\Z)", 
            text, 
            re.DOTALL | re.IGNORECASE
        )
        if inline_match:
            result["tool"] = inline_match.group(1).strip()
            result["input"] = inline_match.group(2).strip()
            # If we found an inline call, we can return early or continue to look for final_answer
            # but usually a tool call means we're not done yet.
            return result

        # 2. Match Standard Format TOOL: name \n INPUT: json
        tool_match = re.search(r"TOOL:\s*(\w+)", text, re.IGNORECASE)
        if tool_match:
            result["tool"] = tool_match.group(1).strip()

        input_match = re.search(
            r"INPUT:\s*(.+?)(?=\n(?:THOUGHT|Thinking Process|Reasoning):|\nTOOL:|\nFINAL|\Z)",
            text,
            re.DOTALL | re.IGNORECASE,
        )
        if input_match:
            result["input"] = input_match.group(1).strip()

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

        # Build initial messages
        messages = self._build_messages(input, context)

        # === SMART HARNESS: Pre-calculate optimized URLs for known domains ===
        smart_hint = self._generate_smart_url(input)
        if smart_hint:
            messages.append(Message(role=Role.USER, content=f"HINT: To be faster, use this direct URL immediately: {smart_hint}"))

        # Get OpenAI-format tool definitions
        openai_tools = self._executor.get_openai_tools() if self._tools else []

        all_tool_results: list[ToolResult] = []
        turns = 0
        total_prompt_tokens = 0
        total_completion_tokens = 0

        for _turn in range(self._max_turns):
            turns += 1

            if self._loop_guard:
                messages = self._loop_guard.compress_context(messages)

            # Build generate kwargs
            gen_kwargs: dict[str, Any] = {}
            if openai_tools:
                gen_kwargs["tools"] = openai_tools

            result = self._generate(messages, **gen_kwargs)

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

            # Build ToolCall objects from raw dicts
            tool_calls = [
                ToolCall(
                    id=tc.get("id", f"call_{i}"),
                    name=tc.get("name", ""),
                    arguments=tc.get("arguments", "{}"),
                )
                for i, tc in enumerate(raw_tool_calls)
            ]

            # Append assistant message with tool calls
            messages.append(
                Message(
                    role=Role.ASSISTANT,
                    content=content,
                    tool_calls=tool_calls,
                )
            )

            # Execute each tool (with loop guard check) and append results
            if self._parallel_tools and len(tool_calls) > 1:
                # Parallel execution
                def _exec_tool(tc: ToolCall) -> tuple:
                    if self._loop_guard:
                        verdict = self._loop_guard.check_call(
                            tc.name,
                            tc.arguments,
                        )
                        if verdict.blocked:
                            return tc, ToolResult(
                                tool_name=tc.name,
                                content=f"Loop guard: {verdict.reason}",
                                success=False,
                            )
                    return tc, self._executor.execute(tc)

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
                    _, tool_result = results_map[id(tc)]
                    all_tool_results.append(tool_result)
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
                    all_tool_results.append(tool_result)

                    # === AUTO-COMPLETE: If browser_extract returned listing data, finish now ===
                    auto_answer = self._check_auto_complete(tool_result, input)
                    if auto_answer:
                        self._emit_turn_end(turns=turns)
                        return AgentResult(
                            content=auto_answer,
                            tool_results=all_tool_results,
                            turns=turns,
                            metadata={
                                "auto_completed": True,
                                "prompt_tokens": total_prompt_tokens,
                                "completion_tokens": total_completion_tokens,
                            },
                        )

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
        self._emit_turn_end(turns=turns, max_turns_exceeded=True)
        return AgentResult(
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
    
    def _compress_tool_outputs(self, messages: List[Message]) -> List[Message]:
        """Truncate large tool outputs that have already been processed by the assistant."""
        THRESHOLD = 1500 # Characters
        new_messages = []
        for i, msg in enumerate(messages):
            # If it's a large observation and there's a subsequent assistant response, truncate it.
            if msg.role == Role.USER and msg.content.startswith("Observation: ") and len(msg.content) > THRESHOLD:
                if i + 1 < len(messages) and messages[i+1].role == Role.ASSISTANT:
                    # Keep a snippet and a note
                    snippet = msg.content[:200]
                    truncated = f"{snippet}... [RAW DATA TRUNCATED TO SAVE MEMORY. REFER TO PREVIOUS SUMMARY IN THOUGHT.]"
                    new_messages.append(Message(role=Role.USER, content=truncated))
                    continue
            new_messages.append(msg)
        return new_messages


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

    def _check_auto_complete(self, tool_result: ToolResult, user_input: str) -> Optional[str]:
        """Check if a browser_extract result contains listing data that can be auto-completed.
        
        When the model is too weak to decide when to stop, the orchestrator
        takes over: it detects structured listing data from browser_extract
        and formats a final answer directly, bypassing the model.
        """
        if tool_result.tool_name != "browser_extract" or not tool_result.success:
            return None
        
        content = tool_result.content
        if ">>> DATA RETRIEVED" not in content:
            return None
        
        # Strip the directive marker
        data_text = content.split(">>> DATA RETRIEVED")[0].strip()
        
        # Parse hotel/listing entries from the cleaned text
        import re as _re
        
        entries = []
        # Look for patterns like:
        # Hotel Name
        # ...
        # Scored 9.1
        # 9.1
        # Superb
        # 3,290 reviews
        
        # Split by blocks that look like entries (usually start with a name and end with reviews)
        # We'll use a more flexible state machine
        lines = data_text.split('\n')
        current_entry = {}
        
        for i, line in enumerate(lines):
            line = line.strip()
            if not line or len(line) < 2: continue
            
            # 1. Detect Score/Rating (The most reliable anchor)
            score_match = _re.search(r'(?:Scored\s+)?(\d\.\d)', line)
            if score_match and 'score' not in current_entry:
                current_entry['score'] = float(score_match.group(1))
                # If we have a score but no name, look back 1-3 lines for the name
                if not current_entry.get('name'):
                    for j in range(1, 5):
                        if i - j >= 0:
                            prev_line = lines[i-j].strip()
                            if len(prev_line) > 5 and 'window' not in prev_line and 'map' not in prev_line and 'access' not in prev_line:
                                current_entry['name'] = prev_line
                                break
                continue

            # 2. Detect Reviews (The end of an entry)
            if 'reviews' in line.lower() and _re.search(r'[\d,]+', line):
                current_entry['reviews'] = _re.search(r'[\d,]+', line).group()
                if current_entry.get('name') and current_entry.get('score'):
                    entries.append(dict(current_entry))
                current_entry = {}
                continue

            # 3. Detect Price
            price_match = _re.search(r'(?:AUD|USD|JPY|EUR|¥|฿|THB|Price)\s*([\d,.]+)', line)
            if price_match:
                current_entry['price'] = line
                continue

        # Fallback: If no entries found with reviews anchor, try a simpler name-score match
        if not entries:
            for i, line in enumerate(lines):
                score_match = _re.search(r'(?:Scored\s+)?([98]\.\d)', line)
                if score_match:
                    score = float(score_match.group(1))
                    # Look for name in previous lines
                    name = ""
                    for j in range(1, 4):
                        if i-j >= 0:
                            prev = lines[i-j].strip()
                            if len(prev) > 5 and 'window' not in prev and 'map' not in prev:
                                name = prev
                                break
                    if name:
                        entries.append({'name': name, 'score': score})

        if not entries:
            return None 
        
        # Deduplicate entries by name
        seen_names = set()
        unique_entries = []
        for e in entries:
            if e['name'] not in seen_names:
                unique_entries.append(e)
                seen_names.add(e['name'])
        entries = unique_entries

        # Filter by user criteria
        user_lower = user_input.lower()
        min_score = 0
        rating_filter = _re.search(r'(\d+(?:\.\d+)?)\+|คะแนน\s*(\d+(?:\.\d+)?)', user_lower)
        if rating_filter:
            min_score = float(rating_filter.group(1) or rating_filter.group(2))
        
        filtered = [e for e in entries if e.get('score', 0) >= min_score]
        
        # Sort
        # If "cheapest" or "ถูกที่สุด" is mentioned, and we have prices
        if ("cheap" in user_lower or "ถูก" in user_lower) and any(e.get('price') for e in filtered):
            def sort_price(e):
                p = e.get('price', '999,999')
                m = _re.search(r'[\d,]+', p)
                return float(m.group().replace(',', '')) if m else 999999
            filtered.sort(key=sort_price)
        else:
            filtered.sort(key=lambda e: e.get('score', 0), reverse=True)
        
        # Limit
        top_n = 3
        top_n_match = _re.search(r'(\d+)\s*(?:อันดับ|อัน|top|ที่สุด)', user_lower)
        if top_n_match: top_n = int(top_n_match.group(1))
        
        display = filtered[:top_n]
        
        if not display:
            return f"พบโรงแรม {len(entries)} แห่งในย่าน Shinjuku แต่อันที่มีคะแนน {min_score}+ ยังไม่แสดงราคา (อาจเป็นเพราะยังไม่ระบุวันที่เช็คอิน)\n\n**โรงแรมแนะนำที่มีคะแนนสูง:**\n" + "\n".join([f"- {e['name']} (คะแนน {e['score']})" for e in entries[:3]]) + "\n\n💡 **Tip:** ลองระบุวันที่เข้าพักด้วย เช่น 'หาโรงแรม Shinjuku คะแนน 9+ วันที่ 15-17 พ.ค.' เพื่อให้ระบบดึงราคาถูกที่สุดมาให้ครับ"
        
        # Format the answer
        result_lines = []
        for i, e in enumerate(display, 1):
            name = e.get('name', 'ไม่ระบุชื่อ')
            score = e.get('score', 'N/A')
            price = e.get('price', 'ไม่ระบุราคา (โปรดเลือกวันที่)')
            rev = e.get('reviews', 'N/A')
            result_lines.append(f"**{i}. {name}**\n   ⭐ คะแนน: {score}/10 | 📝 รีวิว: {rev}\n   💰 ราคา: {price}")
        
        header = f"🏨 ผลการค้นหาโรงแรมย่าน Shinjuku (คะแนน {min_score}+):\n\n"
        return header + "\n\n".join(result_lines) + "\n\n📌 *หมายเหตุ: ข้อมูลนี้เป็นราคาเริ่มต้นหรือราคาประเมิน โปรดเช็คราคาจริงโดยระบุวันที่เข้าพักอีกครั้งครับ*"

    def _generate_smart_url(self, user_input: str) -> Optional[str]:
        """Generate an optimized search URL based on the user's input to bypass homepages."""
        input_lower = user_input.lower()
        import re as _re
        
        # 1. Booking.com Logic
        if "booking" in input_lower:
            # Extract city
            city_match = _re.search(r'(?:ใน|at|in|ย่าน|เมือง)\s*([a-zA-Zก-ฮ]+)', user_input)
            city = city_match.group(1) if city_match else "Tokyo"
            
            # Extract dates (e.g. 20-22 พ.ค. or 2025-05-20)
            checkin = ""
            checkout = ""
            
            # Try to find dates like 20-22
            date_range = _re.search(r'(\d{1,2})[-ถึง\s]+(\d{1,2})\s*([a-zA-Zก-ฮ.]+)', input_lower)
            if date_range:
                d1, d2, month = date_range.groups()
                # Simple month mapper for 2025
                months = {"ม.ค.": "01", "ก.พ.": "02", "มี.ค.": "03", "เม.ย.": "04", "พ.ค.": "05", "มิ.ย.": "06", 
                          "ก.ค.": "07", "ส.ค.": "08", "ก.ย.": "09", "ต.ค.": "10", "พ.ย.": "11", "ธ.ค.": "12",
                          "may": "05", "june": "06", "july": "07"}
                m_num = "05" # Default to May
                for k, v in months.items():
                    if k in month: m_num = v; break
                
                checkin = f"2025-{m_num}-{int(d1):02d}"
                checkout = f"2025-{m_num}-{int(d2):02d}"
            
            # Extract stars
            nflt = ""
            if "5 ดาว" in user_input or "5 star" in input_lower:
                nflt = "&nflt=class%3D5"
            elif "4 ดาว" in user_input or "4 star" in input_lower:
                nflt = "&nflt=class%3D4"
            
            url = f"https://www.booking.com/searchresults.html?ss={city}"
            if checkin: url += f"&checkin={checkin}&checkout={checkout}"
            if nflt: url += nflt
            return url
            
        # 2. Amazon.co.jp Logic
        if "amazon" in input_lower:
            query_match = _re.search(r'(?:หา|search for|find)\s*([a-zA-Zก-ฮ\s]+)', user_input)
            if query_match:
                return f"https://www.amazon.co.jp/s?k={query_match.group(1).strip()}"
                
        return None


__all__ = ["OrchestratorAgent"]
