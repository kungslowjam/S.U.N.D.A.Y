//! Memory manager — thin wrapper around sunday-memory's KnowledgeGraph.
//!
//! Uses a std::sync::Mutex because rusqlite operations are synchronous.

use std::sync::Mutex;
use sunday_memory::{KnowledgeGraph, Fact, Reflection, RelatedEntity};

pub struct MemoryManager {
    graph: Mutex<KnowledgeGraph>,
}

impl MemoryManager {
    pub fn new(db_path: &str) -> Result<Self, String> {
        let kg = KnowledgeGraph::open(db_path).map_err(|e| e.to_string())?;
        Ok(Self {
            graph: Mutex::new(kg),
        })
    }

    // ── Episodic ──

    pub fn add_message(&self, session_tag: &str, role: &str, content: &str) -> Result<(), String> {
        let mut kg = self.graph.lock().map_err(|e| e.to_string())?;
        use sunday_memory::EpisodicMemory;
        let mut conn = kg.conn();
        let mut em = EpisodicMemory::new(&mut conn);
        em.add_message(session_tag, role, content, None, None)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_session_messages(&self, session_tag: &str) -> Result<Vec<sunday_memory::Message>, String> {
        let mut kg = self.graph.lock().map_err(|e| e.to_string())?;
        use sunday_memory::EpisodicMemory;
        let mut conn = kg.conn();
        let em = EpisodicMemory::new(&mut conn);
        em.get_session_messages(session_tag).map_err(|e| e.to_string())
    }

    // ── Graph / Facts ──

    pub fn add_fact(
        &self,
        subject: &str,
        subject_type: &str,
        predicate: &str,
        object_name: Option<&str>,
        object_type: Option<&str>,
        object_value: Option<&str>,
        confidence: f64,
        source: Option<&str>,
    ) -> Result<i64, String> {
        let mut kg = self.graph.lock().map_err(|e| e.to_string())?;
        kg.add_fact(subject, subject_type, predicate, object_name, object_type, object_value, confidence, source)
            .map_err(|e| e.to_string())
    }

    pub fn search_facts(&self, query: &str, limit: usize) -> Result<Vec<Fact>, String> {
        let kg = self.graph.lock().map_err(|e| e.to_string())?;
        kg.search_facts(query, limit).map_err(|e| e.to_string())
    }

    pub fn query_related(&self, entity_name: &str, max_depth: usize) -> Result<Vec<RelatedEntity>, String> {
        let kg = self.graph.lock().map_err(|e| e.to_string())?;
        kg.query_related(entity_name, max_depth).map_err(|e| e.to_string())
    }

    pub fn get_facts_about(&self, entity_name: &str) -> Result<Vec<Fact>, String> {
        let kg = self.graph.lock().map_err(|e| e.to_string())?;
        kg.get_facts_about(entity_name, None).map_err(|e| e.to_string())
    }

    pub fn upsert_entity(&self, name: &str, entity_type: &str, description: Option<&str>) -> Result<i64, String> {
        let mut kg = self.graph.lock().map_err(|e| e.to_string())?;
        kg.upsert_entity(name, entity_type, description).map_err(|e| e.to_string())
    }

    pub fn vote_fact(&self, fact_id: i64, helpful: bool) -> Result<(), String> {
        let mut kg = self.graph.lock().map_err(|e| e.to_string())?;
        kg.vote_fact(fact_id, helpful).map_err(|e| e.to_string())
    }

    pub fn reflect(&self) -> Result<Vec<Reflection>, String> {
        let mut kg = self.graph.lock().map_err(|e| e.to_string())?;
        kg.reflect().map_err(|e| e.to_string())
    }

    pub fn get_reflections(&self, limit: usize) -> Result<Vec<Reflection>, String> {
        let kg = self.graph.lock().map_err(|e| e.to_string())?;
        kg.get_reflections(limit).map_err(|e| e.to_string())
    }
}
