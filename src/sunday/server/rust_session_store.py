"""Rust-backed session store for channel conversations."""

from __future__ import annotations

import json
import logging
from typing import Any, Dict, List, Optional
from sunday._rust_bridge import get_rust_module

logger = logging.getLogger(__name__)

class RustSessionStore:
    """Manages per-sender, per-channel conversation sessions using Rust backend.

    Each session tracks conversation history, notification preferences,
    and pending long responses.
    """

    def __init__(self, db_path: str = "") -> None:
        mod = get_rust_module()
        # Fallback to default path if none provided
        if not db_path:
            from pathlib import Path
            db_path = str(Path.home() / ".sunday" / "sessions_v2.db")
        
        self.inner = mod.SessionStore(db_path)

    def get_or_create(self, sender_id: str, channel_type: str) -> Dict[str, Any]:
        # Rust SessionStore expects (user_id, channel, channel_user_id, display_name)
        # We map sender_id to user_id and channel_user_id for now.
        session_json = self.inner.get_or_create(
            sender_id, channel_type, sender_id, sender_id
        )
        data = json.loads(session_json)
        
        # Map Rust schema to the expected Python Dict schema
        return {
            "sender_id": sender_id,
            "channel_type": channel_type,
            "conversation_history": [
                {"role": m["role"], "content": m["content"]} 
                for m in data.get("messages", [])
            ],
            "preferred_notification_channel": data.get("metadata", {}).get("pref_notify"),
            "pending_response": data.get("metadata", {}).get("pending_res"),
        }

    def append_message(
        self,
        sender_id: str,
        channel_type: str,
        role: str,
        content: str,
    ) -> None:
        # First get session_id
        session_json = self.inner.get_or_create(
            sender_id, channel_type, sender_id, sender_id
        )
        session_data = json.loads(session_json)
        session_id = session_data["session_id"]
        
        self.inner.save_message(session_id, role, content, channel_type)
        # Auto-consolidate if history gets too long
        self.inner.consolidate(session_id)

    def set_notification_preference(
        self,
        sender_id: str,
        channel_type: str,
        preferred: str,
    ) -> None:
        session_json = self.inner.get_or_create(
            sender_id, channel_type, sender_id, sender_id
        )
        session_id = json.loads(session_json)["session_id"]
        self.inner.set_metadata_key(session_id, "pref_notify", json.dumps(preferred))

    def set_pending_response(
        self,
        sender_id: str,
        channel_type: str,
        response: str,
    ) -> None:
        session_json = self.inner.get_or_create(
            sender_id, channel_type, sender_id, sender_id
        )
        session_id = json.loads(session_json)["session_id"]
        self.inner.set_metadata_key(session_id, "pending_res", json.dumps(response))

    def clear_pending_response(self, sender_id: str, channel_type: str) -> None:
        session_json = self.inner.get_or_create(
            sender_id, channel_type, sender_id, sender_id
        )
        session_id = json.loads(session_json)["session_id"]
        self.inner.set_metadata_key(session_id, "pending_res", "null")

    def expire_sessions(self, max_age_hours: int = 24) -> int:
        return self.inner.decay(float(max_age_hours))

    def close(self) -> None:
        # Rust close is handled by Drop or explicit close if exposed
        pass
