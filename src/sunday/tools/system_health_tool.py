import psutil
from datetime import datetime
from typing import Any, Dict, List, Optional

from sunday.core.registry import ToolRegistry
from sunday.core.types import ToolResult
from sunday.tools._stubs import BaseTool, ToolSpec

@ToolRegistry.register("system_health")
class SystemHealthTool(BaseTool):
    """Tool to report system health status (CPU, RAM, Disk)."""
    
    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="system_health",
            description="Get a real-time report of system resources including CPU usage, RAM availability, and Disk space.",
            parameters={
                "type": "object",
                "properties": {
                    "include_details": {"type": "boolean", "description": "Whether to include detailed per-core information.", "default": False}
                }
            },
            category="system"
        )

    def execute(self, include_details: bool = False, **kwargs: Any) -> ToolResult:
        try:
            cpu = psutil.cpu_percent(interval=0.5)
            ram = psutil.virtual_memory()
            disk = psutil.disk_usage('/')
            
            report = (
                f"### System Health Report ({datetime.now().strftime('%H:%M:%S')})\n"
                f"- **CPU Usage**: {cpu:.1f}%\n"
                f"- **RAM Usage**: {ram.percent:.1f}% ({ram.used // (1024**2)}MB / {ram.total // (1024**2)}MB)\n"
                f"- **Disk Free**: {disk.free // (1024**3)}GB free out of {disk.total // (1024**3)}GB"
            )
            
            return ToolResult(
                tool_name="system_health",
                content=report,
                success=True,
                metadata={
                    "cpu": cpu,
                    "ram_percent": ram.percent,
                    "disk_free_gb": disk.free // (1024**3)
                }
            )
        except Exception as e:
            return ToolResult(
                tool_name="system_health",
                content=f"Failed to get system health: {str(e)}",
                success=False
            )
