//! System health tool — report CPU, memory, and disk usage.

use crate::traits::BaseTool;
use sunday_core::{SUNDAYError, ToolResult, ToolSpec};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use sysinfo::{System, Disks};

static SPEC: Lazy<ToolSpec> = Lazy::new(|| ToolSpec {
    name: "system_health".into(),
    description: "Report CPU, memory, and disk usage of the current system".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {},
        "required": []
    }),
    category: "system".into(),
    cost_estimate: 0.0,
    latency_estimate: 0.0,
    requires_confirmation: false,
    timeout_seconds: 5.0,
    required_capabilities: vec![],
    metadata: HashMap::new(),
});

pub struct SystemHealthTool;

impl BaseTool for SystemHealthTool {
    fn tool_id(&self) -> &str {
        "system_health"
    }

    fn spec(&self) -> &ToolSpec {
        &SPEC
    }

    fn execute(&self, _params: &Value) -> Result<ToolResult, SUNDAYError> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_usage = sys.global_cpu_usage();
        let cpu_cores = sys.cpus().len();

        let total_mem = sys.total_memory();
        let used_mem = sys.used_memory();
        let free_mem = sys.free_memory();
        let mem_percent = if total_mem > 0 {
            (used_mem as f64 / total_mem as f64) * 100.0
        } else {
            0.0
        };

        let disks = Disks::new_with_refreshed_list();
        let mut disk_info = Vec::new();
        for disk in &disks {
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total.saturating_sub(available);
            let percent = if total > 0 {
                (used as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            let name = disk.mount_point().to_string_lossy();
            disk_info.push(format!(
                "  {}: {}/{} used ({:.1}%)",
                name,
                format_bytes(used),
                format_bytes(total),
                percent
            ));
        }

        let output = format!(
            "System Health Report:\n\n\
            CPU: {:.1}% usage ({} cores)\n\
            Memory: {}/{} used ({:.1}%) — {} free\n\n\
            Disks:\n{}",
            cpu_usage,
            cpu_cores,
            format_bytes(used_mem * 1024),
            format_bytes(total_mem * 1024),
            mem_percent,
            format_bytes(free_mem * 1024),
            disk_info.join("\n")
        );

        Ok(ToolResult::success("system_health", output))
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes < 1024u64 * 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else {
        format!("{:.2} TB", bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_health() {
        let tool = SystemHealthTool;
        let result = tool.execute(&serde_json::json!({})).unwrap();
        assert!(result.success);
        assert!(result.content.contains("CPU:"));
        assert!(result.content.contains("Memory:"));
        assert!(result.content.contains("Disks:"));
    }
}
