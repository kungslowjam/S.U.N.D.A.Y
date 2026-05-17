use rusqlite::{Connection, Result};

pub fn init_schema(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;

    // ── Episodic Memory ──
    tx.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            session_tag TEXT NOT NULL UNIQUE,
            title       TEXT,
            summary     TEXT,
            created_at  INTEGER DEFAULT (unixepoch()),
            updated_at  INTEGER DEFAULT (unixepoch())
        ) STRICT;",
        [],
    )?;

    tx.execute(
        "CREATE TABLE IF NOT EXISTS messages (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
            role       TEXT NOT NULL CHECK(role IN ('user','assistant','system','tool')),
            content    TEXT NOT NULL,
            tool_calls TEXT,          -- JSON array of tool invocations
            metadata   TEXT,          -- JSON extra
            created_at INTEGER DEFAULT (unixepoch())
        ) STRICT;",
        [],
    )?;

    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, created_at);",
        [],
    )?;

    // ── Knowledge Graph ──
    tx.execute(
        "CREATE TABLE IF NOT EXISTS entities (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL COLLATE NOCASE,
            entity_type TEXT NOT NULL DEFAULT 'concept',
            description TEXT,
            embedding   BLOB,           -- optional raw f32 vector
            created_at  INTEGER DEFAULT (unixepoch())
        ) STRICT;",
        [],
    )?;

    tx.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_entities_name_type ON entities(name, entity_type);",
        [],
    )?;

    tx.execute(
        "CREATE TABLE IF NOT EXISTS facts (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            subject_id  INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
            predicate   TEXT NOT NULL,
            object_id   INTEGER REFERENCES entities(id) ON DELETE CASCADE,
            object_value TEXT,          -- literal value when object is not an entity
            confidence  REAL NOT NULL DEFAULT 1.0 CHECK(confidence BETWEEN 0.0 AND 1.0),
            trust       REAL NOT NULL DEFAULT 0.5 CHECK(trust BETWEEN 0.0 AND 1.0),
            source      TEXT,           -- tool name / message id / session_tag
            session_id  INTEGER REFERENCES sessions(id) ON DELETE SET NULL,
            created_at  INTEGER DEFAULT (unixepoch())
        ) STRICT;",
        [],
    )?;

    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_facts_subject ON facts(subject_id, predicate);",
        [],
    )?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_facts_object ON facts(object_id, predicate);",
        [],
    )?;

    // ── FTS5 for full-text search over facts (literal values + entity descriptions) ──
    tx.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS facts_fts USING fts5(
            content,
            content_rowid = id,
            content_table = facts_fts_content
        );",
        [],
    )?;

    tx.execute(
        "CREATE TABLE IF NOT EXISTS facts_fts_content (
            id INTEGER PRIMARY KEY,
            content TEXT
        );",
        [],
    )?;

    // Triggers to keep FTS index in sync (manual via KnowledgeGraph insert/update)

    // ── Reflection / Synthesis log ──
    tx.execute(
        "CREATE TABLE IF NOT EXISTS reflections (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            insight     TEXT NOT NULL,
            derived_from TEXT,          -- JSON array of fact ids
            confidence  REAL DEFAULT 0.8,
            created_at  INTEGER DEFAULT (unixepoch())
        ) STRICT;",
        [],
    )?;

    tx.commit()?;
    Ok(())
}
