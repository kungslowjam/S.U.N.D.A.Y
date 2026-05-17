//! File grep tool — regex search across files and directories.

use crate::traits::BaseTool;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

static SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "file_grep".into(),
    description: "Search for a regex pattern in files within a directory".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "pattern": {
                "type": "string",
                "description": "Regex pattern to search for"
            },
            "path": {
                "type": "string",
                "description": "Directory or file to search in (default: current directory)"
            },
            "glob": {
                "type": "string",
                "description": "Glob pattern to filter files (e.g., '*.rs', '*.py')"
            },
            "max_results": {
                "type": "integer",
                "description": "Maximum number of matches to return (default: 50)"
            }
        },
        "required": ["pattern"]
    }),
    category: "filesystem".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: false,
    timeout_seconds: 30.0,
    required_capabilities: vec!["file:read".into()],
    metadata: HashMap::new(),
});

pub struct FileGrepTool;

impl BaseTool for FileGrepTool {
    fn tool_id(&self) -> &str {
        "file_grep"
    }

    fn spec(&self) -> &ToolSpec {
        &SPEC
    }

    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let pattern_str = params["pattern"].as_str().unwrap_or("");
        if pattern_str.is_empty() {
            return Ok(ToolResult::failure("file_grep", "Pattern is required"));
        }

        let regex = match Regex::new(pattern_str) {
            Ok(r) => r,
            Err(e) => return Ok(ToolResult::failure("file_grep", format!("Invalid regex: {}", e))),
        };

        let path_str = params["path"].as_str().unwrap_or(".");
        let glob = params["glob"].as_str().unwrap_or("");
        let max_results = params["max_results"].as_u64().unwrap_or(50) as usize;

        let path = Path::new(path_str);
        let mut matches = Vec::new();
        let mut total_matches = 0usize;

        if path.is_file() {
            search_file(path, &regex, &mut matches, &mut total_matches, max_results);
        } else if path.is_dir() {
            search_dir(path, &regex, glob, &mut matches, &mut total_matches, max_results);
        } else {
            return Ok(ToolResult::failure("file_grep", format!("Path not found: {}", path_str)));
        }

        if matches.is_empty() {
            return Ok(ToolResult::success("file_grep", format!("No matches found for '{}' in {}", pattern_str, path_str)));
        }

        let mut output = format!("Found {} match(es) for '{}' in {}:\n\n", total_matches, pattern_str, path_str);
        for m in matches {
            output.push_str(&m);
            output.push('\n');
        }
        if total_matches > max_results {
            output.push_str(&format!("\n... and {} more matches (limit: {})", total_matches - max_results, max_results));
        }

        Ok(ToolResult::success("file_grep", output))
    }
}

fn search_file(
    path: &Path,
    regex: &Regex,
    matches: &mut Vec<String>,
    total_matches: &mut usize,
    max_results: usize,
) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    for (line_num, line) in content.lines().enumerate() {
        if regex.is_match(line) {
            *total_matches += 1;
            if matches.len() < max_results {
                let file = path.display();
                let highlighted = regex.replace_all(line, |caps: &regex::Captures| {
                    format!("**{}**", &caps[0])
                });
                matches.push(format!("{}:{}: {}", file, line_num + 1, highlighted));
            }
        }
    }
}

fn search_dir(
    dir: &Path,
    regex: &Regex,
    glob: &str,
    matches: &mut Vec<String>,
    total_matches: &mut usize,
    max_results: usize,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();

        if name.starts_with('.') {
            continue;
        }

        if path.is_file() {
            if !glob.is_empty() && !glob_match(glob, &name) {
                continue;
            }
            search_file(&path, regex, matches, total_matches, max_results);
        } else if path.is_dir() {
            search_dir(&path, regex, glob, matches, total_matches, max_results);
        }

        if *total_matches >= max_results + 100 {
            break;
        }
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    let txt: Vec<char> = text.chars().collect();
    glob_match_slice(&pat, &txt)
}

fn glob_match_slice(pattern: &[char], text: &[char]) -> bool {
    let mut p = 0usize;
    let mut t = 0usize;
    let mut star = None;
    let mut match_t = 0usize;

    while t < text.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            p += 1;
            match_t = t;
        } else if let Some(s) = star {
            p = s + 1;
            match_t += 1;
            t = match_t;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_grep_simple() {
        let tool = FileGrepTool;
        let result = tool.execute(&serde_json::json!({
            "pattern": "fn test",
            "path": ".",
            "glob": "*.rs"
        })).unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_file_grep_no_match() {
        let tool = FileGrepTool;
        let result = tool.execute(&serde_json::json!({
            "pattern": "XYZ_NOT_FOUND_12345",
            "path": "."
        })).unwrap();
        assert!(result.success);
        assert!(result.content.contains("No matches"));
    }
}
