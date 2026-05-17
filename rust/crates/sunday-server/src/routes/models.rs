//! Model management endpoints.

use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::json;

use crate::state::AppState;

/// List available models.
pub async fn list_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut all_models = Vec::new();

    // 1. Get models from the active engine
    let engine = state.engine.read().await;
    if let Ok(engine_models) = engine.list_models() {
        all_models.extend(engine_models);
    }

    // 2. Scan local directories for GGUF models
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    let global_dir = std::path::PathBuf::from(home).join(".sunday").join("models");
    let local_dir = std::env::current_dir()
        .unwrap_or_default()
        .join("llama-cpp")
        .join("models");

    for dir in &[local_dir, global_dir] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("gguf") {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        if !all_models.contains(&name.to_string()) {
                            all_models.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    // Sort models: Active model first, then alphabetical
    let active_model = state.model.read().await.clone();
    all_models.sort_by(|a, b| {
        if a == &active_model { return std::cmp::Ordering::Less; }
        if b == &active_model { return std::cmp::Ordering::Greater; }
        a.cmp(b)
    });

    // Map to OpenAI-compatible ModelInfo shape so the frontend
    // (which expects `{ data: [{ id, object, created, owned_by }] }`)
    // can render the list correctly.
    let data_models: Vec<serde_json::Value> = all_models
        .into_iter()
        .map(|m| {
            json!({
                "id": m,
                "object": "model",
                "created": 0,
                "owned_by": "llamacpp"
            })
        })
        .collect();

    Json(json!({ "data": data_models }))
}

/// Pull a model (placeholder — would delegate to engine).
pub async fn pull_handler(State(_state): State<AppState>, Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let model_name = body.get("model").and_then(|m| m.as_str()).unwrap_or("");
    Json(json!({
        "status": "queued",
        "model": model_name,
        "message": "Model pull is not yet implemented in Rust server"
    }))
}

/// Switch active model and engine.
pub async fn switch_handler(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    use std::sync::Arc;
    use sunday_engine::{ollama::OllamaEngine, llamacpp::LlamaCppEngine, InferenceEngine};

    let model_name = body.get("model").and_then(|m| m.as_str()).unwrap_or("");
    if model_name.is_empty() {
        return Json(json!({ "error": "Model name is required" }));
    }

    let (new_engine, actual_model): (Arc<dyn InferenceEngine>, String) = if model_name.ends_with(".gguf") {
        // Look for GGUF in known paths
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();
        let global_dir = std::path::PathBuf::from(home).join(".sunday").join("models");
        let local_dir = std::env::current_dir().unwrap_or_default().join("llama-cpp").join("models");
        
        let mut path = None;
        for dir in &[local_dir, global_dir] {
            let p = dir.join(model_name);
            if p.exists() {
                path = Some(p);
                break;
            }
        }

        if let Some(p) = path {
            let engine = LlamaCppEngine::new(p.to_str().unwrap(), 8080, 120.0);
            (Arc::new(engine), model_name.to_string())
        } else {
            return Json(json!({ "error": format!("GGUF model not found: {}", model_name) }));
        }
    } else {
        // Fallback to Ollama
        let engine = OllamaEngine::new("http://127.0.0.1:11434", 30.0);
        (Arc::new(engine), model_name.to_string())
    };

    // Update state
    *state.engine.write().await = new_engine;
    *state.model.write().await = actual_model.clone();

    Json(json!({
        "status": "success",
        "model": actual_model,
        "engine": if model_name.ends_with(".gguf") { "llamacpp" } else { "ollama" }
    }))
}

/// Delete a model.
pub async fn delete_handler(
    State(_state): State<AppState>,
    Path(model_name): Path<String>,
) -> Json<serde_json::Value> {
    Json(json!({
        "status": "success",
        "model": model_name,
        "message": "Model deletion is not yet implemented in Rust server"
    }))
}
