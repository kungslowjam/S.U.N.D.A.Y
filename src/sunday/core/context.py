"""Centralized context management for SUNDAY agents.

Provides utilities for message windowing, compression, and optimization to keep
token usage within limits while preserving essential conversation state.
"""

from __future__ import annotations

import logging
from typing import List, Optional

from sunday.core.types import Message, Role

logger = logging.getLogger(__name__)


class ContextManager:
    """Manages agent conversation context with windowing and compression."""

    def __init__(
        self,
        max_messages: int = 20,
        max_tokens: int = 4096,  # ⚡ Default cap for local models
        compression_threshold: int = 1000, # ⚡ Lower threshold for faster prefill
        preserve_system: bool = True,
        preserve_initial_user: bool = True,
    ):
        self.max_messages = max_messages
        self.max_tokens = max_tokens
        self.compression_threshold = compression_threshold
        self.preserve_system = preserve_system
        self.preserve_initial_user = preserve_initial_user

    def optimize(
        self,
        messages: List[Message],
        *,
        max_messages: Optional[int] = None,
        max_tokens: Optional[int] = None,
        threshold: Optional[int] = None,
    ) -> List[Message]:
        """Apply all optimization strategies in sequence."""
        m = messages
        # 1. Compress large blobs first
        m = self.compress_tool_outputs(m, threshold=threshold)
        # 2. Window by message count
        m = self.apply_window(m, max_messages=max_messages)
        # 3. Final safety: Window by token estimate
        m = self.trim_to_token_limit(m, max_tokens=max_tokens or self.max_tokens)
        return m

    def estimate_tokens(self, messages: List[Message]) -> int:
        """Rough token estimate (4 chars/token + overhead)."""
        total_chars = sum(len(m.content or "") for m in messages)
        overhead = len(messages) * 4
        return (total_chars // 4) + overhead

    def trim_to_token_limit(
        self,
        messages: List[Message],
        max_tokens: int,
    ) -> List[Message]:
        """Aggressively remove middle messages until token count is safe."""
        if not messages or self.estimate_tokens(messages) <= max_tokens:
            return messages

        # Preserve System and Initial User
        system_msgs = [m for m in messages if m.role == Role.SYSTEM]
        user_msgs = [m for m in messages if m.role == Role.USER]
        initial_user = user_msgs[0] if user_msgs else None
        
        # We start with only the most recent messages and try to add back
        # until we hit the limit.
        recent_history = messages[::-1] # Reverse to get most recent first
        final_history = []
        
        # Base tokens (System + Initial User)
        base_msgs = list(system_msgs)
        if initial_user and initial_user not in system_msgs:
            base_msgs.append(initial_user)
        
        current_tokens = self.estimate_tokens(base_msgs)
        
        for msg in recent_history:
            if msg in base_msgs:
                continue
            
            msg_tokens = self.estimate_tokens([msg])
            if current_tokens + msg_tokens > max_tokens:
                break
            
            final_history.append(msg)
            current_tokens += msg_tokens
            
        # Re-assemble in correct order
        return base_msgs + final_history[::-1]

    def compress_tool_outputs(
        self,
        messages: List[Message],
        threshold: Optional[int] = None,
    ) -> List[Message]:
        """Truncate large tool outputs (observations) that have been processed.

        If a TOOL message or a USER message starting with 'Observation:' is larger
        than the threshold and is followed by an ASSISTANT message, we truncate it
        since the assistant has already 'seen' and likely synthesized the data.
        """
        limit = threshold or self.compression_threshold
        new_messages = []
        
        for i, msg in enumerate(messages):
            content = msg.content or ""
            is_large_obs = (
                (msg.role == Role.TOOL or (msg.role == Role.USER and content.startswith("Observation:")))
                and len(content) > limit
            )
            
            # If it's a large observation and there's a subsequent assistant response
            if is_large_obs and i + 1 < len(messages) and messages[i+1].role == Role.ASSISTANT:
                # Truncate content but keep the start and end for context
                truncated_content = (
                    f"{content[:500]}\n\n"
                    f"... [TRUNCATED {len(content) - 1000} CHARS] ...\n\n"
                    f"{content[-500:]}"
                )
                
                # Create a new message object to avoid mutating the original
                new_msg = Message(
                    role=msg.role,
                    content=truncated_content,
                    name=msg.name,
                    tool_calls=msg.tool_calls,
                    tool_call_id=msg.tool_call_id,
                    images=msg.images,
                    metadata=msg.metadata.copy()
                )
                new_msg.metadata["is_truncated"] = True
                new_messages.append(new_msg)
            else:
                new_messages.append(msg)
                
        return new_messages

    def apply_window(
        self,
        messages: List[Message],
        max_messages: Optional[int] = None,
    ) -> List[Message]:
        """Keep only the most recent messages while preserving essential context."""
        limit = max_messages or self.max_messages
        if len(messages) <= limit:
            return messages

        # 1. Identify essential messages
        system_msgs = []
        initial_user = None
        
        if self.preserve_system:
            system_msgs = [m for m in messages if m.role == Role.SYSTEM]
            
        if self.preserve_initial_user:
            # Find the first user message after the system prompt
            for m in messages:
                if m.role == Role.USER:
                    initial_user = m
                    break

        # 2. Calculate window size
        # We want to keep: [System] + [Initial User] + [Recent History]
        recent_count = limit - len(system_msgs)
        if initial_user and initial_user not in messages[-recent_count:]:
            recent_count -= 1
        
        if recent_count <= 0:
            # Fallback if system prompts are too many
            return system_msgs + messages[-2:]

        recent_history = messages[-recent_count:]
        
        # 3. Assemble
        final_messages = list(system_msgs)
        if initial_user and initial_user not in recent_history:
            final_messages.append(initial_user)
        
        final_messages.extend(recent_history)
        
        # Deduplicate (just in case)
        seen_ids = set()
        deduped = []
        for m in final_messages:
            m_id = id(m)
            if m_id not in seen_ids:
                deduped.append(m)
                seen_ids.add(m_id)
                
        return deduped

    def truncate_to_minimal(self, messages: List[Message]) -> List[Message]:
        """Extreme compression: only system prompt and last exchange."""
        system_msgs = [m for m in messages if m.role == Role.SYSTEM]
        tail = [m for m in messages if m.role != Role.SYSTEM]
        return system_msgs + tail[-2:]
