//! Browser automation tools with AX Tree and Element ID support.

use crate::traits::BaseTool;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec};
use crate::browser_native::NativeBrowserSession;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

static SESSION: Lazy<Arc<NativeBrowserSession>> = Lazy::new(|| Arc::new(NativeBrowserSession::new()));

// Helper to resolve a CSS selector or an element ID (e.g. "12" -> "[data-sunday-id=\"12\"]")
fn resolve_selector(input: &str) -> String {
    let trimmed = input.trim();
    if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()) {
        format!("[data-sunday-id=\"{}\"]", trimmed)
    } else {
        trimmed.to_string()
    }
}

// ---------------------------------------------------------------------------
// Browser Navigate
// ---------------------------------------------------------------------------

static NAVIGATE_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "browser_navigate".into(),
    description: "Navigate the browser to a URL".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "url": { "type": "string", "description": "URL to navigate to" }
        },
        "required": ["url"]
    }),
    category: "browser".into(),
    cost_estimate: 0.0,
    latency_estimate: 2.0,
    requires_confirmation: false,
    timeout_seconds: 60.0,
    required_capabilities: vec!["browser:navigate".into()],
    metadata: HashMap::new(),
});

pub struct BrowserNavigateTool;

impl BaseTool for BrowserNavigateTool {
    fn tool_id(&self) -> &str { "browser_navigate" }
    fn spec(&self) -> &ToolSpec { &NAVIGATE_SPEC }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let url = params["url"].as_str().unwrap_or("");
        if url.is_empty() {
            return Ok(ToolResult::failure(self.tool_id(), "URL is required"));
        }

        let rt = tokio::runtime::Handle::current();
        let result = rt.block_on(async {
            let page = SESSION.ensure_page(false).await.map_err(|e| e.to_string())?;
            page.goto(url).await.map_err(|e| e.to_string())?;
            
            // Wait for SPA rendering
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
            
            // Extract all visible page text via Chrome JS evaluation
            let text = match page.evaluate("document.body.innerText").await {
                Ok(js_val) => js_val.value().and_then(|v| v.as_str()).unwrap_or("").to_string(),
                Err(_) => "".to_string(),
            };
            
            let trimmed = if text.len() > 3000 {
                format!("{}... [Truncated]", &text[..3000])
            } else {
                text
            };
            
            Ok::<String, String>(format!("Navigated to {}.\n\nVisible page text:\n{}", url, trimmed))
        });

        match result {
            Ok(msg) => Ok(ToolResult::success(self.tool_id(), msg)),
            Err(e) => Ok(ToolResult::failure(self.tool_id(), e)),
        }
    }
}

// ---------------------------------------------------------------------------
// Browser Screenshot
// ---------------------------------------------------------------------------

static SCREENSHOT_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "browser_screenshot".into(),
    description: "Take a screenshot of the current page".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "name": { "type": "string", "description": "Optional name for the screenshot" }
        }
    }),
    category: "browser".into(),
    cost_estimate: 0.0,
    latency_estimate: 1.0,
    requires_confirmation: false,
    timeout_seconds: 30.0,
    required_capabilities: vec!["browser:screenshot".into()],
    metadata: HashMap::new(),
});

pub struct BrowserScreenshotTool;

impl BaseTool for BrowserScreenshotTool {
    fn tool_id(&self) -> &str { "browser_screenshot" }
    fn spec(&self) -> &ToolSpec { &SCREENSHOT_SPEC }
    fn execute(&self, _params: &Value) -> Result<ToolResult, SUNDAYError> {
        let rt = tokio::runtime::Handle::current();
        let result = rt.block_on(async {
            let path = SESSION.capture_screenshot_file().await.map_err(|e| e.to_string())?;
            Ok::<String, String>(path)
        });

        match result {
            Ok(path) => {
                let mut res = ToolResult::success(self.tool_id(), "Screenshot captured");
                let uri = if path.starts_with('/') {
                    format!("file://{}", path.replace('\\', "/"))
                } else {
                    format!("file:///{}", path.replace('\\', "/"))
                };
                res.metadata.insert("screenshot_path".to_string(), Value::String(uri));
                Ok(res)
            },
            Err(e) => Ok(ToolResult::failure(self.tool_id(), e)),
        }
    }
}

// ---------------------------------------------------------------------------
// Browser View Tree (AX Tree)
// ---------------------------------------------------------------------------

