//! Hermes Agentic Tools — Delegation and Planning.

use crate::traits::BaseTool;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Delegate Browser
// ---------------------------------------------------------------------------

static BROWSER_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "delegate_browser".into(),
    description: "Delegate a complex web task to a specialized browser agent".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "task": { "type": "string", "description": "The web task to perform (e.g., 'Book a flight to NYC')" }
        },
        "required": ["task"]
    }),
    category: "hermes".into(),
    cost_estimate: 0.05,
    latency_estimate: 15.0,
    requires_confirmation: true,
    timeout_seconds: 300.0,
    required_capabilities: vec!["browser:full_control".into()],
    metadata: HashMap::new(),
});

pub struct DelegateBrowserTool;

impl BaseTool for DelegateBrowserTool {
    fn tool_id(&self) -> &str { "delegate_browser" }
    fn spec(&self) -> &ToolSpec { &BROWSER_SPEC }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let task = params["task"].as_str().unwrap_or("");
        // In native mode, this triggers a specialized sub-agent loop
        Ok(ToolResult::success(self.tool_id(), format!("Browser Agent initiated for task: '{}'. Executing...", task)))
    }
}

// ---------------------------------------------------------------------------
// Delegate Research
// ---------------------------------------------------------------------------

static RESEARCH_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "delegate_research".into(),
    description: "Delegate a deep research task to a specialized research agent".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "query": { "type": "string", "description": "The research query" },
            "depth": { "type": "string", "enum": ["quick", "deep"], "description": "Research depth" }
        },
        "required": ["query"]
    }),
    category: "hermes".into(),
    cost_estimate: 0.10,
    latency_estimate: 30.0,
    requires_confirmation: false,
    timeout_seconds: 600.0,
    required_capabilities: vec!["network:search".into(), "network:fetch".into()],
    metadata: HashMap::new(),
});

pub struct DelegateResearchTool;

impl BaseTool for DelegateResearchTool {
    fn tool_id(&self) -> &str { "delegate_research" }
    fn spec(&self) -> &ToolSpec {
        &RESEARCH_SPEC
    }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let query = params["query"].as_str().unwrap_or("");
        Ok(ToolResult::success(self.tool_id(), format!("Research Agent initiated for query: '{}'. Searching sources...", query)))
    }
}
