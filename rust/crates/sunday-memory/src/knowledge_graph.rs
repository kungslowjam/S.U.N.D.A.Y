use rusqlite::{named_params, Connection, OptionalExtension, Result, Row};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::schema::init_schema;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: i64,
    pub name: String,
    pub entity_type: String,
    pub description: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub id: i64,
    pub subject_id: i64,
    pub subject_name: String,
    pub predicate: String,
    pub object_id: Option<i64>,
    pub object_name: Option<String>,
    pub object_value: Option<String>,
    pub confidence: f64,
    pub trust: f64,
    pub source: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub id: i64,
    pub insight: String,
    pub derived_from: Option<String>,
    pub confidence: f64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedEntity {
    pub entity_id: i64,
    pub name: String,
    pub entity_type: String,
    pub relationship: String,
    pub depth: usize,
    pub trust: f64,
}

// ---------------------------------------------------------------------------
// KnowledgeGraph
// ---------------------------------------------------------------------------

pub struct KnowledgeGraph {
    conn: Connection,
}

impl KnowledgeGraph {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        init_schema(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        init_schema(&mut conn)?;
        Ok(Self { conn })
    }

    // -----------------------------------------------------------------------
    // Entity CRUD
    // -----------------------------------------------------------------------

    pub fn add_entity(&mut self, name: &str, entity_type: &str, description: Option<&str>) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO entities (name, entity_type, description)
             VALUES (:name, :etype, :desc)
             ON CONFLICT(name, entity_type) DO UPDATE SET description = excluded.description",
            named_params! {
                ":name": name,
                ":etype": entity_type,
                ":desc": description,
            },
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_entity_by_name(&self, name: &str, entity_type: Option<&str>) -> Result<Option<Entity>> {
        let sql = if entity_type.is_some() {
            "SELECT id, name, entity_type, description, created_at FROM entities
             WHERE name = :name AND entity_type = :etype LIMIT 1"
        } else {
            "SELECT id, name, entity_type, description, created_at FROM entities
             WHERE name = :name LIMIT 1"
        };
        let mut stmt = self.conn.prepare(sql)?;
        let row = if let Some(et) = entity_type {
            stmt.query_row(named_params! { ":name": name, ":etype": et }, Self::map_entity).optional()?
        } else {
            stmt.query_row(named_params! { ":name": name }, Self::map_entity).optional()?
        };
        Ok(row)
    }

    pub fn upsert_entity(&mut self, name: &str, entity_type: &str, description: Option<&str>) -> Result<i64> {
        if let Some(e) = self.get_entity_by_name(name, Some(entity_type))? {
            // Update description if provided
            if description.is_some() {
                self.conn.execute(
                    "UPDATE entities SET description = :desc WHERE id = :id",
                    named_params! { ":desc": description, ":id": e.id },
                )?;
            }
            Ok(e.id)
        } else {
            self.add_entity(name, entity_type, description)
        }
    }

    // -----------------------------------------------------------------------
    // Fact CRUD
    // -----------------------------------------------------------------------

    /// Add a fact. If subject/object names are provided they are resolved (or created) into entities.
    pub fn add_fact(
        &mut self,
        subject_name: &str,
        subject_type: &str,
        predicate: &str,
        object_name: Option<&str>,
        object_type: Option<&str>,
        object_value: Option<&str>,
        confidence: f64,
        source: Option<&str>,
    ) -> Result<i64> {
        let tx = self.conn.transaction()?;

        let subject_id = {
            let mut stmt = tx.prepare(
                "INSERT INTO entities (name, entity_type) VALUES (:name, :etype)
                 ON CONFLICT(name, entity_type) DO UPDATE SET name = excluded.name
                 RETURNING id"
            )?;
            stmt.query_row(
                named_params! { ":name": subject_name, ":etype": subject_type },
                |row| row.get::<_, i64>(0),
            )?
        };

        let object_id: Option<i64> = if let (Some(on), Some(ot)) = (object_name, object_type) {
            let mut stmt = tx.prepare(
                "INSERT INTO entities (name, entity_type) VALUES (:name, :etype)
                 ON CONFLICT(name, entity_type) DO UPDATE SET name = excluded.name
                 RETURNING id"
            )?;
            let oid = stmt.query_row(
                named_params! { ":name": on, ":etype": ot },
                |row| row.get::<_, i64>(0),
            )?;
            Some(oid)
        } else {
            None
        };

        tx.execute(
            "INSERT INTO facts (subject_id, predicate, object_id, object_value, confidence, source)
             VALUES (:sid, :pred, :oid, :oval, :conf, :src)
             ON CONFLICT DO NOTHING",
            named_params! {
                ":sid": subject_id,
                ":pred": predicate,
                ":oid": object_id,
                ":oval": object_value,
                ":conf": confidence,
                ":src": source,
            },
        )?;

        let fact_id = tx.last_insert_rowid();

        // Sync FTS content
        let content = format!("{} {} {}", subject_name, predicate, object_name.unwrap_or(object_value.unwrap_or("")));
        tx.execute(
            "INSERT INTO facts_fts_content (id, content) VALUES (:id, :content)
             ON CONFLICT(id) DO UPDATE SET content = excluded.content",
            named_params! { ":id": fact_id, ":content": content },
        )?;

        tx.commit()?;
        Ok(fact_id)
    }

