//! sunday-speech — high-performance, in-memory STT and TTS engines.

pub mod stt;
pub mod tts;

pub use stt::WhisperEngine;
pub use tts::KokoroEngine;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub segments: Vec<SpeechSegment>,
    pub language: String,
    pub probability: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}
