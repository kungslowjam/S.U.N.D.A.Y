"""Lightweight pure-Python lesson store for Memory Consolidation.

Uses stdlib sqlite3 directly — no Rust backend, no external dependencies.
Designed to store and retrieve compact 'lessons' that the Orchestrator
extracts after each task, so the agent can learn from past experiences.
"""

from __future__ import annotations

import json
import sqlite3
import time
import uuid
from pathlib import Path
from typing import Any, Dict, List, Optional


# Default DB location alongside the main memory.db
_DEFAULT_DB = Path.home() / ".sunday" / "lessons.db"


class Lesson:
    """A single lesson entry."""

    __slots__ = ("doc_id", "content", "source", "metadata", "created_at")

    def __init__(
        self,
        doc_id: str,
        content: str,
        source: str = "",
        metadata: Optional[Dict[str, Any]] = None,
        created_at: float = 0.0,
    ) -> None:
        self.doc_id = doc_id
        self.content = content
        self.source = source
        self.metadata = metadata or {}
        self.created_at = created_at or time.time()


class LessonStore:
    """Pure-Python SQLite store for agent lessons.

    Provides two core operations:
    * ``store(content)`` — persist a lesson
    * ``search(query, top_k)`` — FTS5 full-text search (falls back to LIKE)
    """

    def __init__(self, db_path: str | Path = "") -> None:
        self._db_path = str(db_path or _DEFAULT_DB)
        Path(self._db_path).parent.mkdir(parents=True, exist_ok=True)
        self._conn = sqlite3.connect(self._db_path)
        self._conn.execute("PRAGMA journal_mode=WAL")
        self._create_tables()

    # ------------------------------------------------------------------
    # Schema
    # ------------------------------------------------------------------

    def _create_tables(self) -> None:
        self._conn.executescript("""
            CREATE TABLE IF NOT EXISTS lessons (
                id         TEXT PRIMARY KEY,
                content    TEXT NOT NULL,
                source     TEXT NOT NULL DEFAULT '',
                metadata   TEXT NOT NULL DEFAULT '{}',
                created_at REAL NOT NULL
            );
        """)
        # Try to create FTS5 virtual table; silently skip if FTS5 is not
        # compiled into the SQLite build.
        try:
            self._conn.execute("""
                CREATE VIRTUAL TABLE IF NOT EXISTS lessons_fts
                USING fts5(content, source, content='lessons', content_rowid='rowid')
            """)
            self._has_fts = True
        except sqlite3.OperationalError:
            self._has_fts = False
        self._conn.commit()

    # ------------------------------------------------------------------
    # Store
    # ------------------------------------------------------------------

    def store(
        self,
        content: str,
        *,
        source: str = "",
        metadata: Optional[Dict[str, Any]] = None,
    ) -> str:
        """Persist a lesson and return its unique id."""
        doc_id = uuid.uuid4().hex[:12]
        now = time.time()
        meta_json = json.dumps(metadata) if metadata else "{}"
        self._conn.execute(
            "INSERT INTO lessons (id, content, source, metadata, created_at) "
            "VALUES (?, ?, ?, ?, ?)",
            (doc_id, content, source, meta_json, now),
        )
        if self._has_fts:
            try:
                self._conn.execute(
                    "INSERT INTO lessons_fts (rowid, content, source) "
                    "VALUES (last_insert_rowid(), ?, ?)",
                    (content, source),
                )
            except sqlite3.OperationalError:
                pass  # FTS sync failure is non-critical
        self._conn.commit()
        return doc_id

    # ------------------------------------------------------------------
    # Search / Retrieve
    # ------------------------------------------------------------------

    def search(self, query: str, top_k: int = 3) -> List[Lesson]:
        """Search lessons by keyword (FTS5 if available, LIKE fallback)."""
        if not query.strip():
            return []

        rows: list = []
        if self._has_fts:
            try:
                # FTS5 MATCH query — wrap each word in quotes for safety
                terms = " OR ".join(
                    f'"{w}"' for w in query.split() if len(w) > 1
                )
                if terms:
                    rows = self._conn.execute(
                        "SELECT l.id, l.content, l.source, l.metadata, l.created_at "
                        "FROM lessons_fts f "
                        "JOIN lessons l ON l.rowid = f.rowid "
                        f"WHERE lessons_fts MATCH ? "
                        "ORDER BY rank LIMIT ?",
                        (terms, top_k),
                    ).fetchall()
            except sqlite3.OperationalError:
                rows = []

        # Fallback: LIKE search
        if not rows:
            keywords = [f"%{w}%" for w in query.split()[:3] if len(w) > 1]
            if keywords:
                where_clauses = " OR ".join(
                    "content LIKE ?" for _ in keywords
                )
                rows = self._conn.execute(
                    f"SELECT id, content, source, metadata, created_at "
                    f"FROM lessons WHERE {where_clauses} "
                    f"ORDER BY created_at DESC LIMIT ?",
                    (*keywords, top_k),
                ).fetchall()

        return [
            Lesson(
                doc_id=r[0],
                content=r[1],
                source=r[2],
                metadata=json.loads(r[3]) if r[3] else {},
                created_at=r[4],
            )
            for r in rows
        ]

    def count(self) -> int:
        """Return total number of stored lessons."""
        return self._conn.execute("SELECT COUNT(*) FROM lessons").fetchone()[0]

    def close(self) -> None:
        """Close the database connection."""
        self._conn.close()


# Module-level singleton for easy access
_instance: Optional[LessonStore] = None


def get_lesson_store() -> LessonStore:
    """Get or create the global LessonStore singleton."""
    global _instance
    if _instance is None:
        _instance = LessonStore()
    return _instance


__all__ = ["Lesson", "LessonStore", "get_lesson_store"]
