//! Chat completions endpoint with SSE streaming.

use axum::{
    extract::State,
    response::{sse::Event, Sse},
    Json,
};
use serde::Deserialize;
use sunday_core::{Message, Role};
use sunday_engine::traits::messages_to_dicts;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::StreamExt;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<serde_json::Value>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i64,
}

fn default_temperature() -> f64 { 0.7 }
fn default_max_tokens() -> i64 { 2048 }

/// OpenAI-compatible chat completions endpoint.
pub async fn completions_handler(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let messages: Vec<Message> = req.messages.iter().filter_map(|m| {
        let role = m.get("role")?.as_str()?;
        let content = m.get("content")?.as_str()?;
        let role = match role {
            "system" => Role::System,
            "user" => Role::User,
            "assistant" => Role::Assistant,
            _ => Role::User,
        };
        Some(Message::new(role, content))
    }).collect();

    let model = if req.model.is_empty() { state.model.clone() } else { req.model };
    let engine = state.engine.clone();

    let stream = async_stream::stream! {
        match engine.stream(&messages, &model, req.temperature, req.max_tokens, None).await {
            Ok(token_stream) => {
                let mut stream = token_stream;
                let mut index = 0u32;

                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(text) => {
                            let event = Event::default()
                                .json_data(serde_json::json!({
                                    "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                                    "object": "chat.completion.chunk",
                                    "created": chrono::Utc::now().timestamp(),
                                    "model": &model,
                                    "choices": [{
                                        "index": index,
                                        "delta": { "content": text },
                                        "finish_reason": null
                                    }]
                                }))
                                .unwrap_or_else(|_| Event::default().data("{}"));
                            yield Ok(event);
                            index += 1;
                        }
                        Err(e) => {
                            let event = Event::default()
                                .event("error")
                                .data(format!("{{\"error\":\"{}\"}}", e));
                            yield Ok(event);
                            break;
                        }
                    }
                }

                // [DONE] sentinel
                yield Ok(Event::default().data("[DONE]"));
            }
            Err(e) => {
                let event = Event::default()
                    .event("error")
                    .data(format!("{{\"error\":\"{}\"}}", e));
                yield Ok(event);
            }
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text(""),
    )
}
