//! Memory management tools — search and store documents in the memory backend.

use crate::traits::BaseTool;
use crate::storage::traits::MemoryBackend;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::sync::Arc;
use std::collections::HashMap;

static SEARCH_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "memory_search".into(),
    description: "Search the long-term memory for documents relevant to a query.".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Search query"
            },
            "top_k": {
                "type": "integer",
                "description": "Number of results to return (default: 5)",
                "default": 5
            }
        },
        "required": ["query"]
    }),
    category: "memory".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.5,
    requires_confirmation: false,
    timeout_seconds: 10.0,
    required_capabilities: vec!["memory:read".into()],
    metadata: HashMap::new(),
});

static STORE_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "memory_store".into(),
    description: "Store a document into long-term memory for later retrieval.".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "content": {
                "type": "string",
                "description": "Document content to store"
            },
            "source": {
                "type": "string",
                "description": "Source identifier (e.g. filename, URL)",
                "default": "manual"
            }
        },
        "required": ["content"]
    }),
    category: "memory".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.5,
    requires_confirmation: false,
    timeout_seconds: 10.0,
    required_capabilities: vec!["memory:write".into()],
    metadata: HashMap::new(),
});

pub struct MemorySearchTool {
    backend: Arc<dyn MemoryBackend>,
}

impl MemorySearchTool {
    pub fn new(backend: Arc<dyn MemoryBackend>) -> Self {
        Self { backend }
    }
}

impl BaseTool for MemorySearchTool {
    fn tool_id(&self) -> &str {
        "memory_search"
    }
    fn spec(&self) -> &ToolSpec {
        &SEARCH_SPEC
    }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let query = params["query"].as_str().unwrap_or("");
        let top_k = params["top_k"].as_u64().unwrap_or(5) as usize;

        if query.is_empty() {
            return Ok(ToolResult::failure("memory_search", "Empty query"));
        }

        match self.backend.retrieve(query, top_k) {
            Ok(results) => {
                if results.is_empty() {
                    Ok(ToolResult::success("memory_search", "No relevant documents found."))
                } else {
                    let lines: Vec<String> = results
                        .iter()
                        .map(|r| format!("[score={:.3}] {} (source: {})", r.score, r.content, r.source))
                        .collect();
                    Ok(ToolResult::success("memory_search", lines.join("\n")))
                }
            }
            Err(e) => Ok(ToolResult::failure("memory_search", format!("Search failed: {}", e))),
        }
    }
}

pub struct MemoryStoreTool {
    backend: Arc<dyn MemoryBackend>,
}

impl MemoryStoreTool {
    pub fn new(backend: Arc<dyn MemoryBackend>) -> Self {
        Self { backend }
    }
}

impl BaseTool for MemoryStoreTool {
    fn tool_id(&self) -> &str {
        "memory_store"
    }
    fn spec(&self) -> &ToolSpec {
        &STORE_SPEC
    }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let content = params["content"].as_str().unwrap_or("");
        let source = params["source"].as_str().unwrap_or("manual");

        if content.is_empty() {
            return Ok(ToolResult::failure("memory_store", "Empty content"));
        }

        match self.backend.store(content, source, None) {
            Ok(doc_id) => Ok(ToolResult::success("memory_store", format!("Stored document ID: {}", doc_id))),
            Err(e) => Ok(ToolResult::failure("memory_store", format!("Store failed: {}", e))),
        }
    }
}
