//! Telemetry endpoints.

use axum::{
    extract::State,
    Json,
};
use serde_json::json;

use crate::state::AppState;

pub async fn stats_handler(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "total_requests": 0,
        "total_tokens": 0,
        "prompt_tokens": 0,
        "completion_tokens": 0
    }))
}

pub async fn energy_handler(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "total_energy_j": 0.0,
        "energy_per_token_j": 0.0,
        "avg_power_w": 0.0,
        "cpu_temp_c": null,
        "gpu_temp_c": null
    }))
}

pub async fn savings_handler(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "total_calls": 0,
        "total_prompt_tokens": 0,
        "total_completion_tokens": 0,
        "total_tokens": 0,
        "per_provider": [
            {
                "provider": "gpt-5.3",
                "total_cost": 0.0,
                "input_cost": 0.0,
                "output_cost": 0.0,
                "energy_joules": 0.0,
                "energy_wh": 0.0,
                "flops": 0.0
            },
            {
                "provider": "claude-opus-4.6",
                "total_cost": 0.0,
                "input_cost": 0.0,
                "output_cost": 0.0
            },
            {
                "provider": "gemini-3.1-pro",
                "total_cost": 0.0,
                "input_cost": 0.0,
                "output_cost": 0.0
            }
        ],
        "monthly_projection": {
            "gpt-5.3": 0.0,
            "claude-opus-4.6": 0.0,
            "gemini-3.1-pro": 0.0
        }
    }))
}
