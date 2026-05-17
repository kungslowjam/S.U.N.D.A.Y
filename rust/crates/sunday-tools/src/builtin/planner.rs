//! Task Planning and Management tool.

use crate::traits::BaseTool;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;

static SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "task_planner".into(),
    description: "Create, update, and track a multi-step plan for a complex objective".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "objective": { "type": "string", "description": "The high-level goal" },
            "action": { "type": "string", "enum": ["create", "update", "status"], "description": "Action to perform" },
            "steps": { "type": "array", "items": { "type": "string" }, "description": "List of steps (for 'create' or 'update')" },
            "completed_step_index": { "type": "integer", "description": "Index of the step that was just completed" }
        },
        "required": ["objective", "action"]
    }),
    category: "management".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: false,
    timeout_seconds: 10.0,
    required_capabilities: vec![],
    metadata: HashMap::new(),
});

pub struct TaskPlannerTool;

impl BaseTool for TaskPlannerTool {
    fn tool_id(&self) -> &str { "task_planner" }
    fn spec(&self) -> &ToolSpec { &SPEC }
    fn execute(&self, params: &Value) -> Result<ToolResult, SUNDAYError> {
        let objective = params["objective"].as_str().unwrap_or("");
        let action = params["action"].as_str().unwrap_or("status");
        
        // This tool primarily manages the agent's internal state/context about the task.
        // The result is returned to the agent to help it maintain its own roadmap.
        let result_msg = match action {
            "create" => format!("✅ Plan created for objective: '{}'. Steps: {:?}", objective, params["steps"]),
            "update" => format!("🔄 Plan updated for: '{}'.", objective),
            "status" => format!("📊 Current status of objective: '{}'.", objective),
            _ => "Unknown action".to_string(),
        };

        Ok(ToolResult::success(self.tool_id(), result_msg))
    }
}
