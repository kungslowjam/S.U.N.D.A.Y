"""Decorator-based registry for runtime discovery of pluggable components.

Adapted from IPW's ``src/ipw/core/registry.py``.  Each typed subclass gets its
own isolated storage so registrations in one registry never leak into another.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Callable, Dict, Generic, Tuple, Type, TypeVar

if TYPE_CHECKING:
    from sunday.agents._stubs import BaseAgent
    from sunday.engine._stubs import InferenceEngine
    from sunday.tools.storage._stubs import MemoryBackend

T = TypeVar("T")


class RegistryBase(Generic[T]):
    """Generic registry base class with class-specific entry isolation."""

    @classmethod
    def _entries(cls) -> Dict[str, T]:
        attr_name = f"_registry_entries_{cls.__name__}"
        storage = getattr(cls, attr_name, None)
        if storage is None:
            storage: Dict[str, T] = {}
            setattr(cls, attr_name, storage)
        return storage

    @classmethod
    def _lazy_map(cls) -> Dict[str, str]:
        attr_name = f"_registry_lazy_{cls.__name__}"
        l_map = getattr(cls, attr_name, None)
        if l_map is None:
            l_map: Dict[str, str] = {}
            setattr(cls, attr_name, l_map)
        return l_map

    @classmethod
    def register_lazy(cls, key: str, module_path: str) -> None:
        """Map a *key* to a *module_path* for lazy loading."""
        cls._lazy_map()[key] = module_path

    @classmethod
    def register(cls, key: str) -> Callable[[T], T]:
        """Decorator that registers *entry* under *key*."""

        def decorator(entry: T) -> T:
            entries = cls._entries()
            # If it was lazy, remove it now that it's loaded
            cls._lazy_map().pop(key, None)
            entries[key] = entry
            return entry

        return decorator

    @classmethod
    def register_value(cls, key: str, value: T) -> T:
        """Imperatively register a *value* under *key*."""
        entries = cls._entries()
        # If it was lazy, remove it now that it's loaded
        cls._lazy_map().pop(key, None)
        entries[key] = value
        return value

    @classmethod
    def create(cls, key: str, *args: Any, **kwargs: Any) -> Any:
        """Look up *key* and instantiate it with the given arguments."""
        entry = cls.get(key)
        if not callable(entry):
            raise TypeError(
                f"{cls.__name__} entry '{key}' is not callable"
                " and cannot be instantiated"
            )
        return entry(*args, **kwargs)

    @classmethod
    def _ensure_loaded(cls, key: str) -> None:
        """Trigger module import if *key* is currently lazy."""
        if key in cls._entries():
            return
        mod_path = cls._lazy_map().pop(key, None)
        if mod_path:
            import importlib
            try:
                importlib.import_module(mod_path)
            except ImportError:
                # If loading fails, it's fine, we just won't find the entry
                pass

    @classmethod
    def get(cls, key: str) -> T:
        """Retrieve the entry for *key*, raising ``KeyError`` if missing."""
        cls._ensure_loaded(key)
        try:
            return cls._entries()[key]
        except KeyError as exc:
            raise KeyError(
                f"{cls.__name__} does not have an entry for '{key}'"
            ) from exc

    @classmethod
    def contains(cls, key: str) -> bool:
        """Check whether *key* is registered (including lazy entries)."""
        return key in cls._entries() or key in cls._lazy_map()

    @classmethod
    def keys(cls) -> Tuple[str, ...]:
        """Return all registered keys (including lazy entries)."""
        return tuple(set(cls._entries().keys()) | set(cls._lazy_map().keys()))

    @classmethod
    def items(cls) -> Tuple[Tuple[str, T], ...]:
        """Return all ``(key, entry)`` pairs. Forces loading of all lazy entries."""
        for key in list(cls._lazy_map().keys()):
            cls._ensure_loaded(key)
        return tuple(cls._entries().items())

    @classmethod
    def clear(cls) -> None:
        """Remove all entries."""
        cls._entries().clear()
        cls._lazy_map().clear()


# ---------------------------------------------------------------------------
# Typed subclass registries — one per primitive
# ---------------------------------------------------------------------------


class ModelRegistry(RegistryBase[Any]):
    """Registry for ``ModelSpec`` objects."""


class EngineRegistry(RegistryBase[Type["InferenceEngine"]]):
    """Registry for inference engine backends."""


class MemoryRegistry(RegistryBase[Type["MemoryBackend"]]):
    """Registry for memory / retrieval backends."""


class AgentRegistry(RegistryBase[Type["BaseAgent"]]):
    """Registry for agent implementations."""


class ToolRegistry(RegistryBase[Any]):
    """Registry for tool specifications."""


class RouterPolicyRegistry(RegistryBase[Any]):
    """Registry for router policy implementations."""


class BenchmarkRegistry(RegistryBase[Any]):
    """Registry for benchmark implementations."""


class ChannelRegistry(RegistryBase[Any]):
    """Registry for channel implementations."""


class LearningRegistry(RegistryBase[Any]):
    """Registry for learning policies."""


class SkillRegistry(RegistryBase[Any]):
    """Registry for skill manifests."""


class SpeechRegistry(RegistryBase[Any]):
    """Registry for speech backend implementations."""


class CompressionRegistry(RegistryBase[Any]):
    """Registry for context compression strategies."""


class TTSRegistry(RegistryBase[Any]):
    """Registry for text-to-speech backend implementations."""


class ConnectorRegistry(RegistryBase[Any]):
    """Registry for data source connectors (Gmail, Slack, etc.)."""


class MinerRegistry(RegistryBase[Any]):
    """Registry for Pearl mining provider implementations.

    Each provider implements the ``MiningProvider`` ABC defined in
    ``sunday.mining._stubs``. Registry keys are short lowercase strings
    such as ``"vllm-pearl"`` (CUDA + Hopper) and (future) ``"mlx-pearl"``,
    ``"llamacpp-pearl-metal"``, ``"ollama-pearl"``.
    """


__all__ = [
    "AgentRegistry",
    "BenchmarkRegistry",
    "ChannelRegistry",
    "CompressionRegistry",
    "ConnectorRegistry",
    "EngineRegistry",
    "LearningRegistry",
    "MemoryRegistry",
    "MinerRegistry",
    "ModelRegistry",
    "RegistryBase",
    "RouterPolicyRegistry",
    "SkillRegistry",
    "SpeechRegistry",
    "TTSRegistry",
    "ToolRegistry",
]
