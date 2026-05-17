use rusqlite::{named_params, Connection, OptionalExtension, Result, Row};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub session_tag: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: i64,
    pub session_id: i64,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<String>,
    pub metadata: Option<String>,
    pub created_at: i64,
}

pub struct EpisodicMemory<'a> {
    conn: &'a mut Connection,
}

impl<'a> EpisodicMemory<'a> {
    pub fn new(conn: &'a mut Connection) -> Self {
        Self { conn }
    }

    pub fn create_session(&mut self, tag: &str, title: Option<&str>) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO sessions (session_tag, title) VALUES (:tag, :title)
             ON CONFLICT(session_tag) DO UPDATE SET updated_at = unixepoch()",
            named_params! { ":tag": tag, ":title": title },
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn add_message(
        &mut self,
        session_tag: &str,
        role: &str,
        content: &str,
        tool_calls: Option<&str>,
        metadata: Option<&str>,
    ) -> Result<i64> {
        let session_id = self.get_or_create_session_id(session_tag, None)?;
        self.conn.execute(
            "INSERT INTO messages (session_id, role, content, tool_calls, metadata)
             VALUES (:sid, :role, :content, :tc, :meta)",
            named_params! {
                ":sid": session_id,
                ":role": role,
                ":content": content,
                ":tc": tool_calls,
                ":meta": metadata,
            },
        )?;
        self.conn.execute(
            "UPDATE sessions SET updated_at = unixepoch() WHERE id = :id",
            named_params! { ":id": session_id },
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_session_messages(&self, session_tag: &str) -> Result<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT m.id, m.session_id, m.role, m.content, m.tool_calls, m.metadata, m.created_at
             FROM messages m
             JOIN sessions s ON s.id = m.session_id
             WHERE s.session_tag = :tag
             ORDER BY m.created_at ASC"
        )?;
        let rows = stmt.query_map(named_params! { ":tag": session_tag }, Self::map_message)?;
        rows.collect()
    }

    pub fn search_sessions(&self, keyword: &str, limit: usize) -> Result<Vec<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT s.id, s.session_tag, s.title, s.summary, s.created_at, s.updated_at
             FROM sessions s
             JOIN messages m ON m.session_id = s.id
             WHERE s.session_tag LIKE :q OR s.title LIKE :q OR m.content LIKE :q
             ORDER BY s.updated_at DESC
             LIMIT :lim"
        )?;
        let rows = stmt.query_map(
            named_params! { ":q": format!("%{}%", keyword), ":lim": limit as i64 },
            Self::map_session,
        )?;
        rows.collect()
    }

    pub fn update_session_summary(&mut self, session_tag: &str, summary: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET summary = :summary, updated_at = unixepoch()
             WHERE session_tag = :tag",
            named_params! { ":summary": summary, ":tag": session_tag },
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn get_or_create_session_id(&mut self, tag: &str, title: Option<&str>) -> Result<i64> {
        let existing: Option<i64> = {
            let mut stmt = self.conn.prepare(
                "SELECT id FROM sessions WHERE session_tag = :tag"
            )?;
            stmt.query_row(named_params! { ":tag": tag }, |row| row.get::<_, i64>(0)).optional()?
        };
        if let Some(id) = existing {
            Ok(id)
        } else {
            self.create_session(tag, title)
        }
    }

    fn map_session(row: &Row) -> Result<Session> {
        Ok(Session {
            id: row.get(0)?,
            session_tag: row.get(1)?,
            title: row.get(2)?,
            summary: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    }

    fn map_message(row: &Row) -> Result<Message> {
        Ok(Message {
            id: row.get(0)?,
            session_id: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            tool_calls: row.get(4)?,
            metadata: row.get(5)?,
            created_at: row.get(6)?,
        })
    }
}
