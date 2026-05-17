//! File read/write tools.

use crate::traits::BaseTool;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec};
use sunday_security::file_policy::is_sensitive_file;
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

static READ_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "file_read".into(),
    description: "Read the contents of a file".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "path": { "type": "string", "description": "File path to read" }
        },
        "required": ["path"]
    }),
    category: "filesystem".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: false,
    timeout_seconds: 10.0,
    required_capabilities: vec!["file:read".into()],
    metadata: HashMap::new(),
});

static EDIT_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "file_edit".into(),
    description: "Edit a file by replacing an old string with a new string".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "path": { "type": "string", "description": "File path to edit" },
            "old_string": { "type": "string", "description": "Exact string to replace" },
            "new_string": { "type": "string", "description": "Replacement string" }
        },
        "required": ["path", "old_string", "new_string"]
    }),
    category: "filesystem".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: true,
    timeout_seconds: 10.0,
    required_capabilities: vec!["file:write".into()],
    metadata: HashMap::new(),
});

static WRITE_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "file_write".into(),
    description: "Write or append content to a file".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "path": { "type": "string", "description": "File path to write" },
            "content": { "type": "string", "description": "Content to write" },
            "mode": { "type": "string", "description": "Write mode: 'write' or 'append' (default: 'write')" },
            "create_dirs": { "type": "boolean", "description": "Create parent directories if missing (default: false)" }
        },
        "required": ["path", "content"]
    }),
    category: "filesystem".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: true,
    timeout_seconds: 10.0,
    required_capabilities: vec!["file:write".into()],
    metadata: HashMap::new(),
});

pub struct FileEditTool;

impl BaseTool for FileEditTool {
    fn tool_id(&self) -> &str {
        "file_edit"
    }
    fn spec(&self) -> &ToolSpec {
        &EDIT_SPEC
    }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let path_str = params["path"].as_str().unwrap_or("");
        let old_string = params["old_string"].as_str().unwrap_or("");
        let new_string = params["new_string"].as_str().unwrap_or("");
        let path = Path::new(path_str);

        if is_sensitive_file(path) {
            return Ok(ToolResult::failure(
                "file_edit",
                format!("Access denied: '{}' is a sensitive file", path_str),
            ));
        }

        if old_string.is_empty() {
            return Ok(ToolResult::failure("file_edit", "old_string cannot be empty"));
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return Ok(ToolResult::failure(
                    "file_edit",
                    format!("Error reading '{}': {}", path_str, e),
                ))
            }
        };

        if !content.contains(old_string) {
            return Ok(ToolResult::failure(
                "file_edit",
                format!("old_string not found in '{}'", path_str),
            ));
        }

        let occurrences = content.matches(old_string).count();
        if occurrences > 1 {
            return Ok(ToolResult::failure(
                "file_edit",
                format!(
                    "old_string appears {} times in '{}'. Please be more specific.",
                    occurrences, path_str
                ),
            ));
        }

        let new_content = content.replacen(old_string, new_string, 1);
        match std::fs::write(path, new_content) {
            Ok(()) => Ok(ToolResult::success(
                "file_edit",
                format!("Edited '{}' (replaced 1 occurrence)", path_str),
            )),
            Err(e) => Ok(ToolResult::failure(
                "file_edit",
                format!("Error writing '{}': {}", path_str, e),
            )),
        }
    }
}

pub struct FileReadTool;

impl BaseTool for FileReadTool {
    fn tool_id(&self) -> &str {
        "file_read"
    }
    fn spec(&self) -> &ToolSpec {
        &READ_SPEC
    }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let path_str = params["path"].as_str().unwrap_or("");
        let path = Path::new(path_str);

        if is_sensitive_file(path) {
            return Ok(ToolResult::failure(
                "file_read",
                format!("Access denied: '{}' is a sensitive file", path_str),
            ));
        }

        match std::fs::read_to_string(path) {
            Ok(content) => Ok(ToolResult::success("file_read", content)),
            Err(e) => Ok(ToolResult::failure(
                "file_read",
                format!("Error reading '{}': {}", path_str, e),
            )),
        }
    }
}

pub struct FileWriteTool;

impl BaseTool for FileWriteTool {
    fn tool_id(&self) -> &str {
        "file_write"
    }
    fn spec(&self) -> &ToolSpec {
        &WRITE_SPEC
    }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let path_str = params["path"].as_str().unwrap_or("");
        let content = params["content"].as_str().unwrap_or("");
        let mode = params["mode"].as_str().unwrap_or("write");
        let create_dirs = params["create_dirs"].as_bool().unwrap_or(false);
        let path = Path::new(path_str);

        if is_sensitive_file(path) {
            return Ok(ToolResult::failure(
                "file_write",
                format!("Access denied: '{}' is a sensitive file", path_str),
            ));
        }

        if create_dirs {
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        return Ok(ToolResult::failure(
                            "file_write",
                            format!("Error creating directory: {}", e),
                        ));
                    }
                }
            }
        }

        let result = if mode == "append" {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .and_then(|mut f| {
                    use std::io::Write;
                    f.write_all(content.as_bytes())
                })
        } else {
            std::fs::write(path, content)
        };

        match result {
            Ok(()) => Ok(ToolResult::success(
                "file_write",
                format!("{} {} bytes to {}",
                    if mode == "append" { "Appended" } else { "Written" },
                    content.len(),
                    path_str
                ),
            )),
            Err(e) => Ok(ToolResult::failure(
                "file_write",
                format!("Error writing '{}': {}", path_str, e),
            )),
        }
    }
}

static READ_MULTIPLE_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "read_multiple_files".into(),
    description: "Read the contents of multiple files at once".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "paths": {
                "type": "array",
                "items": { "type": "string" },
                "description": "List of file paths to read"
            }
        },
        "required": ["paths"]
    }),
    category: "filesystem".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: false,
    timeout_seconds: 30.0,
    required_capabilities: vec!["file:read".into()],
    metadata: HashMap::new(),
});

pub struct FileReadMultipleTool;

impl BaseTool for FileReadMultipleTool {
    fn tool_id(&self) -> &str { "read_multiple_files" }
    fn spec(&self) -> &ToolSpec { &READ_MULTIPLE_SPEC }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let paths = params["paths"].as_array().ok_or_else(|| SUNDAYError::Tool(sunday_core::ToolError::InvalidParams("paths must be an array".into())))?;
        let mut results = String::new();

        for path_val in paths {
            let path_str = path_val.as_str().unwrap_or("");
            let path = Path::new(path_str);

            results.push_str(&format!("--- FILE: {} ---\n", path_str));
            if is_sensitive_file(path) {
                results.push_str("[Access Denied]\n\n");
                continue;
            }

            match std::fs::read_to_string(path) {
                Ok(content) => {
                    results.push_str(&content);
                    results.push_str("\n\n");
                }
                Err(e) => {
                    results.push_str(&format!("[Error reading file: {}]\n\n", e));
                }
            }
        }

        Ok(ToolResult::success(self.tool_id(), results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_read_sensitive_blocked() {
        let tool = FileReadTool;
        let result = tool
            .execute(&serde_json::json!({"path": ".env"}))
            .unwrap();
        assert!(!result.success);
        assert!(result.content.contains("sensitive"));
    }

    #[test]
    fn test_file_write_sensitive_blocked() {
        let tool = FileWriteTool;
        let result = tool
            .execute(&serde_json::json!({"path": "id_rsa", "content": "secret"}))
            .unwrap();
        assert!(!result.success);
    }
}
