//! Speech-to-text and text-to-speech endpoints.

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct SttRequest {
    pub audio_url: Option<String>,
    pub language: Option<String>,
}

#[derive(Deserialize)]
pub struct TtsRequest {
    pub text: String,
    pub voice: Option<String>,
    #[allow(dead_code)]
    pub speed: Option<f64>,
}

/// STT endpoint — transcribe audio to text.
pub async fn stt_handler(
    State(_state): State<AppState>,
    Json(req): Json<SttRequest>,
) -> Response {
    tracing::info!("STT request: audio_url={:?}, lang={:?}", req.audio_url, req.language);
    // TODO: Integrate with sunday-speech crate
    let body = serde_json::json!({
        "text": "",
        "status": "not_implemented"
    });
    (axum::http::StatusCode::NOT_IMPLEMENTED, Json(body)).into_response()
}

/// TTS endpoint — synthesize text to audio.
pub async fn tts_handler(
    State(_state): State<AppState>,
    Json(req): Json<TtsRequest>,
) -> Response {
    tracing::info!("TTS request: text_len={}, voice={:?}", req.text.len(), req.voice);
    // TODO: Integrate with sunday-speech crate
    let body = serde_json::json!({
        "audio_url": null,
        "status": "not_implemented"
    });
    (axum::http::StatusCode::NOT_IMPLEMENTED, Json(body)).into_response()
}
