//! Code analysis tool — extracts symbols and structure from source files.

use crate::traits::BaseTool;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use regex::Regex;

static SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "code_analyzer".into(),
    description: "Analyze source code to extract functions, classes, and important symbols".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "path": { "type": "string", "description": "Path to the source file" },
            "language": { "type": "string", "description": "Programming language (e.g., 'rust', 'python', 'javascript')" }
        },
        "required": ["path"]
    }),
    category: "coding".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.5,
    requires_confirmation: false,
    timeout_seconds: 20.0,
    required_capabilities: vec!["file:read".into()],
    metadata: HashMap::new(),
});

pub struct CodeAnalyzerTool;

impl BaseTool for CodeAnalyzerTool {
    fn tool_id(&self) -> &str { "code_analyzer" }
    fn spec(&self) -> &ToolSpec { &SPEC }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let path_str = params["path"].as_str().unwrap_or("");
        let path = Path::new(path_str);
        
        if !path.exists() {
            return Ok(ToolResult::failure(self.tool_id(), format!("File not found: {}", path_str)));
        }

        let content = std::fs::read_to_string(path).map_err(|e| SUNDAYError::Io(e))?;
        let mut symbols = Vec::new();

        // Basic Regex-based symbol extraction (Simplified for broad support)
        let rust_fn = Regex::new(r"fn\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
        let py_fn = Regex::new(r"def\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
        let js_fn = Regex::new(r"function\s+([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();

        for cap in rust_fn.captures_iter(&content) { symbols.push(format!("[Rust Fn] {}", &cap[1])); }
        for cap in py_fn.captures_iter(&content) { symbols.push(format!("[Python Fn] {}", &cap[1])); }
        for cap in js_fn.captures_iter(&content) { symbols.push(format!("[JS Fn] {}", &cap[1])); }

        let result_msg = if symbols.is_empty() {
            format!("No major symbols identified in {}. The file might be purely data or uses a different syntax.", path_str)
        } else {
            format!("Analysis of {}:\n\n{}", path_str, symbols.join("\n"))
        };

        Ok(ToolResult::success(self.tool_id(), result_msg))
    }
}
