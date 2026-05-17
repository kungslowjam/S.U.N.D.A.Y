use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Result;
use tracing::{info, warn};
use chrono::Utc;

use crate::types::{WorkflowNode, WorkflowResult, WorkflowStepResult, NodeType};
use crate::graph::WorkflowGraph;

pub trait WorkflowSystem: Send + Sync {
    fn execute_agent(&self, input: &str, agent: Option<&str>, tools: Option<&[String]>) -> Result<String>;
    fn execute_tool(&self, name: &str, args: &str) -> Result<String>;
}

pub struct WorkflowEngine {
    _max_parallel: usize,
}

impl WorkflowEngine {
    pub fn new(max_parallel: usize) -> Self {
        Self { _max_parallel: max_parallel }
    }

    pub async fn run(
        &self,
        graph: &WorkflowGraph,
        system: Arc<dyn WorkflowSystem>,
        initial_input: String,
    ) -> Result<WorkflowResult> {
        info!("Starting workflow execution: {}", graph.name);
        let start_time = Utc::now();
        
        let mut outputs: HashMap<String, String> = HashMap::new();
        outputs.insert("_input".to_string(), initial_input);
        
        let mut all_steps = Vec::new();
        let mut success = true;

        let stages = graph.execution_stages();
        for stage in stages {
            let mut futures = Vec::new();
            for node_id in stage {
                let node = graph.get_node(&node_id).unwrap().clone();
                let system_clone = system.clone();
                let outputs_clone = outputs.clone();
                let preds = graph.predecessors(&node_id);
                
                futures.push(tokio::spawn(async move {
                    Self::execute_node(node, system_clone, outputs_clone, preds).await
                }));
            }

            let results = futures::future::join_all(futures).await;
            for res in results {
                match res {
                    Ok(Ok(step)) => {
                        outputs.insert(step.node_id.clone(), step.output.clone());
                        if !step.success {
                            success = false;
                        }
                        all_steps.push(step);
                    }
                    Ok(Err(e)) => {
                        warn!("Node execution failed: {}", e);
                        success = false;
                    }
                    Err(e) => {
                        warn!("Task join failed: {}", e);
                        success = false;
                    }
                }
            }

            if !success { break; }
        }

        let end_time = Utc::now();
        let final_output = all_steps.last().map(|s| s.output.clone()).unwrap_or_default();

        Ok(WorkflowResult {
            workflow_name: graph.name.clone(),
            success,
            steps: all_steps,
            final_output,
            total_duration_seconds: (end_time - start_time).num_milliseconds() as f64 / 1000.0,
        })
    }

    async fn execute_node(
        node: WorkflowNode,
        system: Arc<dyn WorkflowSystem>,
        outputs: HashMap<String, String>,
        preds: Vec<String>,
    ) -> Result<WorkflowStepResult> {
        let start_time = Utc::now();
        
        // Prepare input
        let input = if !preds.is_empty() {
            preds.iter()
                .filter_map(|p| outputs.get(p))
                .cloned()
                .collect::<Vec<_>>()
                .join("\n\n")
        } else {
            outputs.get("_input").cloned().unwrap_or_default()
        };

        let result = match node.node_type {
            NodeType::Agent => {
                let agent_name = node.agent.as_deref();
                let tools = node.tools.as_deref();
                system.execute_agent(&input, agent_name, tools)
                    .map(|out| (true, out))
                    .unwrap_or_else(|e| (false, e.to_string()))
            }
            NodeType::Tool => {
                let tool_name = node.config.get("tool_name").and_then(|v| v.as_str()).unwrap_or("");
                let tool_args = node.config.get("tool_args").and_then(|v| v.as_str()).unwrap_or("{}");
                system.execute_tool(tool_name, tool_args)
                    .map(|out| (true, out))
                    .unwrap_or_else(|e| (false, e.to_string()))
            }
            NodeType::Condition => {
                // Simplified condition evaluation for now
                let res = if let Some(expr) = &node.condition_expr {
                    if expr.contains("success") { "true" } else { "false" }
                } else {
                    "true"
                };
                (true, res.to_string())
            }
            NodeType::Transform => {
                let out = match node.transform_expr.as_deref() {
                    Some("concatenate") => input,
                    _ => input,
                };
                (true, out)
            }
            NodeType::Loop => {
                // Loop would need its own internal loop logic, simplified for MVP
                (true, "Loop executed".to_string())
            }
        };

        let end_time = Utc::now();
        Ok(WorkflowStepResult {
            node_id: node.id,
            success: result.0,
            output: result.1,
            duration_seconds: (end_time - start_time).num_milliseconds() as f64 / 1000.0,
            metadata: HashMap::new(),
        })
    }
}
