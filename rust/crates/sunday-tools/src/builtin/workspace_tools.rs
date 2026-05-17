//! Workspace and Project management tools.

use crate::traits::BaseTool;
use sunday_core::{ToolResult, ToolSpec, SUNDAYError, ToolError};
use serde_json::{Value, json};
use std::path::{PathBuf};
use std::fs;
use once_cell::sync::Lazy;
use std::collections::HashMap;

static SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "project_create".into(),
    description: "Create a dedicated project folder for a task".into(),
    parameters: json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Name of the project folder"
            }
        },
        "required": ["name"]
    }),
    category: "workspace".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: false,
    timeout_seconds: 30.0,
    required_capabilities: vec!["file:write".into()],
    metadata: HashMap::new(),
});

pub struct ProjectWorkspaceTool;

impl ProjectWorkspaceTool {
    fn get_base_dir() -> PathBuf {
        PathBuf::from("workspaces")
    }
}

impl BaseTool for ProjectWorkspaceTool {
    fn tool_id(&self) -> &str {
        "project_create"
    }

    fn spec(&self) -> &ToolSpec {
        &SPEC
    }

    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let name = params["name"].as_str().ok_or_else(|| SUNDAYError::Tool(ToolError::Execution("name is required".into())))?;
        
        let base_dir = Self::get_base_dir();
        let project_dir = base_dir.join(name);

        if let Err(e) = fs::create_dir_all(&project_dir) {
            return Err(SUNDAYError::Io(e));
        }

        let absolute_path = fs::canonicalize(&project_dir).unwrap_or(project_dir.clone());
        tracing::info!("Created project workspace: {:?}", absolute_path);

        let mut metadata = HashMap::new();
        metadata.insert("project_name".to_string(), json!(name));
        metadata.insert("path".to_string(), json!(absolute_path.to_string_lossy()));

        Ok(ToolResult {
            tool_name: self.tool_id().to_string(),
            content: format!("Project workspace created at: {}", absolute_path.display()),
            success: true,
            usage: HashMap::new(),
            cost_usd: 0.0,
            latency_seconds: 0.0,
            metadata,
        })
    }
}
