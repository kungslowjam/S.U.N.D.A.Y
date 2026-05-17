//! Memory endpoints — backed by sunday-memory (SQLite KG + Episodic).

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct StoreRequest {
    pub subject: String,
    #[serde(default = "default_subject_type")]
    pub subject_type: String,
    pub predicate: String,
    pub object_name: Option<String>,
    pub object_type: Option<String>,
    pub object_value: Option<String>,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    pub source: Option<String>,
}

fn default_subject_type() -> String { "concept".into() }
fn default_confidence() -> f64 { 0.9 }

#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize { 10 }

#[derive(Deserialize)]
pub struct RelatedRequest {
    pub entity: String,
    #[serde(default = "default_depth")]
    pub depth: usize,
}

fn default_depth() -> usize { 2 }

pub async fn store_handler(
    State(state): State<AppState>,
    Json(body): Json<StoreRequest>,
) -> Json<serde_json::Value> {
    match &state.memory {
        Some(memory) => match memory.add_fact(
            &body.subject,
            &body.subject_type,
            &body.predicate,
            body.object_name.as_deref(),
            body.object_type.as_deref(),
            body.object_value.as_deref(),
            body.confidence,
            body.source.as_deref(),
        ) {
            Ok(id) => Json(json!({"status": "stored", "fact_id": id })),
            Err(e) => Json(json!({"error": e })),
        },
        None => Json(json!({"error": "Memory not initialized"})),
    }
}

pub async fn search_handler(
    State(state): State<AppState>,
    Json(body): Json<SearchRequest>,
) -> Json<serde_json::Value> {
    match &state.memory {
        Some(memory) => match memory.search_facts(&body.query, body.limit) {
            Ok(facts) => Json(json!({"status": "ok", "facts": facts })),
            Err(e) => Json(json!({"error": e })),
        },
        None => Json(json!({"error": "Memory not initialized"})),
    }
}

pub async fn related_handler(
    State(state): State<AppState>,
    Json(body): Json<RelatedRequest>,
) -> Json<serde_json::Value> {
    match &state.memory {
        Some(memory) => match memory.query_related(&body.entity, body.depth) {
            Ok(entities) => Json(json!({"status": "ok", "related": entities })),
            Err(e) => Json(json!({"error": e })),
        },
        None => Json(json!({"error": "Memory not initialized"})),
    }
}

pub async fn stats_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    match &state.memory {
        Some(memory) => {
            let reflections = memory.get_reflections(5).unwrap_or_default();
            Json(json!({
                "status": "ok",
                "reflections": reflections,
                "note": "Full stats via SQL directly"
            }))
        }
        None => Json(json!({"error": "Memory not initialized"})),
    }
}

pub async fn get_facts_handler(
    State(state): State<AppState>,
    Path(entity): Path<String>,
) -> Json<serde_json::Value> {
    match &state.memory {
        Some(memory) => match memory.get_facts_about(&entity) {
            Ok(facts) => Json(json!({"status": "ok", "entity": entity, "facts": facts })),
            Err(e) => Json(json!({"error": e })),
        },
        None => Json(json!({"error": "Memory not initialized"})),
    }
}
