use serde::{Deserialize, Serialize};
use surrealdb::engine::local::Mem;
use surrealdb::Surreal;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: serde_json::Value,
    pub timestamp: i64,
}

pub struct VectorMemory {
    db: Arc<Surreal<surrealdb::engine::local::Db>>,
}

impl VectorMemory {
    pub async fn new_in_memory() -> Result<Self, Box<dyn std::error::Error>> {
        let db = Surreal::new::<Mem>(()).await?;
        db.use_ns("sunday").use_db("memory").await?;
        
        // Define table and vector index (SurrealDB 2.0+ support)
        db.query("DEFINE TABLE memory SCHEMAFULL;").await?;
        db.query("DEFINE FIELD embedding ON memory TYPE array<float>;").await?;
        
        Ok(Self { db: Arc::new(db) })
    }

    pub async fn add_lesson(&self, content: &str, embedding: Vec<f32>) -> Result<(), Box<dyn std::error::Error>> {
        let id = uuid::Uuid::new_v4().to_string();
        let entry = MemoryEntry {
            id: id.clone(),
            content: content.to_string(),
            embedding,
            metadata: serde_json::json!({"type": "lesson"}),
            timestamp: chrono::Utc::now().timestamp(),
        };

        let _: Option<MemoryEntry> = self.db.create(("memory", &id)).content(entry).await?;
        info!("Added lesson to SurrealDB memory: {}", id);
        Ok(())
    }

    pub async fn search_similar(&self, _embedding: Vec<f32>, limit: usize) -> Result<Vec<MemoryEntry>, Box<dyn std::error::Error>> {
        // Placeholder for vector search query in SurrealDB
        // SurrealDB uses `<|index|>` or similar for vector search in 2.x
        let mut response = self.db.query("SELECT * FROM memory LIMIT $limit")
            .bind(("limit", limit))
            .await?;
            
        let entries: Vec<MemoryEntry> = response.take(0)?;
        Ok(entries)
    }
}
