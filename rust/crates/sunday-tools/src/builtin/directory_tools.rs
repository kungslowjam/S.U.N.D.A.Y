//! Directory listing tool — fast filesystem traversal.

use crate::traits::BaseTool;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

static SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "list_directory".into(),
    description: "List files and directories with optional recursive traversal and pattern filtering".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "path": {
                "type": "string",
                "description": "Directory path to list. Defaults to current directory."
            },
            "recursive": {
                "type": "boolean",
                "description": "List recursively (default: false)"
            },
            "pattern": {
                "type": "string",
                "description": "Glob pattern to filter files (e.g., '*.py', '*.md')"
            },
            "max_depth": {
                "type": "integer",
                "description": "Maximum depth for recursive listing (default: 3)"
            },
            "show_hidden": {
                "type": "boolean",
                "description": "Include hidden files (default: false)"
            }
        },
        "required": []
    }),
    category: "filesystem".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: false,
    timeout_seconds: 30.0,
    required_capabilities: vec!["file:read".into()],
    metadata: HashMap::new(),
});

pub struct ListDirectoryTool;

impl BaseTool for ListDirectoryTool {
    fn tool_id(&self) -> &str {
        "list_directory"
    }

    fn spec(&self) -> &ToolSpec {
        &SPEC
    }

    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let path_str = params["path"].as_str().unwrap_or(".");
        let recursive = params["recursive"].as_bool().unwrap_or(false);
        let pattern = params["pattern"].as_str().unwrap_or("");
        let max_depth = params["max_depth"].as_u64().unwrap_or(3) as usize;
        let show_hidden = params["show_hidden"].as_bool().unwrap_or(false);

        let path = Path::new(path_str);

        if !path.exists() {
            return Ok(ToolResult::failure(
                "list_directory",
                format!("Directory not found: {}", path_str),
            ));
        }
        if !path.is_dir() {
            return Ok(ToolResult::failure(
                "list_directory",
                format!("Not a directory: {}", path_str),
            ));
        }

        let mut output = String::new();

        if recursive {
            output.push_str(&format!("Directory tree of {}:\n", path.canonicalize().unwrap_or_else(|_| path.to_path_buf()).display()));
            walk_dir(path, "", &mut output, max_depth, 0, pattern, show_hidden)?;
        } else {
            output.push_str(&format!("Contents of {}:\n", path.canonicalize().unwrap_or_else(|_| path.to_path_buf()).display()));
            list_flat(path, &mut output, pattern, show_hidden)?;
        }

        Ok(ToolResult::success("list_directory", output))
    }
}

fn list_flat(path: &Path, output: &mut String, pattern: &str, show_hidden: bool) -> std::io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| {
        let is_dir = e.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        let name = e.file_name().to_string_lossy().to_lowercase();
        (!is_dir, name)
    });

    for entry in entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if !show_hidden && name_str.starts_with('.') {
            continue;
        }

        if !pattern.is_empty() {
            let is_dir = entry.file_type()?.is_dir();
            if !is_dir && !glob_match(pattern, &name_str) {
                continue;
            }
        }

        let is_dir = entry.file_type()?.is_dir();
        let prefix = if is_dir { "📁 " } else { "📄 " };
        let size = if !is_dir {
            match entry.metadata() {
                Ok(m) => format!(" ({})", format_size(m.len())),
                Err(_) => String::new(),
            }
        } else {
            String::new()
        };

        output.push_str(&format!("  {}{}{}\n", prefix, name_str, size));
    }

    Ok(())
}

fn walk_dir(
    path: &Path,
    prefix: &str,
    output: &mut String,
    max_depth: usize,
    current_depth: usize,
    pattern: &str,
    show_hidden: bool,
) -> std::io::Result<()> {
    if current_depth >= max_depth {
        output.push_str(&format!("{}  ... (max depth reached)\n", prefix));
        return Ok(());
    }

    let entries: Vec<_> = match std::fs::read_dir(path) {
        Ok(r) => r.collect::<Result<Vec<_>, _>>()?,
        Err(_) => {
            output.push_str(&format!("{}  [permission denied]\n", prefix));
            return Ok(());
        }
    };

    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if !show_hidden && name_str.starts_with('.') {
            continue;
        }

        let is_dir = entry.file_type()?.is_dir();
        if is_dir {
            dirs.push(entry);
        } else if pattern.is_empty() || glob_match(pattern, &name_str) {
            files.push(entry);
        }
    }

    dirs.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());
    files.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());

    for file in files {
        let name = file.file_name();
        let name_str = name.to_string_lossy();
        let size = match file.metadata() {
            Ok(m) => format!(" ({})", format_size(m.len())),
            Err(_) => String::new(),
        };
        output.push_str(&format!("{}  📄 {}{}\n", prefix, name_str, size));
    }

    for (i, dir) in dirs.iter().enumerate() {
        let name = dir.file_name();
        let name_str = name.to_string_lossy();
        let is_last = i == dirs.len() - 1;
        let connector = if is_last { "└──" } else { "├──" };
        output.push_str(&format!("{}  {} 📁 {}/\n", prefix, connector, name_str));
        let extension = if is_last { "    " } else { "│   " };
        walk_dir(&dir.path(), &format!("{}{}", prefix, extension), output, max_depth, current_depth + 1, pattern, show_hidden)?;
    }

    Ok(())
}

fn format_size(size: u64) -> String {
    if size < 1024 {
        format!("{}B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1}KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1}MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Simple glob matching (supports * and ?)
fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    glob_match_slice(&pattern_chars, &text_chars)
}

fn glob_match_slice(pattern: &[char], text: &[char]) -> bool {
    let mut p_idx = 0usize;
    let mut t_idx = 0usize;
    let mut star_idx = None;
    let mut match_idx = 0usize;

    while t_idx < text.len() {
        if p_idx < pattern.len() && (pattern[p_idx] == '?' || pattern[p_idx] == text[t_idx]) {
            p_idx += 1;
            t_idx += 1;
        } else if p_idx < pattern.len() && pattern[p_idx] == '*' {
            star_idx = Some(p_idx);
            p_idx += 1;
            match_idx = t_idx;
        } else if let Some(star) = star_idx {
            p_idx = star + 1;
            match_idx += 1;
            t_idx = match_idx;
        } else {
            return false;
        }
    }

    while p_idx < pattern.len() && pattern[p_idx] == '*' {
        p_idx += 1;
    }

    p_idx == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.py", "test.py"));
        assert!(glob_match("*.py", "foo.bar.py"));
        assert!(!glob_match("*.py", "test.txt"));
        assert!(glob_match("test.?", "test.1"));
        assert!(!glob_match("test.?", "test.12"));
        assert!(glob_match("*", "anything"));
    }

    #[test]
    fn test_list_directory_flat() {
        let tool = ListDirectoryTool;
        let result = tool.execute(&serde_json::json!({"path": "."})).unwrap();
        assert!(result.success);
        assert!(!result.content.is_empty());
    }

    #[test]
    fn test_list_directory_not_found() {
        let tool = ListDirectoryTool;
        let result = tool.execute(&serde_json::json!({"path": "/nonexistent/path"})).unwrap();
        assert!(!result.success);
    }
}