    pub fn get_facts_about(&self, entity_name: &str, predicate_filter: Option<&str>) -> Result<Vec<Fact>> {
        let sql = if predicate_filter.is_some() {
            "SELECT f.id, f.subject_id, s.name, f.predicate, f.object_id, o.name, f.object_value,
                    f.confidence, f.trust, f.source, f.created_at
             FROM facts f
             JOIN entities s ON s.id = f.subject_id
             LEFT JOIN entities o ON o.id = f.object_id
             WHERE s.name = :ename AND f.predicate = :pred
             ORDER BY f.trust DESC, f.created_at DESC"
        } else {
            "SELECT f.id, f.subject_id, s.name, f.predicate, f.object_id, o.name, f.object_value,
                    f.confidence, f.trust, f.source, f.created_at
             FROM facts f
             JOIN entities s ON s.id = f.subject_id
             LEFT JOIN entities o ON o.id = f.object_id
             WHERE s.name = :ename
             ORDER BY f.trust DESC, f.created_at DESC"
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = if let Some(p) = predicate_filter {
            stmt.query_map(named_params! { ":ename": entity_name, ":pred": p }, Self::map_fact)?
        } else {
            stmt.query_map(named_params! { ":ename": entity_name }, Self::map_fact)?
        };
        rows.collect()
    }

    // -----------------------------------------------------------------------
    // Graph Traversal (Recursive CTE)
    // -----------------------------------------------------------------------

    /// Traverse the knowledge graph starting from an entity, up to `max_depth` hops.
    pub fn query_related(&self, entity_name: &str, max_depth: usize) -> Result<Vec<RelatedEntity>> {
        let mut stmt = self.conn.prepare(
            "WITH RECURSIVE traversal(entity_id, name, entity_type, relationship, depth, trust) AS (
                -- Anchor: facts where entity is subject
                SELECT s.id, s.name, s.entity_type, f.predicate, 0, f.trust
                FROM entities s
                JOIN facts f ON f.subject_id = s.id
                WHERE s.name = :ename

                UNION

                -- Anchor: facts where entity is object
                SELECT o.id, o.name, o.entity_type, f.predicate, 0, f.trust
                FROM entities o
                JOIN facts f ON f.object_id = o.id
                WHERE o.name = :ename

                UNION ALL

                -- Recursive step: follow facts forward from current entity
                SELECT e.id, e.name, e.entity_type, f.predicate, t.depth + 1, f.trust
                FROM traversal t
                JOIN facts f ON f.subject_id = t.entity_id
                JOIN entities e ON e.id = f.object_id
                WHERE t.depth < :maxd AND f.object_id IS NOT NULL

                UNION ALL

                -- Recursive step: follow facts backward to subject
                SELECT e.id, e.name, e.entity_type, f.predicate, t.depth + 1, f.trust
                FROM traversal t
                JOIN facts f ON f.object_id = t.entity_id
                JOIN entities e ON e.id = f.subject_id
                WHERE t.depth < :maxd
            )
            SELECT entity_id, name, entity_type, relationship, depth, AVG(trust) as avg_trust
            FROM traversal
            WHERE name != :ename
            GROUP BY entity_id, name, entity_type, relationship, depth
            ORDER BY depth, avg_trust DESC"
        )?;

        let rows = stmt.query_map(
            named_params! { ":ename": entity_name, ":maxd": max_depth as i64 },
            |row| {
                Ok(RelatedEntity {
                    entity_id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    relationship: row.get(3)?,
                    depth: row.get::<_, i64>(4)? as usize,
                    trust: row.get(5)?,
                })
            },
        )?;
        rows.collect()
    }

