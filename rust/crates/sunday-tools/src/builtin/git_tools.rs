//! Git tools — status, diff, log.

use crate::traits::BaseTool;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::process::Command;

fn run_git(args: &[&str], cwd: Option<&str>) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }
        Err(e) => Err(format!("Failed to run git: {}", e)),
    }
}

macro_rules! git_tool {
    ($struct_name:ident, $tool_id:expr, $desc:expr, $git_cmd:expr) => {
        static $struct_name: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
            name: $tool_id.into(),
            description: $desc.into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "description": "Repository directory (optional)" }
                }
            }),
            category: "git".into(),
            cost_estimate: 0.0,
            latency_estimate: 0.0,
            requires_confirmation: false,
            timeout_seconds: 10.0,
            required_capabilities: vec!["file:read".into()],
            metadata: HashMap::new(),
        });
    };
}

git_tool!(GIT_STATUS_SPEC, "git_status", "Show git status", "status");
git_tool!(GIT_DIFF_SPEC, "git_diff", "Show git diff", "diff");
git_tool!(GIT_LOG_SPEC, "git_log", "Show git log", "log");

static GIT_COMMIT_SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "git_commit".into(),
    description: "Stage files and create a git commit".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "message": { "type": "string", "description": "Commit message" },
            "files": { "type": "string", "description": "Files to stage (default: '.')" },
            "cwd": { "type": "string", "description": "Repository directory (optional)" }
        },
        "required": ["message"]
    }),
    category: "git".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: true,
    timeout_seconds: 15.0,
    required_capabilities: vec!["file:write".into()],
    metadata: HashMap::new(),
});

pub struct GitStatusTool;
impl BaseTool for GitStatusTool {
    fn tool_id(&self) -> &str { "git_status" }
    fn spec(&self) -> &ToolSpec { &GIT_STATUS_SPEC }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let cwd = params["cwd"].as_str();
        match run_git(&["status", "--short"], cwd) {
            Ok(output) => Ok(ToolResult::success("git_status", output)),
            Err(e) => Ok(ToolResult::failure("git_status", e)),
        }
    }
}

pub struct GitDiffTool;
impl BaseTool for GitDiffTool {
    fn tool_id(&self) -> &str { "git_diff" }
    fn spec(&self) -> &ToolSpec { &GIT_DIFF_SPEC }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let cwd = params["cwd"].as_str();
        match run_git(&["diff"], cwd) {
            Ok(output) => Ok(ToolResult::success("git_diff", output)),
            Err(e) => Ok(ToolResult::failure("git_diff", e)),
        }
    }
}

pub struct GitLogTool;
impl BaseTool for GitLogTool {
    fn tool_id(&self) -> &str { "git_log" }
    fn spec(&self) -> &ToolSpec { &GIT_LOG_SPEC }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let cwd = params["cwd"].as_str();
        let n = params["n"].as_i64().unwrap_or(10);
        match run_git(&["log", "--oneline", &format!("-{}", n)], cwd) {
            Ok(output) => Ok(ToolResult::success("git_log", output)),
            Err(e) => Ok(ToolResult::failure("git_log", e)),
        }
    }
}

pub struct GitCommitTool;
impl BaseTool for GitCommitTool {
    fn tool_id(&self) -> &str { "git_commit" }
    fn spec(&self) -> &ToolSpec { &GIT_COMMIT_SPEC }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let message = params["message"].as_str().unwrap_or("");
        let files = params["files"].as_str().unwrap_or(".");
        let cwd = params["cwd"].as_str();

        if message.is_empty() {
            return Ok(ToolResult::failure("git_commit", "Commit message is required"));
        }

        match run_git(&["add", files], cwd) {
            Ok(_) => {}
            Err(e) => return Ok(ToolResult::failure("git_commit", format!("git add failed: {}", e))),
        }

        match run_git(&["commit", "-m", message], cwd) {
            Ok(output) => Ok(ToolResult::success("git_commit", format!("Committed: {}", output.trim()))),
            Err(e) => Ok(ToolResult::failure("git_commit", format!("git commit failed: {}", e))),
        }
    }
}
