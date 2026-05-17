//! Channel REST API — send messages and list configured channels.

use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use crate::state::AppState;
use sunday_channels::{Channel, ChannelError};

#[derive(Deserialize)]
pub struct SendRequest {
    pub recipient: String,
    pub text: String,
}

/// List configured channels.
pub async fn list_handler(State(_state): State<AppState>) -> Response {
    // TODO: Read from config or registry
    let body = serde_json::json!({
        "channels": [
            {"id": "telegram", "enabled": false},
            {"id": "slack", "enabled": false},
            {"id": "whatsapp", "enabled": false},
            {"id": "line", "enabled": false},
        ]
    });
    Json(body).into_response()
}

/// Send a message through a specific channel.
pub async fn send_handler(
    State(_state): State<AppState>,
    Path(channel): Path<String>,
    Json(req): Json<SendRequest>,
) -> Response {
    tracing::info!("Channel send: {} -> {}", channel, req.recipient);

    let result = match channel.as_str() {
        "telegram" => {
            if let Some(token) = std::env::var("TELEGRAM_BOT_TOKEN").ok().filter(|s| !s.is_empty()) {
                let ch = sunday_channels::telegram::TelegramChannel::new(token);
                ch.send_message(&req.recipient, &req.text).await
            } else {
                Err(ChannelError::Config("TELEGRAM_BOT_TOKEN not set".into()))
            }
        }
        "slack" => {
            let webhook = std::env::var("SLACK_WEBHOOK_URL").ok();
            let token = std::env::var("SLACK_BOT_TOKEN").ok();
            let ch = sunday_channels::slack::SlackChannel::new(webhook, token);
            ch.send_message(&req.recipient, &req.text).await
        }
        "whatsapp" => {
            if let (Some(token), Some(phone_id)) = (
                std::env::var("WHATSAPP_ACCESS_TOKEN").ok().filter(|s| !s.is_empty()),
                std::env::var("WHATSAPP_PHONE_NUMBER_ID").ok().filter(|s| !s.is_empty()),
            ) {
                let ch = sunday_channels::whatsapp::WhatsAppChannel::new(token, phone_id);
                ch.send_message(&req.recipient, &req.text).await
            } else {
                Err(ChannelError::Config("WHATSAPP_ACCESS_TOKEN or WHATSAPP_PHONE_NUMBER_ID not set".into()))
            }
        }
        "line" => {
            if let Some(token) = std::env::var("LINE_CHANNEL_ACCESS_TOKEN").ok().filter(|s| !s.is_empty()) {
                let ch = sunday_channels::line::LineChannel::new(token);
                ch.send_message(&req.recipient, &req.text).await
            } else {
                Err(ChannelError::Config("LINE_CHANNEL_ACCESS_TOKEN not set".into()))
            }
        }
        _ => Err(ChannelError::UnknownChannel(channel.clone())),
    };

    match result {
        Ok(()) => {
            let body = serde_json::json!({"status": "sent", "channel": channel});
            Json(body).into_response()
        }
        Err(e) => {
            let body = serde_json::json!({"error": e.to_string()});
            (axum::http::StatusCode::BAD_REQUEST, Json(body)).into_response()
        }
    }
}
