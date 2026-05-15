"""Graphify tool — architectural codebase visualization and knowledge graph extraction."""

from __future__ import annotations

import json
import logging
import os
import subprocess
from pathlib import Path
from typing import Any, List, Optional

from sunday.core.registry import ToolRegistry
from sunday.core.types import ToolResult
from sunday.tools._stubs import BaseTool, ToolSpec

logger = logging.getLogger(__name__)


@ToolRegistry.register("graphify")
class GraphifyTool(BaseTool):
    """Index a codebase into a knowledge graph using Graphify."""

    tool_id = "graphify"

    def __init__(self, output_dir: str = "graphify-out"):
        self._output_dir = output_dir

    @property
    def spec(self) -> ToolSpec:
        return ToolSpec(
            name="graphify",
            description=(
                "Analyze a codebase and generate an architectural knowledge graph. "
                "Provides a 'Big Picture' view including dependency chains, call graphs, "
                "and 'god nodes' (highly connected components)."
            ),
            parameters={
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the project directory to analyze. Default: '.'",
                    },
                    "force_reindex": {
                        "type": "boolean",
                        "description": "If true, bypass cache and re-analyze all files.",
                    },
                    "report_only": {
                        "type": "boolean",
                        "description": "If true, return the existing GRAPH_REPORT.md if it exists without re-running.",
                    },
                },
            },
            category="code",
            timeout_seconds=300.0,  # Indexing large repos can take time
        )

    def execute(self, **params: Any) -> ToolResult:
        project_path = params.get("path", ".")
        force = params.get("force_reindex", False)
        report_only = params.get("report_only", False)

        abs_path = Path(project_path).resolve()
        out_dir = abs_path / self._output_dir
        report_path = out_dir / "GRAPH_REPORT.md"
        json_path = out_dir / "graph.json"

        if report_only and report_path.exists():
            return ToolResult(
                tool_name="graphify",
                content=report_path.read_text(encoding="utf-8"),
                success=True,
                metadata={"path": str(report_path)},
            )

        # Run graphify
        # Note: 'graphify' command comes from 'graphifyy' package
        cmd = ["graphify", str(abs_path)]
        if force:
            # Check if graphify has a force flag or if we just delete out_dir
            if out_dir.exists():
                import shutil
                shutil.rmtree(out_dir)

        try:
            # Set environment to suppress interactive prompts if any
            env = os.environ.copy()
            env["PYTHONUNBUFFERED"] = "1"
            
            process = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                cwd=str(abs_path),
                env=env,
                timeout=self.spec.timeout_seconds,
            )

            if process.returncode != 0:
                error_msg = f"Graphify failed (exit {process.returncode}):\n{process.stderr}"
                logger.error(error_msg)
                return ToolResult(
                    tool_name="graphify",
                    content=error_msg,
                    success=False,
                )

            # Collect results
            content = ""
            if report_path.exists():
                content = report_path.read_text(encoding="utf-8")
            elif json_path.exists():
                # Summarize JSON if report is missing
                with open(json_path, "r", encoding="utf-8") as f:
                    data = json.load(f)
                    nodes = data.get("nodes", [])
                    edges = data.get("edges", [])
                    content = f"Graph generated successfully.\nNodes: {len(nodes)}\nEdges: {len(edges)}"
            else:
                content = f"Graphify finished but no output found in {out_dir}.\nSTDOUT: {process.stdout}"

            return ToolResult(
                tool_name="graphify",
                content=content,
                success=True,
                metadata={
                    "stdout": process.stdout,
                    "output_dir": str(out_dir),
                },
            )

        except subprocess.TimeoutExpired:
            return ToolResult(
                tool_name="graphify",
                content="Graphify analysis timed out.",
                success=False,
            )
        except Exception as exc:
            return ToolResult(
                tool_name="graphify",
                content=f"Error running Graphify: {exc}",
                success=False,
            )


__all__ = ["GraphifyTool"]
