"""Shared engine utilities and re-exports."""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any, Dict, List

from sunday.core.types import Message
from sunday.engine._stubs import InferenceEngine
from sunday._rust_bridge import Tokenizer
import os

_LOADED_TOKENIZERS = set()


class EngineConnectionError(Exception):
    """Raised when an engine is unreachable."""


def messages_to_dicts(messages: Sequence[Message]) -> List[Dict[str, Any]]:
    """Convert ``Message`` objects to OpenAI-format dicts."""
    out: List[Dict[str, Any]] = []
    for m in messages:
        d: Dict[str, Any] = {"role": m.role.value}
        
        # Handle Multi-modal content
        if m.images:
            # Cloud engines (OpenAI, Gemini, Anthropic) usually expect list of blocks
            content_blocks = [{"type": "text", "text": m.content}]
            for img in m.images:
                # If it's already a data URL, use it, otherwise wrap it
                url = img if img.startswith("data:") else f"data:image/png;base64,{img}"
                content_blocks.append({"type": "image_url", "image_url": {"url": url}})
            d["content"] = content_blocks
            # Ollama also likes a top-level images list
            d["images"] = m.images
        else:
            d["content"] = m.content

        if m.name:
            d["name"] = m.name
        if m.tool_calls:
            d["tool_calls"] = [
                {
                    "id": tc.id,
                    "type": "function",
                    "function": {
                        "name": tc.name,
                        "arguments": tc.arguments,
                    },
                }
                for tc in m.tool_calls
            ]
        if m.tool_call_id:
            d["tool_call_id"] = m.tool_call_id
        out.append(d)
    return out


def estimate_prompt_tokens(messages: Sequence[Message], model_name: str = "default") -> int:
    """Estimate full prompt token count from message content.

    Uses Rust-backed precise tokenizer if available, otherwise falls back to
    rough character-based estimation.
    """
    if Tokenizer:
        # Auto-load if not already loaded and file exists
        if model_name not in _LOADED_TOKENIZERS:
            # Try common paths
            paths = [
                "llama-cpp/models/tokenizer.json",
                "models/tokenizer.json",
                f"models/{model_name}/tokenizer.json"
            ]
            for p in paths:
                if os.path.exists(p):
                    try:
                        Tokenizer.load_from_file(model_name, p)
                        _LOADED_TOKENIZERS.add(model_name)
                        print(f"[✨ TOKENIZER] Loaded precise tokenizer for '{model_name}' from {p}")
                        break
                    except:
                        continue

        total = 0
        for m in messages:
            total += Tokenizer.count_tokens(model_name, m.content or "")
        return total

    total_chars = sum(len(m.content) for m in messages)
    # ~4 tokens overhead per message for role markers / separators
    overhead = len(messages) * 4
    return max(1, total_chars // 4 + overhead)


__all__ = [
    "EngineConnectionError",
    "InferenceEngine",
    "estimate_prompt_tokens",
    "messages_to_dicts",
]
