//! Microsoft Office (Word & Excel) automation tools — STUBBED.
//!
//! The original COM-based implementation is broken due to windows crate API changes.
//! Restoring from backup: `office_tools.rs.bak`

use crate::traits::BaseTool;
use sunday_core::{ToolResult, ToolSpec, SUNDAYError};
use serde_json::Value;
use once_cell::sync::Lazy;
use std::collections::HashMap;

static WORD_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "word_write".into(),
    description: "Create or update a Word document with text (stub — not implemented)".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "path": { "type": "string", "description": "Document path" },
            "text": { "type": "string", "description": "Text to write" }
        },
        "required": ["path", "text"]
    }),
    category: "office".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: false,
    timeout_seconds: 30.0,
    required_capabilities: vec!["file:write".into()],
    metadata: HashMap::new(),
});

static EXCEL_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "excel_read".into(),
    description: "Read data from an Excel file (stub — not implemented)".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "path": { "type": "string", "description": "Excel file path" },
            "sheet": { "type": "string", "description": "Sheet name" }
        },
        "required": ["path"]
    }),
    category: "office".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: false,
    timeout_seconds: 30.0,
    required_capabilities: vec!["file:read".into()],
    metadata: HashMap::new(),
});

pub struct WordWriteTool;
pub struct ExcelReadTool;

impl BaseTool for WordWriteTool {
    fn tool_id(&self) -> &str { "word_write" }
    fn spec(&self) -> &ToolSpec { &WORD_SPEC }
    fn execute(&self, _params: &Value) -> Result<ToolResult, SUNDAYError> {
        Ok(ToolResult::failure("word_write", "Word COM automation is not available in this build"))
    }
}

impl BaseTool for ExcelReadTool {
    fn tool_id(&self) -> &str { "excel_read" }
    fn spec(&self) -> &ToolSpec { &EXCEL_SPEC }
    fn execute(&self, _params: &Value) -> Result<ToolResult, SUNDAYError> {
        Ok(ToolResult::failure("excel_read", "Excel COM automation is not available in this build"))
    }
}
