//! Embedding generation backends for semantic memory retrieval.

use sunday_core::SUNDAYError;
use sunday_core::error::EngineError;
use serde_json::Value;

/// Trait for text embedding models.
pub trait Embedder: Send + Sync {
    /// Embed a single text into a dense vector.
    fn embed(&self, text: &str) -> Result<Vec<f32>, SUNDAYError>;

    /// Embed multiple texts (batch). Default implementation calls `embed` sequentially.
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SUNDAYError> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// Vector dimensionality.
    fn dim(&self) -> usize;
}

// ---------------------------------------------------------------------------
// Ollama Embedder
// ---------------------------------------------------------------------------

/// Embedder that calls Ollama's `/api/embed` endpoint.
pub struct OllamaEmbedder {
    client: reqwest::blocking::Client,
    base_url: String,
    model: String,
    dim: usize,
}

impl OllamaEmbedder {
    pub fn new(base_url: &str, model: &str, dim: usize) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            dim,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new("http://localhost:11434", "nomic-embed-text", 768)
    }
}

impl Embedder for OllamaEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>, SUNDAYError> {
        if text.is_empty() {
            return Ok(vec![0.0f32; self.dim]);
        }

        let url = format!("{}/api/embed", self.base_url);
        let body = serde_json::json!({
            "model": self.model,
            "input": text,
        });

        let response = self.client
            .post(&url)
            .json(&body)
            .send()
            .map_err(|e| SUNDAYError::Engine(EngineError::Connection(format!("Ollama embed request failed: {}", e))))?;

        if !response.status().is_success() {
            return Err(SUNDAYError::Engine(EngineError::Http(
                format!("Ollama embed returned status: {}", response.status())
            )));
        }

        let json: Value = response.json().map_err(|e| {
            SUNDAYError::Engine(EngineError::Deserialization(format!("Failed to parse Ollama embed response: {}", e)))
        })?;

        // Ollama /api/embed returns { "embeddings": [[...]] }
        let embeddings = json.get("embeddings")
            .and_then(|e| e.as_array())
            .ok_or_else(|| SUNDAYError::Engine(EngineError::Generation("Missing embeddings field in Ollama response".into())))?;

        if embeddings.is_empty() {
            return Ok(vec![0.0f32; self.dim]);
        }

        let first = &embeddings[0];
        let vec: Vec<f32> = first.as_array()
            .ok_or_else(|| SUNDAYError::Engine(EngineError::Generation("Invalid embedding format".into())))?
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();

        if vec.len() != self.dim {
            return Err(SUNDAYError::Engine(EngineError::Generation(
                format!("Embedding dimension mismatch: expected {}, got {}", self.dim, vec.len())
            )));
        }

        Ok(vec)
    }

    fn dim(&self) -> usize {
        self.dim
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// L2-normalize a vector in-place.
pub fn l2_normalize(vec: &mut [f32]) {
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in vec.iter_mut() {
            *v /= norm;
        }
    }
}
