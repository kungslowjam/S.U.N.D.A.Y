"""Native Rust-backed inference engine using llama.cpp directly."""

from __future__ import annotations

import json
import logging
from typing import Any, Dict, List, Optional, Sequence, AsyncIterator

from sunday.core.registry import EngineRegistry
from sunday.core.types import Message
from sunday.engine._base import InferenceEngine, messages_to_dicts
from sunday.engine._stubs import StreamChunk
from sunday._rust_bridge import get_rust_module, generate_result_from_json

logger = logging.getLogger(__name__)

@EngineRegistry.register("native")
class NativeEngine(InferenceEngine):
    """Direct GGUF inference via llama.cpp (Rust native)."""

    engine_id = "native"

    def __init__(self, host: Optional[str] = None, **kwargs: Any):
        """Initialize with model path (passed as 'host' for registry compatibility)."""
        import os
        from pathlib import Path

        mod = get_rust_module()
        
        # Default fallback chain
        default_model = "llama-cpp/models/Qwen3.5-0.8B-Q4_K_M.gguf"
        if not host:
            if not os.path.exists(default_model):
                # Try to find ANY gguf in the models directory
                models_dir = Path("llama-cpp/models")
                if models_dir.exists():
                    ggufs = sorted(list(models_dir.glob("*.gguf")))
                    if ggufs:
                        default_model = str(ggufs[0])
                        logger.info("Auto-discovered model: %s", default_model)
            self._model_path = default_model
        else:
            self._model_path = host

        try:
            self._inner = mod.Engine("native", host=self._model_path)
            logger.info("Initialized NativeEngine with model: %s", self._model_path)
        except Exception as exc:
            logger.error("Failed to initialize NativeEngine: %s", exc)
            raise

    def generate(
        self,
        messages: Sequence[Message],
        *,
        model: str,
        temperature: float = 0.7,
        max_tokens: int = 1024,
        **kwargs: Any,
    ) -> Dict[str, Any]:
        """Synchronous completion."""
        # Convert core Messages to bridge PyMessages
        mod = get_rust_module()
        py_msgs = []
        for m in messages:
            py_msgs.append(mod.PyMessage(m.role.value, m.content or ""))
            
        json_res = self._inner.generate(
            py_msgs, 
            model, 
            temperature=temperature, 
            max_tokens=max_tokens
        )
        return generate_result_from_json(json_res)

    async def stream(
        self,
        messages: Sequence[Message],
        *,
        model: str,
        temperature: float = 0.7,
        max_tokens: int = 1024,
        **kwargs: Any,
    ) -> AsyncIterator[str]:
        """Yield token strings (Note: Currently sync in bridge, so yields all at once or simulated)."""
        # For now, NativeLlamaEngine.stream is not yet implemented in bridge.
        # We fall back to generate() but wrap it in an iterator for compatibility.
        res = self.generate(
            messages, 
            model=model, 
            temperature=temperature, 
            max_tokens=max_tokens, 
            **kwargs
        )
        content = res.get("content", "")
        # Yield in chunks to mimic streaming if possible, or just once
        yield content

    def list_models(self) -> List[str]:
        """List available models."""
        try:
            return self._inner.list_models()
        except Exception:
            return [self._model_path]

    def health(self) -> bool:
        """Check if engine is healthy."""
        try:
            return self._inner.health()
        except Exception:
            return False

__all__ = ["NativeEngine"]
