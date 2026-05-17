use ort::session::Session;
use ndarray::Array2;
use crate::AudioData;
use anyhow::Result;

/// High-performance TTS engine using Kokoro ONNX model.
pub struct KokoroEngine {
    session: parking_lot::Mutex<Session>,
    // TODO: Add phonemizer and voice embeddings
}

impl KokoroEngine {
    /// Load the Kokoro ONNX model and associated assets.
    pub fn new(model_path: &str) -> Result<Self> {
        let session = Session::builder()?
            .commit_from_file(model_path)?;
            
        Ok(Self {
            session: parking_lot::Mutex::new(session),
        })
    }

    /// Synthesize text to audio samples (24kHz f32).
    pub async fn synthesize(&self, _text: &str, _voice: &str) -> Result<AudioData> {
        // 1. Text to Phonemes
        let tokens: Vec<i64> = vec![0]; 

        // 2. Prepare Inputs
        let input_tensor = Array2::from_shape_vec((1, tokens.len()), tokens)?;
        let input_value = ort::value::Value::from_array(input_tensor)?;
        
        // 3. Run Inference
        let mut session = self.session.lock();
        let outputs = session.run(ort::inputs![
            "input" => input_value,
        ])?;
        
        let output = outputs["output"].try_extract_tensor::<f32>()?;
        let samples = output.1.to_vec();

        Ok(AudioData {
            samples,
            sample_rate: 24000,
        })
    }
}
