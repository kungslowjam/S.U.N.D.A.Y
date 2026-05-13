use std::path::Path;
use tokenizers::Tokenizer;
use crate::error::SUNDAYError;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;

/// A thread-safe, cached tokenizer manager.
pub struct TokenizerManager {
    cache: RwLock<HashMap<String, Tokenizer>>,
}

impl TokenizerManager {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Load a tokenizer from a JSON file (e.g. tokenizer.json)
    pub fn load_from_file(&self, name: &str, path: &Path) -> Result<(), SUNDAYError> {
        let tokenizer = Tokenizer::from_file(path)
            .map_err(|e| SUNDAYError::Engine(crate::error::EngineError::Tokenizer(format!("Failed to load tokenizer {}: {}", name, e))))?;
        self.cache.write().insert(name.to_string(), tokenizer);
        Ok(())
    }

    /// Count tokens in a string using a named tokenizer.
    /// Falls back to a rough character-based estimate if tokenizer not found.
    pub fn count_tokens(&self, name: &str, text: &str) -> usize {
        let cache = self.cache.read();
        if let Some(tokenizer) = cache.get(name) {
            match tokenizer.encode(text, true) {
                Ok(encoding) => encoding.get_ids().len(),
                Err(_) => text.len() / 4 + 1,
            }
        } else {
            // Fallback: rough estimate
            text.len() / 4 + 1
        }
    }
}

pub static TOKENIZER: Lazy<TokenizerManager> = Lazy::new(TokenizerManager::new);
