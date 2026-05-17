//! Parse THOUGHT/TOOL/INPUT/FINAL_ANSWER from model output.

use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use once_cell::sync::Lazy;

// Match THOUGHT/Thinking Process/Reasoning/<thought> headers.
// Capture group 1 is the content up to first terminator.
static THOUGHT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?is)(?:THOUGHT|Thinking Process|Reasoning|<thought>):\s*(.+?)(?:\nTOOL:|\nFINAL[_ ]?ANSWER:|</thought>|\z)").unwrap()
});

static FINAL_ANSWER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)FINAL[_ ]?ANSWER:\s*(.+)$").unwrap()
});

static XML_TOOL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<tool_call>\s*(\{.*?\})\s*</tool_call>").unwrap()
});

static INLINE_TOOL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)TOOL:\s*([\w_]+)\s*\((.+?)\)(?:\n|$)").unwrap()
});

static TOOL_NAME_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)TOOL:\s*([\w_]+)").unwrap()
});

static INPUT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)INPUT:\s*(.+?)(?:\n(?:THOUGHT|Thinking Process|Reasoning):|\nTOOL:|\nFINAL|$)").unwrap()
});

/// Parse structured response into thought/tool/input/final_answer.
pub fn parse_structured_response(text: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    result.insert("thought".to_string(), String::new());
    result.insert("tool".to_string(), String::new());
    result.insert("input".to_string(), String::new());
    result.insert("final_answer".to_string(), String::new());

    let mut search_text = text.to_string();

    // 1. Extract THOUGHT block
    if let Some(caps) = THOUGHT_RE.captures(text) {
        if let Some(m) = caps.get(1) {
            result.insert("thought".to_string(), m.as_str().trim().to_string());
        }
        // Only remove the thought header+content, NOT the terminator
        // The full match includes the terminator (\nTOOL: etc), so we need to put it back
        if let Some(full) = caps.get(0) {
            let full_str = full.as_str();
            // Find what terminator was matched and restore it
            let restored = if full_str.ends_with("\nTOOL:") {
                "\nTOOL:"
            } else if full_str.ends_with("\nFINAL ANSWER:") || full_str.ends_with("\nFINAL_ANSWER:") {
                if full_str.ends_with("\nFINAL ANSWER:") { "\nFINAL ANSWER:" } else { "\nFINAL_ANSWER:" }
            } else if full_str.ends_with("</thought>") {
                "</thought>"
            } else {
                ""
            };
            search_text = search_text.replace(full_str, restored);
        }
    }

    // 2. FINAL_ANSWER check (priority)
    if let Some(caps) = FINAL_ANSWER_RE.captures(&search_text) {
        if let Some(m) = caps.get(1) {
            result.insert("final_answer".to_string(), m.as_str().trim().to_string());
            return result;
        }
    }

    // 3. XML Tool Call Format
    if let Some(caps) = XML_TOOL_RE.captures(&search_text) {
        if let Some(m) = caps.get(1) {
            if let Ok(tc) = serde_json::from_str::<Value>(m.as_str()) {
                if let Some(name) = tc.get("name").and_then(|v| v.as_str()) {
                    result.insert("tool".to_string(), name.to_string());
                }
                if let Some(args) = tc.get("arguments") {
                    result.insert("input".to_string(), args.to_string());
                }
                return result;
            }
        }
    }

    // 4. Standard Inline Format
    if let Some(caps) = INLINE_TOOL_RE.captures(&search_text) {
        if let Some(name) = caps.get(1) {
            result.insert("tool".to_string(), name.as_str().trim().to_string());
        }
        if let Some(input) = caps.get(2) {
            result.insert("input".to_string(), input.as_str().trim().to_string());
        }
        return result;
    }

    // 5. Standard Format TOOL: name \n INPUT: json
    if let Some(caps) = TOOL_NAME_RE.captures(&search_text) {
        if let Some(m) = caps.get(1) {
            result.insert("tool".to_string(), m.as_str().trim().to_string());
        }
    }

    if let Some(caps) = INPUT_RE.captures(&search_text) {
        if let Some(m) = caps.get(1) {
            result.insert("input".to_string(), m.as_str().trim().to_string());
        }
    }

    // Validate: if tool empty but input not, clear input
    if result.get("tool").map(|s| s.is_empty()).unwrap_or(true)
        && result.get("input").map(|s| !s.is_empty()).unwrap_or(false) {
        result.insert("input".to_string(), String::new());
    }

    result
}
