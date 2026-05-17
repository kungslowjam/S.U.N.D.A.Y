//! Web search tool — DuckDuckGo Lite search.

use crate::traits::BaseTool;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;

static SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "web_search".into(),
    description: "Search the web using DuckDuckGo. Returns a list of search results with title, snippet, and URL.".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Search query"
            },
            "num_results": {
                "type": "integer",
                "description": "Number of results to return (default: 5)",
                "default": 5
            }
        },
        "required": ["query"]
    }),
    category: "network".into(),
    cost_estimate: 0.0,
    latency_estimate: 2.0,
    requires_confirmation: false,
    timeout_seconds: 15.0,
    required_capabilities: vec!["network:fetch".into()],
    metadata: HashMap::new(),
});

pub struct WebSearchTool;

impl BaseTool for WebSearchTool {
    fn tool_id(&self) -> &str {
        "web_search"
    }
    fn spec(&self) -> &ToolSpec {
        &SPEC
    }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let query = params["query"].as_str().unwrap_or("");
        if query.is_empty() {
            return Ok(ToolResult::failure("web_search", "Empty query"));
        }
        let num_results = params["num_results"].as_u64().unwrap_or(5) as usize;

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| SUNDAYError::Io(std::io::Error::other(e.to_string())))?;

        let url = format!(
            "https://lite.duckduckgo.com/lite/?q={}",
            urlencoding::encode(query)
        );

        match client.get(&url).header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)").send() {
            Ok(resp) => {
                let html = resp.text().unwrap_or_default();
                let results = parse_ddg_lite(&html, num_results);
                if results.is_empty() {
                    Ok(ToolResult::success("web_search", "No results found.".to_string()))
                } else {
                    let text = results.join("\n---\n");
                    Ok(ToolResult::success("web_search", text))
                }
            }
            Err(e) => Ok(ToolResult::failure(
                "web_search",
                format!("Search failed: {}", e),
            )),
        }
    }
}

/// Very simple HTML parser for DuckDuckGo Lite results.
fn parse_ddg_lite(html: &str, limit: usize) -> Vec<String> {
    let mut results = Vec::new();
    let lines: Vec<&str> = html.lines().collect();
    let mut i = 0;

    while i < lines.len() && results.len() < limit {
        // DuckDuckGo Lite result links are in <a rel="nofollow" href="...">
        if lines[i].contains("<a rel=\"nofollow\"") && lines[i].contains("class=\"result-link\"") {
            // Extract URL
            let url = extract_attr(lines[i], "href").unwrap_or_default();
            // Title is usually the next line or same line
            let title = strip_html(lines[i]);

            // Snippet is a few lines down
            let mut snippet = String::new();
            for j in (i + 1)..(i + 8).min(lines.len()) {
                if lines[j].contains("class=\"result-snippet\"") {
                    snippet = strip_html(lines[j]);
                    break;
                }
            }

            if !url.is_empty() {
                results.push(format!("Title: {}\nURL: {}\nSnippet: {}", title.trim(), url, snippet.trim()));
            }
        }
        i += 1;
    }

    results
}

fn extract_attr(line: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    if let Some(start) = line.find(&pattern) {
        let rest = &line[start + pattern.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

fn strip_html(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result
}
