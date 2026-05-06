//! MemoryBackend trait for all storage backends.

use sunday_core::{SUNDAYError, RetrievalResult};
use serde_json::Value;

pub trait MemoryBackend: Send + Sync {
    fn backend_id(&self) -> &str;
    fn store(
        &self,
        content: &str,
        source: &str,
        metadata: Option<&Value>,
    ) -> Result<String, SUNDAYError>;
    fn retrieve(
        &self,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<RetrievalResult>, SUNDAYError>;
    fn delete(&self, doc_id: &str) -> Result<bool, SUNDAYError>;
    fn clear(&self) -> Result<(), SUNDAYError>;
    fn count(&self) -> Result<usize, SUNDAYError>;
}
