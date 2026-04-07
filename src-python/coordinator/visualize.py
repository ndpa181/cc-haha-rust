"""
Visualization utilities for TaskDAG - Mermaid, Graphviz, JSON formats.
"""

from __future__ import annotations

from typing import Any
from .dag import TaskDAG, TaskNode, TaskStatus


class DAGVisualizer:
    """Generate various visualizations of a TaskDAG."""

    def __init__(self, dag: TaskDAG) -> None:
        self.dag = dag

    def to_mermaid(self, title: str = "Task DAG") -> str:
        """Generate Mermaid flowchart."""
        lines = [f"## {title}\n", "```mermaid", "flowchart TD"]

        # Add styling
        lines.append("    %% Task nodes")
        for task_id, node in self.dag._nodes.items():
            status_suffix = self._status_suffix(node.status)
            lines.append(f'    {task_id}{status_suffix}["{node.name}"]')

        lines.append("")

        # Add dependency edges
        lines.append("    %% Dependencies")
        for task_id, node in self.dag._nodes.items():
            for dep in node.deps:
                lines.append(f"    {dep} --> {task_id}")

        lines.append("```")

        # Add legend
        lines.append("\n### Legend")
        lines.append("- 🟡 Yellow: Running")
        lines.append("- 🟢 Green: Completed")
        lines.append("- 🔴 Red: Failed")
        lines.append("- ⬜ Gray: Pending")

        return "\n".join(lines)

    def to_graphviz(self) -> str:
        """Generate Graphviz DOT format."""
        lines = [
            "digraph TaskDAG {",
            '    rankdir=TB;',
            '    node [shape=box, style="rounded,filled"];',
            '    edge [arrowhead=normal];',
            "",
        ]

        # Add nodes with colors
        for task_id, node in self.dag._nodes.items():
            color = self._graphviz_color(node.status)
            label = node.name.replace('"', '\\"')
            lines.append(f'    "{task_id}" [label="{label}", fillcolor="{color}"];')

        lines.append("")

        # Add edges
        for task_id, node in self.dag._nodes.items():
            for dep in node.deps:
                lines.append(f'    "{dep}" -> "{task_id}";')

        lines.append("}")

        return "\n".join(lines)

    def to_json(self) -> dict[str, Any]:
        """Generate JSON representation."""
        nodes = []
        for task_id, node in self.dag._nodes.items():
            nodes.append({
                "id": task_id,
                "name": node.name,
                "status": node.status.name,
                "deps": node.deps,
                "metadata": node.metadata,
                "result": node.result,
                "error": node.error,
            })

        return {
            "node_count": len(nodes),
            "execution_levels": self.dag.get_execution_order(),
            "completed": self.dag.get_completed_tasks(),
            "failed": self.dag.get_failed_tasks(),
            "nodes": nodes,
        }

    def to_markdown_table(self) -> str:
        """Generate a markdown table of task status."""
        lines = [
            "| Task ID | Name | Status | Dependencies |",
            "|----------|------|--------|---------------|",
        ]

        for task_id, node in self.dag._nodes.items():
            deps_str = ", ".join(node.deps) if node.deps else "-"
            status_icon = self._status_icon(node.status)
            lines.append(f"| {task_id} | {node.name} | {status_icon} {node.status.name} | {deps_str} |")

        return "\n".join(lines)

    def _status_suffix(self, status: TaskStatus) -> str:
        """Get Mermaid shape suffix for status."""
        return {
            TaskStatus.PENDING: "",
            TaskStatus.RUNNING: "{O}",
            TaskStatus.COMPLETED: "((",
            TaskStatus.FAILED: "{/}",
            TaskStatus.BLOCKED: "[>]",
        }.get(status, "")

    def _status_color(self, status: TaskStatus) -> str:
        """Get Mermaid color for status."""
        return {
            TaskStatus.PENDING: "gray",
            TaskStatus.RUNNING: "yellow",
            TaskStatus.COMPLETED: "green",
            TaskStatus.FAILED: "red",
            TaskStatus.BLOCKED: "orange",
        }.get(status, "gray")

    def _graphviz_color(self, status: TaskStatus) -> str:
        """Get Graphviz color for status."""
        return {
            TaskStatus.PENDING: "#E8E8E8",
            TaskStatus.RUNNING: "#FFF3CD",
            TaskStatus.COMPLETED: "#D4EDDA",
            TaskStatus.FAILED: "#F8D7DA",
            TaskStatus.BLOCKED: "#FFE5CC",
        }.get(status, "#E8E8E8")

    def _status_icon(self, status: TaskStatus) -> str:
        """Get emoji icon for status."""
        return {
            TaskStatus.PENDING: "⬜",
            TaskStatus.RUNNING: "🟡",
            TaskStatus.COMPLETED: "🟢",
            TaskStatus.FAILED: "🔴",
            TaskStatus.BLOCKED: "🟠",
        }.get(status, "⬜")
