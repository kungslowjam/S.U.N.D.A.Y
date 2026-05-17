//! Extract tool calls from text output (Action/Action Input, XML, inline).

use regex::Regex;
use serde_json::Value;
use once_cell::sync::Lazy;

static ACTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)Action:\s*(.+)").unwrap()
});

static ACTION_INPUT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)Action Input:\s*(.+?)(?:\n\n|\z)").unwrap()
});

static XML_TOOL_CALL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<tool_call>\s*(\w+)\s*(.*?)</\w+>").unwrap()
});

static XML_PARAM_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\$(\w+)=(.+?)(?:\$|\n<|</|$)").unwrap()
});

// Matches <key>value</key> — we capture key and value, then verify closing tag matches manually.
static XML_TAG_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<(\w+)>(.*?)</(\w+)>").unwrap()
});

static XML_KV_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\w+)\s*:\s*(.+?)(?:\n\w+\s*:|$)").unwrap()
});

static INLINE_XML_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<(\w+)\s+(.*?)/?>").unwrap()
});

static ATTR_QUOTED_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(\w+)=["']([^"']*)["']"#).unwrap()
});

static ATTR_UNQUOTED_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\w+)=(\S+)").unwrap()
});

/// Extract tool call from text. Returns (tool_name, params_json) or None.
pub fn extract_tool_call(text: &str) -> Option<(String, String)> {
    // Format 1: Action / Action Input
    if let Some(action_caps) = ACTION_RE.captures(text) {
        let tool_name = action_caps.get(1)?.as_str().trim().to_string();
        let input = ACTION_INPUT_RE
            .captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_else(|| "{}".to_string());
        return Some((tool_name, input));
    }

    // Format 2: <tool_call>tool_name ... </tool_call>
    if let Some(xml_caps) = XML_TOOL_CALL_RE.captures(text) {
        let tool_name = xml_caps.get(1)?.as_str().trim().to_string();
        let raw_params = xml_caps.get(2)?.as_str().trim();

        let mut params = serde_json::Map::new();

        // Try $key=value format
        for caps in XML_PARAM_RE.captures_iter(raw_params) {
            let key = caps.get(1)?.as_str().to_string();
            let val = caps.get(2)?.as_str().trim().trim_end_matches("</>\n").to_string();
            params.insert(key, Value::String(val));
        }

        // Try <key>value</key> format (validate open/close tags match)
        for caps in XML_TAG_RE.captures_iter(raw_params) {
            let open_tag = caps.get(1)?.as_str();
            let close_tag = caps.get(3)?.as_str();
            if open_tag != close_tag {
                continue; // skip mismatched tags
            }
            let key = open_tag.to_string();
            let val = caps.get(2)?.as_str().trim().to_string();
            // Try parse as int
            let v = if let Ok(n) = val.parse::<i64>() {
                Value::Number(n.into())
            } else {
                Value::String(val)
            };
            params.insert(key, v);
        }

        // Try key: value format
        if params.is_empty() {
            for caps in XML_KV_RE.captures_iter(raw_params) {
                let key = caps.get(1)?.as_str().to_string();
                let val = caps.get(2)?.as_str().trim().trim_matches('"').trim_matches('\'').to_string();
                let v = if let Ok(n) = val.parse::<i64>() {
                    Value::Number(n.into())
                } else {
                    Value::String(val)
                };
                params.insert(key, v);
            }
        }

        let json_str = if params.is_empty() {
            "{}".to_string()
        } else {
            Value::Object(params).to_string()
        };
        return Some((tool_name, json_str));
    }

    // Format 3: <tool_name attr="value" />
    if let Some(inline_caps) = INLINE_XML_RE.captures(text) {
        let tool_name = inline_caps.get(1)?.as_str().trim().to_string();

        // Skip common non-tool tags
        let lower = tool_name.to_lowercase();
        if matches!(lower.as_str(), "think" | "br" | "hr" | "p" | "div" | "span" | "b" | "i") {
            return None;
        }

        let attr_str = inline_caps.get(2)?.as_str().trim();
        let mut params = serde_json::Map::new();

        // Quoted attributes
        for caps in ATTR_QUOTED_RE.captures_iter(attr_str) {
            let key = caps.get(1)?.as_str().to_string();
            let val = caps.get(2)?.as_str().to_string();
            params.insert(key, Value::String(val));
        }

        // Unquoted attributes
        if params.is_empty() {
            for caps in ATTR_UNQUOTED_RE.captures_iter(attr_str) {
                let key = caps.get(1)?.as_str().to_string();
                let val = caps.get(2)?.as_str().trim_end_matches('>').to_string();
                params.insert(key, Value::String(val));
            }
        }

        let json_str = if params.is_empty() {
            "{}".to_string()
        } else {
            Value::Object(params).to_string()
        };
        return Some((tool_name, json_str));
    }

    None
}
