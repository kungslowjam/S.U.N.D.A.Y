//! Repository Mapping tool — generates a high-level overview of the project structure.

use crate::traits::BaseTool;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

static SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "repo_map".into(),
    description: "Generate a comprehensive map of the project, including file structure and key symbols".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "root": { "type": "string", "description": "Root directory to map (default: '.')" }
        }
    }),
    category: "coding".into(),
    cost_estimate: 0.0,
    latency_estimate: 2.0,
    requires_confirmation: false,
    timeout_seconds: 60.0,
    required_capabilities: vec!["file:read".into()],
    metadata: HashMap::new(),
});

pub struct RepoMapTool;

impl BaseTool for RepoMapTool {
    fn tool_id(&self) -> &str { "repo_map" }
    fn spec(&self) -> &ToolSpec { &SPEC }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let root_str = params["root"].as_str().unwrap_or(".");
        let root = Path::new(root_str);
        
        let mut map = String::new();
        map.push_str(&format!("--- REPOSITORY MAP: {} ---\n\n", root.display()));

        if let Err(e) = map_dir(root, &mut map, 0, 3) {
            return Ok(ToolResult::failure(self.tool_id(), format!("Failed to map directory: {}", e)));
        }

        Ok(ToolResult::success(self.tool_id(), map))
    }
}

fn map_dir(path: &Path, output: &mut String, depth: usize, max_depth: usize) -> std::io::Result<()> {
    if depth > max_depth {
        return Ok(());
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        
        // Filter hidden/ignored folders
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }
        
        entries.push(entry);
    }

    // Sort: directories first, then files
    entries.sort_by_key(|e| {
        let is_dir = e.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
        (!is_dir, e.file_name().to_string_lossy().to_lowercase())
    });

    for entry in entries {
        let entry_path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let indent = "  ".repeat(depth);
        let is_dir = entry.file_type()?.is_dir();

        if is_dir {
            output.push_str(&format!("{}📁 {}/\n", indent, name));
            let _ = map_dir(&entry_path, output, depth + 1, max_depth);
        } else {
            output.push_str(&format!("{}📄 {}\n", indent, name));
        }
    }

    Ok(())
}
