//! Web fetch tool — fetch a URL and extract readable text.

use crate::traits::BaseTool;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

static SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "web_fetch".into(),
    description: "Fetch a URL and return the extracted text content".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "url": {
                "type": "string",
                "description": "URL to fetch"
            },
            "max_length": {
                "type": "integer",
                "description": "Maximum characters to return (default: 8000)"
            }
        },
        "required": ["url"]
    }),
    category: "web".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: false,
    timeout_seconds: 15.0,
    required_capabilities: vec!["network".into()],
    metadata: HashMap::new(),
});

pub struct WebFetchTool;

impl BaseTool for WebFetchTool {
    fn tool_id(&self) -> &str {
        "web_fetch"
    }

    fn spec(&self) -> &ToolSpec {
        &SPEC
    }

    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let url = params["url"].as_str().unwrap_or("");
        if url.is_empty() {
            return Ok(ToolResult::failure("web_fetch", "URL is required"));
        }

        let max_length = params["max_length"].as_u64().unwrap_or(8000) as usize;

        let client = match reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("SUNDAY-Agent/1.0")
            .build()
        {
            Ok(c) => c,
            Err(e) => return Ok(ToolResult::failure("web_fetch", format!("Client build error: {}", e))),
        };

        let resp = match client.get(url).send() {
            Ok(r) => r,
            Err(e) => return Ok(ToolResult::failure("web_fetch", format!("Request failed: {}", e))),
        };

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let html = match resp.text() {
            Ok(t) => t,
            Err(e) => return Ok(ToolResult::failure("web_fetch", format!("Read error: {}", e))),
        };

        let text = if content_type.contains("text/html") || html.trim_start().starts_with('<') {
            html_to_text(&html)
        } else {
            html
        };

        let trimmed = if text.len() > max_length {
            format!("{}\n\n[truncated: {} chars total]", &text[..max_length], text.len())
        } else {
            text
        };

        Ok(ToolResult::success("web_fetch", trimmed))
    }
}

fn html_to_text(html: &str) -> String {
    let mut text = html.to_string();

    text = Regex::new(r"<script[^>]*>[\s\S]*?</script>")
        .unwrap()
        .replace_all(&text, "")
        .to_string();
    text = Regex::new(r"<style[^>]*>[\s\S]*?</style>")
        .unwrap()
        .replace_all(&text, "")
        .to_string();
    text = Regex::new(r"<nav[^>]*>[\s\S]*?</nav>")
        .unwrap()
        .replace_all(&text, "")
        .to_string();
    text = Regex::new(r"<header[^>]*>[\s\S]*?</header>")
        .unwrap()
        .replace_all(&text, "")
        .to_string();
    text = Regex::new(r"<footer[^>]*>[\s\S]*?</footer>")
        .unwrap()
        .replace_all(&text, "")
        .to_string();

    text = Regex::new(r"<br\s*/?>")
        .unwrap()
        .replace_all(&text, "\n")
        .to_string();
    text = Regex::new(r"</p>")
        .unwrap()
        .replace_all(&text, "\n\n")
        .to_string();
    text = Regex::new(r"</div>")
        .unwrap()
        .replace_all(&text, "\n")
        .to_string();
    text = Regex::new(r"</li>")
        .unwrap()
        .replace_all(&text, "\n")
        .to_string();
    text = Regex::new(r"<[^>]+>")
        .unwrap()
        .replace_all(&text, "")
        .to_string();

    text = decode_html_entities(&text);

    let lines: Vec<&str> = text.lines().collect();
    let mut cleaned = Vec::new();
    let mut last_was_empty = false;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !last_was_empty {
                cleaned.push("");
                last_was_empty = true;
            }
        } else {
            cleaned.push(trimmed);
            last_was_empty = false;
        }
    }

    cleaned.join("\n").trim().to_string()
}

fn decode_html_entities(text: &str) -> String {
    let mut result = text.to_string();
    let replacements: &[(&str, &str)] = &[
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&nbsp;", " "),
        ("&ndash;", "–"),
        ("&mdash;", "—"),
        ("&hellip;", "…"),
        ("&copy;", "©"),
        ("&reg;", "®"),
        ("&trade;", "™"),
    ];
    for (entity, ch) in replacements {
        result = result.replace(entity, ch);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_to_text() {
        let html = r#"<html><body><p>Hello world</p><script>alert('x')</script><div>Line 2</div></body></html>"#;
        let text = html_to_text(html);
        assert!(text.contains("Hello world"));
        assert!(!text.contains("<script>"));
        assert!(text.contains("Line 2"));
    }
}
