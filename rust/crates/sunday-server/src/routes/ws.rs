//! WebSocket handlers for chat streaming and agent events.

use axum::{
    extract::{State, WebSocketUpgrade},
    response::Response,
};

use crate::state::AppState;

/// WebSocket chat streaming endpoint.
pub async fn chat_stream_handler(
    ws: WebSocketUpgrade,
    State(_state): State<AppState>,
) -> Response {
    ws.on_upgrade(handle_chat_stream)
}

async fn handle_chat_stream(mut socket: axum::extract::ws::WebSocket) {
    use axum::extract::ws::Message;

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                // Parse JSON request and stream response
                let response = format!("Echo: {}", text);
                let _ = socket.send(Message::Text(response)).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}

/// WebSocket agent events endpoint.
pub async fn agent_events_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_agent_events(socket, state))
}

async fn handle_agent_events(mut socket: axum::extract::ws::WebSocket, state: AppState) {
    use axum::extract::ws::Message;

    let mut rx = state.bus.subscribe();

    loop {
        tokio::select! {
            Ok(event) = rx.recv() => {
                let payload = serde_json::json!({
                    "type": event.event_type.to_string(),
                    "timestamp": event.timestamp,
                    "data": event.data,
                });
                if socket.send(Message::Text(payload.to_string())).await.is_err() {
                    break;
                }
            }
            Some(Ok(Message::Close(_))) = socket.recv() => {
                break;
            }
        }
    }
}
