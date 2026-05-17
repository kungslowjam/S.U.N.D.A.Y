//! Webhook handlers for external messaging services.

use axum::{
    extract::{Query, State},
    response::Response,
    Json,
};
use serde::Deserialize;

use crate::state::AppState;

// ---------------------------------------------------------------------------
// Twilio
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct TwilioWebhook {
    #[serde(rename = "From")]
    pub from: String,
    #[serde(rename = "Body")]
    pub body: String,
}

pub async fn twilio_handler(
    State(_state): State<AppState>,
    Query(params): Query<TwilioWebhook>,
) -> Response {
    tracing::info!("Twilio message from {}: {}", params.from, params.body);
    // TODO: Route through channel bridge
    axum::response::IntoResponse::into_response("OK")
}

// ---------------------------------------------------------------------------
// WhatsApp
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct WhatsAppVerify {
    #[serde(rename = "hub.mode")]
    pub mode: String,
    #[serde(rename = "hub.verify_token")]
    #[allow(dead_code)]
    pub verify_token: String,
    #[serde(rename = "hub.challenge")]
    pub challenge: String,
}

pub async fn whatsapp_verify_handler(
    State(_state): State<AppState>,
    Query(params): Query<WhatsAppVerify>,
) -> Response {
    tracing::info!("WhatsApp verification: mode={}", params.mode);
    // TODO: Validate verify_token against config
    axum::response::IntoResponse::into_response(params.challenge)
}

pub async fn whatsapp_incoming_handler(
    State(_state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    tracing::info!("WhatsApp incoming: {}", body);
    // TODO: Route through channel bridge
    axum::response::IntoResponse::into_response("OK")
}

// ---------------------------------------------------------------------------
// BlueBubbles
// ---------------------------------------------------------------------------

pub async fn bluebubbles_handler(
    State(_state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    tracing::info!("BlueBubbles webhook: {}", body);
    // TODO: Route through channel bridge
    axum::response::IntoResponse::into_response("OK")
}

// ---------------------------------------------------------------------------
// LINE
// ---------------------------------------------------------------------------

pub async fn line_handler(
    State(_state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    tracing::info!("LINE webhook: {}", body);
    // TODO: Route through channel bridge
    axum::response::IntoResponse::into_response("OK")
}
