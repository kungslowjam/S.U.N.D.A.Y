use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    #[serde(rename = "agent")]
    Agent,
    #[serde(rename = "tool")]
    Tool,
    #[serde(rename = "condition")]
    Condition,
    #[serde(rename = "transform")]
    Transform,
    #[serde(rename = "loop")]
    Loop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    pub node_type: NodeType,
    pub agent: Option<String>,
    pub tools: Option<Vec<String>>,
    pub config: HashMap<String, serde_json::Value>,
    pub condition_expr: Option<String>,
    pub transform_expr: Option<String>,
    pub max_iterations: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub source: String,
    pub target: String,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepResult {
    pub node_id: String,
    pub success: bool,
    pub output: String,
    pub duration_seconds: f64,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub workflow_name: String,
    pub success: bool,
    pub steps: Vec<WorkflowStepResult>,
    pub final_output: String,
    pub total_duration_seconds: f64,
}