static VIEW_TREE_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "browser_view_tree".into(),
    description: "Get a list of interactive elements (AX Tree) on the current page with their IDs".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {}
    }),
    category: "browser".into(),
    cost_estimate: 0.0,
    latency_estimate: 1.0,
    requires_confirmation: false,
    timeout_seconds: 30.0,
    required_capabilities: vec!["browser:navigate".into()],
    metadata: HashMap::new(),
});

pub struct BrowserViewTreeTool;

impl BaseTool for BrowserViewTreeTool {
    fn tool_id(&self) -> &str { "browser_view_tree" }
    fn spec(&self) -> &ToolSpec { &VIEW_TREE_SPEC }
    fn execute(&self, _params: &Value) -> Result<ToolResult, SUNDAYError> {
        let rt = tokio::runtime::Handle::current();
        let result = rt.block_on(async {
            SESSION.get_ax_tree().await.map_err(|e| e.to_string())
        });

        match result {
            Ok(json_val) => {
                let mut tree_text = String::new();
                tree_text.push_str("Interactive Elements on Current Page:\n");
                if let Some(arr) = json_val.as_array() {
                    if arr.is_empty() {
                        tree_text.push_str("(No interactive elements found)");
                    } else {
                        for el in arr {
                            let id = el["id"].as_i64().unwrap_or(0);
                            let role = el["role"].as_str().unwrap_or("element");
                            let text = el["text"].as_str().unwrap_or("");
                            tree_text.push_str(&format!("[{}] {}: \"{}\"\n", id, role, text));
                        }
                    }
                } else {
                    tree_text.push_str("(Failed to parse elements)");
                }
                Ok(ToolResult::success(self.tool_id(), tree_text))
            }
            Err(e) => Ok(ToolResult::failure(self.tool_id(), e)),
        }
    }
}

// ---------------------------------------------------------------------------
// Browser Click (Antigravity/OpenWork Style)
// ---------------------------------------------------------------------------

static CLICK_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "browser_click".into(),
    description: "Click an element on the page using a CSS selector or an element ID from browser_view_tree".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "selector": { "type": "string", "description": "CSS selector or element ID (e.g. '12' or '#login-btn') to click" }
        },
        "required": ["selector"]
    }),
    category: "browser".into(),
    cost_estimate: 0.0,
    latency_estimate: 1.0,
    requires_confirmation: false,
    timeout_seconds: 30.0,
    required_capabilities: vec!["browser:click".into()],
    metadata: HashMap::new(),
});

pub struct BrowserClickTool;

impl BaseTool for BrowserClickTool {
    fn tool_id(&self) -> &str { "browser_click" }
    fn spec(&self) -> &ToolSpec { &CLICK_SPEC }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let input_selector = params["selector"].as_str().unwrap_or("");
        let resolved = resolve_selector(input_selector);
        
        let rt = tokio::runtime::Handle::current();
        let result = rt.block_on(async {
            SESSION.human_click(&resolved).await.map_err(|e| e.to_string())
        });
        match result {
            Ok(_) => Ok(ToolResult::success(self.tool_id(), format!("Clicked element: {}", input_selector))),
            Err(e) => Ok(ToolResult::failure(self.tool_id(), e)),
        }
    }
}

// ---------------------------------------------------------------------------
// Browser Type (Antigravity/OpenWork Style)
// ---------------------------------------------------------------------------

static TYPE_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "browser_type".into(),
    description: "Type text into an element using a CSS selector or an element ID from browser_view_tree".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "selector": { "type": "string", "description": "CSS selector or element ID (e.g. '12' or '#username') to type into" },
            "text": { "type": "string", "description": "Text to type" }
        },
        "required": ["selector", "text"]
    }),
    category: "browser".into(),
    cost_estimate: 0.0,
    latency_estimate: 2.0,
    requires_confirmation: false,
    timeout_seconds: 30.0,
    required_capabilities: vec!["browser:type".into()],
    metadata: HashMap::new(),
});

pub struct BrowserTypeTool;

impl BaseTool for BrowserTypeTool {
    fn tool_id(&self) -> &str { "browser_type" }
    fn spec(&self) -> &ToolSpec { &TYPE_SPEC }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let input_selector = params["selector"].as_str().unwrap_or("");
        let resolved = resolve_selector(input_selector);
        let text = params["text"].as_str().unwrap_or("");
        
        let rt = tokio::runtime::Handle::current();
        let result = rt.block_on(async {
            SESSION.human_type(&resolved, text).await.map_err(|e| e.to_string())
        });
        match result {
            Ok(_) => Ok(ToolResult::success(self.tool_id(), format!("Typed into element: {}", input_selector))),
            Err(e) => Ok(ToolResult::failure(self.tool_id(), e)),
        }
    }
}
