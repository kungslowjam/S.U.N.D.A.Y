//! ToolExecutor — central dispatch with RBAC, taint, timeout.

use crate::builtin::BuiltinTool;
use crate::traits::BaseTool;
use sunday_core::error::{SUNDAYError, ToolError};
use sunday_core::{EventBus, EventType, ToolResult};
use sunday_security::capabilities::CapabilityPolicy;
use sunday_security::taint::{TaintSet, check_taint};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub struct ToolExecutor {
    tools: HashMap<String, BuiltinTool>,
    capability_policy: Option<CapabilityPolicy>,
    path_guard: Option<sunday_security::capabilities::PathGuard>,
    bus: Option<Arc<EventBus>>,
    default_timeout: Duration,
}

impl ToolExecutor {
    pub fn new(
        capability_policy: Option<CapabilityPolicy>,
        path_guard: Option<sunday_security::capabilities::PathGuard>,
        bus: Option<Arc<EventBus>>,
    ) -> Self {
        Self {
            tools: HashMap::new(),
            capability_policy,
            path_guard,
            bus,
            default_timeout: Duration::from_secs(30),
        }
    }

    pub fn with_builtins(
        capability_policy: Option<CapabilityPolicy>,
        path_guard: Option<sunday_security::capabilities::PathGuard>,
        bus: Option<Arc<EventBus>>,
    ) -> Self {
        let mut exec = Self::new(capability_policy, path_guard, bus);
        
        use crate::builtin::*;
        exec.register(BuiltinTool::ApplyPatch(ApplyPatchTool));
        exec.register(BuiltinTool::BrowserNavigate(BrowserNavigateTool));
        exec.register(BuiltinTool::BrowserScreenshot(BrowserScreenshotTool));
        exec.register(BuiltinTool::BrowserClick(BrowserClickTool));
        exec.register(BuiltinTool::BrowserType(BrowserTypeTool));
        exec.register(BuiltinTool::BrowserViewTree(BrowserViewTreeTool));
        exec.register(BuiltinTool::Calculator(CalculatorTool));
        exec.register(BuiltinTool::Think(ThinkTool));
        exec.register(BuiltinTool::FileEdit(FileEditTool));
        exec.register(BuiltinTool::FileRead(FileReadTool));
        exec.register(BuiltinTool::FileReadMultiple(FileReadMultipleTool));
        exec.register(BuiltinTool::FileWrite(FileWriteTool));
        exec.register(BuiltinTool::FileGrep(FileGrepTool));
        exec.register(BuiltinTool::ListDirectory(ListDirectoryTool));
        exec.register(BuiltinTool::ShellExec(ShellExecTool));
        exec.register(BuiltinTool::HttpRequest(HttpRequestTool));
        exec.register(BuiltinTool::WebSearch(WebSearchTool));
        exec.register(BuiltinTool::GitStatus(GitStatusTool));
        exec.register(BuiltinTool::GitDiff(GitDiffTool));
        exec.register(BuiltinTool::GitLog(GitLogTool));
        exec.register(BuiltinTool::GitCommit(GitCommitTool));
        exec.register(BuiltinTool::CryptoPrice(CryptoPriceTool));
        exec.register(BuiltinTool::WebFetch(WebFetchTool));
        exec.register(BuiltinTool::SystemHealth(SystemHealthTool));
        exec.register(BuiltinTool::ScanChunks(ScanChunksTool));
        
        exec
    }

    pub fn register(&mut self, tool: BuiltinTool) {
        let id = tool.tool_id().to_string();
        self.tools.insert(id, tool);
    }

    pub fn get_tool(&self, name: &str) -> Option<&BuiltinTool> {
        self.tools.get(name)
    }

    pub fn list_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub fn tool_specs(&self) -> Vec<Value> {
        self.tools.values().map(|t| t.to_openai_function()).collect()
    }

    pub fn execute(
        &self,
        tool_name: &str,
        params: &Value,
        agent_id: Option<&str>,
        taint: Option<&TaintSet>,
    ) -> Result<ToolResult, SUNDAYError> {
        let tool = self.tools.get(tool_name).ok_or_else(|| {
            SUNDAYError::Tool(ToolError::NotFound(tool_name.to_string()))
        })?;

        // RBAC check
        if let (Some(policy), Some(aid)) = (&self.capability_policy, agent_id) {
            let spec = tool.spec();
            for cap in &spec.required_capabilities {
                if !policy.check(aid, cap, "") {
                    return Err(SUNDAYError::Tool(ToolError::CapabilityDenied(
                        aid.to_string(),
                        format!("{} (tool: {})", cap, tool_name),
                    )));
                }
            }
        }

        // Taint check
        if let Some(taint_set) = taint {
            if let Some(violation) = check_taint(tool_name, taint_set) {
                return Err(SUNDAYError::Tool(ToolError::TaintViolation(
                    tool_name.to_string(),
                    violation,
                )));
            }
        }

        // Path check
        if let Some(ref guard) = self.path_guard {
            for key in ["path", "cwd", "dir"] {
                if let Some(Value::String(p)) = params.get(key) {
                    if !guard.is_safe(std::path::Path::new(p)) {
                        return Err(SUNDAYError::Security(
                            sunday_core::error::SecurityError::PolicyViolation(format!(
                                "Path access denied: {} (parameter: {})",
                                p, key
                            )),
                        ));
                    }
                }
            }
        }

        // Emit start event
        if let Some(ref bus) = self.bus {
            let data = serde_json::json!({
                "tool_name": tool_name,
                "arguments": params
            });
            bus.publish(EventType::ToolCallStart, data);
        }

        let start = std::time::Instant::now();
        let timeout = Duration::from_secs_f64(tool.spec().timeout_seconds);
        let timeout = if timeout.is_zero() { self.default_timeout } else { timeout };

        let result = tool.execute(params);
        let elapsed = start.elapsed();

        if elapsed > timeout {
            if let Some(ref bus) = self.bus {
                let data = serde_json::json!({
                    "tool_name": tool_name
                });
                bus.publish(EventType::ToolTimeout, data);
            }
            return Err(SUNDAYError::Tool(ToolError::Timeout(
                timeout.as_secs_f64(),
                tool_name.to_string(),
            )));
        }

        // Emit end event
        if let Some(ref bus) = self.bus {
            let succ = result.as_ref().map(|r| r.success).unwrap_or(false);
            let content = result.as_ref().map(|r| r.content.clone()).unwrap_or_else(|e| e.to_string());
            let data = serde_json::json!({
                "tool_name": tool_name,
                "duration_seconds": elapsed.as_secs_f64(),
                "success": succ,
                "result": content
            });
            bus.publish(EventType::ToolCallEnd, data);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_register_and_execute() {
        let mut exec = ToolExecutor::new(None, None, None);
        exec.register(BuiltinTool::Calculator(crate::builtin::calculator::CalculatorTool));
        let result = exec
            .execute("calculator", &serde_json::json!({"expression": "2+2"}), None, None)
            .unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_executor_tool_not_found() {
        let exec = ToolExecutor::new(None, None, None);
        let err = exec
            .execute("nonexistent", &serde_json::json!({}), None, None)
            .unwrap_err();
        assert!(matches!(err, SUNDAYError::Tool(ToolError::NotFound(_))));
    }
}
