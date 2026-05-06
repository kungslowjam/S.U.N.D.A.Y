//! BaseTool trait — interface for all tool implementations.

use sunday_core::{ToolResult, ToolSpec};
use serde_json::Value;

/// Base trait for all tools.
pub trait BaseTool: Send + Sync {
    fn tool_id(&self) -> &str;
    fn spec(&self) -> &ToolSpec;
    fn execute(&self, params: &Value) -> Result<ToolResult, sunday_core::SUNDAYError>;

    /// Convert to OpenAI function calling format.
    fn to_openai_function(&self) -> Value {
        let spec = self.spec();
        serde_json::json!({
            "type": "function",
            "function": {
                "name": spec.name,
                "description": spec.description,
                "parameters": spec.parameters,
            }
        })
    }
}
