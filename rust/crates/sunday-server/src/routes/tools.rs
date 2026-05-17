//! Tool endpoints.

use axum::{
    extract::State,
    Json,
};
use serde_json::json;

use crate::state::AppState;

pub async fn list_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    // Assuming state.tools.list_tools() returns Vec<String>
    let tools_list = state.tools.list_tools();
    
    // The frontend usually expects a list of objects with name, description, etc.
    // For now we mock the full objects using the string names
    let tools_data: Vec<serde_json::Value> = tools_list.into_iter().map(|name| {
        json!({
            "name": name,
            "description": format!("Built-in tool: {}", name),
            "category": "Built-in",
            "is_auth_required": false
        })
    }).collect();

    Json(json!({
        "tools": tools_data
    }))
}
