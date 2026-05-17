//! Agent management endpoints.

use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::json;

use crate::state::AppState;
use sunday_agents::{AgentRuntime, native_react::NativeReActAgent, OjAgent};
use std::sync::Arc;

pub async fn list_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    let agents = state.agent_runtime.list_agents();
    Json(json!({ "agents": agents }))
}

pub async fn create_handler(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("NativeAgent");
    
    // Creating an instance of NativeReActAgent
    let engine = state.engine.read().await.clone();
    let model = state.model.read().await.clone();
    
    // Bridge the engine to rig-core's CompletionModel
    let rig_model = sunday_engine::rig_adapter::RigModelAdapter::new(engine, model.clone());
    
    // Instantiate ReAct agent
    let native_agent = NativeReActAgent::new(
        rig_model, 
        state.tools.clone(),
        10, // max_turns
        0.7, // temperature
    );
    
    let agent_id = native_agent.agent_id().to_string();
    state.agent_runtime.register_agent(Arc::new(native_agent), name);
    
    Json(json!({
        "status": "created",
        "agent_id": agent_id,
        "name": name,
        "message": "Agent created natively via Axum"
    }))
}

pub async fn get_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if let Some(agent_state) = state.agent_runtime.get_state(&id) {
        Json(json!(agent_state))
    } else {
        Json(json!({ "error": "not found" }))
    }
}

pub async fn delete_handler(
    State(_state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    // Requires an unregister feature in AgentRuntime. Skipping actual unregister for now.
    Json(json!({
        "status": "stopped",
        "agent_id": id,
    }))
}

pub async fn message_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let message = body.get("content").or_else(|| body.get("message")).and_then(|v| v.as_str()).unwrap_or("");
    
    // Just trigger a tick in the background so the UI doesn't block
    let runtime = state.agent_runtime.clone();
    let msg = message.to_string();
    
    tokio::spawn(async move {
        // execute_tick handles the AgentTickStart / AgentTickEnd
        let _ = runtime.execute_tick(&id, &msg, None).await;
    });

    Json(json!({
        "status": "queued",
        "message": "Message sent to agent."
    }))
}

pub async fn state_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if let Some(agent_state) = state.agent_runtime.get_state(&id) {
        Json(json!(agent_state))
    } else {
        Json(json!({ "error": "not found" }))
    }
}

pub async fn get_messages_handler(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> Json<serde_json::Value> {
    Json(json!({
        "messages": []
    }))
}

pub async fn run_handler(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> Json<serde_json::Value> {
    Json(json!({
        "status": "started"
    }))
}