    // -----------------------------------------------------------------------
    // Search
    // -----------------------------------------------------------------------

    pub fn search_facts(&self, query: &str, limit: usize) -> Result<Vec<Fact>> {
        let mut stmt = self.conn.prepare(
            "SELECT f.id, f.subject_id, s.name, f.predicate, f.object_id, o.name, f.object_value,
                    f.confidence, f.trust, f.source, f.created_at
             FROM facts_fts_content c
             JOIN facts f ON f.id = c.id
             JOIN entities s ON s.id = f.subject_id
             LEFT JOIN entities o ON o.id = f.object_id
             WHERE facts_fts_content MATCH :q
             ORDER BY rank
             LIMIT :lim"
        )?;
        let rows = stmt.query_map(
            named_params! { ":q": query, ":lim": limit as i64 },
            Self::map_fact,
        )?;
        rows.collect()
    }

    // -----------------------------------------------------------------------
    // Trust Scoring
    // -----------------------------------------------------------------------

    pub fn update_trust(&mut self, fact_id: i64, delta: f64) -> Result<()> {
        self.conn.execute(
            "UPDATE facts SET trust = max(0.0, min(1.0, trust + :delta)) WHERE id = :id",
            named_params! { ":delta": delta, ":id": fact_id },
        )?;
        Ok(())
    }

    pub fn vote_fact(&mut self, fact_id: i64, helpful: bool) -> Result<()> {
        let delta = if helpful { 0.05 } else { -0.10 };
        self.update_trust(fact_id, delta)
    }

    // -----------------------------------------------------------------------
    // Reflection / Synthesis
    // -----------------------------------------------------------------------

    /// Find high-confidence patterns and store them as reflections.
    /// Simplified: finds entities with many facts and stores a summary.
    pub fn reflect(&mut self) -> Result<Vec<Reflection>> {
        let mut reflections = Vec::new();

        // Pattern 1: Entities with many high-trust facts → "important entity"
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.name, s.entity_type, COUNT(*) as cnt, AVG(f.trust) as avg_trust
             FROM facts f
             JOIN entities s ON s.id = f.subject_id
             GROUP BY s.id
             HAVING cnt >= 3 AND avg_trust >= 0.7
             ORDER BY cnt DESC"
        )?;
        let candidates = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, f64>(4)?,
            ))
        })?;

        for candidate in candidates {
            let (eid, name, etype, cnt, trust) = candidate?;
            let insight = format!(
                "{} ({}) is a well-documented entity with {} high-trust facts (avg trust: {:.2}).",
                name, etype, cnt, trust
            );

            self.conn.execute(
                "INSERT INTO reflections (insight, derived_from, confidence)
                 VALUES (:insight, :from, :conf)
                 ON CONFLICT DO NOTHING",
                named_params! {
                    ":insight": &insight,
                    ":from": &format!("entity:{}", eid),
                    ":conf": trust,
                },
            )?;

            reflections.push(Reflection {
                id: self.conn.last_insert_rowid(),
                insight,
                derived_from: Some(format!("entity:{}", eid)),
                confidence: trust,
                created_at: chrono::Utc::now().timestamp(),
            });
        }

        Ok(reflections)
    }

    pub fn get_reflections(&self, limit: usize) -> Result<Vec<Reflection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, insight, derived_from, confidence, created_at
             FROM reflections
             ORDER BY confidence DESC, created_at DESC
             LIMIT :lim"
        )?;
        let rows = stmt.query_map(named_params! { ":lim": limit as i64 }, |row| {
            Ok(Reflection {
                id: row.get(0)?,
                insight: row.get(1)?,
                derived_from: row.get(2)?,
                confidence: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        rows.collect()
    }

    pub fn conn(&mut self) -> &mut Connection {
        &mut self.conn
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn map_entity(row: &Row) -> Result<Entity> {
        Ok(Entity {
            id: row.get(0)?,
            name: row.get(1)?,
            entity_type: row.get(2)?,
            description: row.get(3)?,
            created_at: row.get(4)?,
        })
    }

    fn map_fact(row: &Row) -> Result<Fact> {
        Ok(Fact {
            id: row.get(0)?,
            subject_id: row.get(1)?,
            subject_name: row.get(2)?,
            predicate: row.get(3)?,
            object_id: row.get(4)?,
            object_name: row.get(5)?,
            object_value: row.get(6)?,
            confidence: row.get(7)?,
            trust: row.get(8)?,
            source: row.get(9)?,
            created_at: row.get(10)?,
        })
    }
}
