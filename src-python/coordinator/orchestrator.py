"""
Coordinator Orchestrator - Ties together DAG, Memory, and Exploration modules.
Provides high-level workflow orchestration for complex task execution.
"""

from __future__ import annotations

import asyncio
import time
from dataclasses import dataclass, field
from typing import Any, Callable, Coroutine

from .dag import TaskDAG, TaskStatus
from memory.knowledge_graph import KnowledgeGraph, MemoryType
from exploration.report_builder import ReportBuilder


@dataclass
class OrchestratorConfig:
    """Configuration for the orchestrator."""
    max_parallel_tasks: int = 4
    task_timeout_secs: float = 300.0
    enable_memory: bool = True
    enable_reporting: bool = True
    report_title: str = "Orchestrated Task Report"


class Orchestrator:
    """
    High-level workflow orchestrator combining:
    - TaskDAG for dependency management
    - KnowledgeGraph for context/memory
    - ReportBuilder for output
    """

    def __init__(self, config: OrchestratorConfig | None = None) -> None:
        self.config = config or OrchestratorConfig()
        self.dag = TaskDAG()
        self.memory = KnowledgeGraph() if self.config.enable_memory else None
        self.report = ReportBuilder(
            self.config.report_title,
            scope="orchestrated-workflow"
        )
        self._execution_start: float = 0.0

    def add_task(
        self,
        task_id: str,
        name: str,
        deps: list[str] | None = None,
        metadata: dict[str, Any] | None = None,
    ) -> Orchestrator:
        """Add a task to the DAG. Returns self for chaining."""
        self.dag.add_task(task_id, name, deps, metadata)
        return self

    def add_tasks_batch(self, tasks: list[dict[str, Any]]) -> Orchestrator:
        """Add multiple tasks at once."""
        for task in tasks:
            self.dag.add_task(
                task_id=task["id"],
                name=task["name"],
                deps=task.get("deps"),
                metadata=task.get("metadata"),
            )
        return self

    async def execute(
        self,
        executor_fn: Callable[[str], Coroutine[Any, Any, Any]],
    ) -> dict[str, Any]:
        """
        Execute all tasks in the DAG.

        Args:
            executor_fn: Async function that takes task_id and returns result

        Returns:
            Dict with execution results and metadata
        """
        self._execution_start = time.time()

        # Record to memory if enabled
        if self.memory:
            self.memory.add(
                f"execution_{int(time.time())}",
                f"Started execution of {len(self.dag._nodes)} tasks",
                MemoryType.PROJECT,
                importance=0.8,
            )

        # Execute DAG
        results = await self.dag.execute(executor_fn, self.config.max_parallel_tasks)

        # Record completion
        duration = time.time() - self._execution_start

        if self.memory:
            completed = self.dag.get_completed_tasks()
            failed = self.dag.get_failed_tasks()

            self.memory.add(
                f"execution_complete_{int(time.time())}",
                f"Execution completed: {len(completed)} succeeded, {len(failed)} failed in {duration:.1f}s",
                MemoryType.FACT,
                importance=0.9,
            )

        return {
            "results": {
                tid: {
                    "status": r.status.name,
                    "result": r.result,
                    "error": r.error,
                    "duration_ms": r.duration_ms,
                }
                for tid, r in results.items()
            },
            "completed": self.dag.get_completed_tasks(),
            "failed": self.dag.get_failed_tasks(),
            "duration_secs": duration,
        }

    def execute_sync(
        self,
        executor_fn: Callable[[str], Any],
    ) -> dict[str, Any]:
        """Synchronous wrapper around execute."""
        return asyncio.run(self.execute(executor_fn))

    def generate_report(self) -> str:
        """Generate a markdown report of the execution."""
        if not self.config.enable_reporting:
            return ""

        # Add execution summary section
        self.report.add_chapter("Execution Summary")

        completed = self.dag.get_completed_tasks()
        failed = self.dag.get_failed_tasks()

        if not failed:
            self.report.add_finding(
                "All Tasks Completed",
                f"Successfully executed {len(completed)} tasks",
                severity="info",
            )
        else:
            self.report.add_finding(
                "Some Tasks Failed",
                f"{len(failed)} tasks failed: {failed}",
                severity="critical",
            )

        self.report.set_completed()
        return self.report.build_markdown()

    def get_context(self, query: str, limit: int = 5) -> list[Any]:
        """Search memory for relevant context."""
        if not self.memory:
            return []

        results = self.memory.search(query, limit=limit)
        return [r.entry for r in results]

    def store_context(
        self,
        name: str,
        content: str,
        memory_type: MemoryType,
        importance: float = 0.5,
    ) -> None:
        """Store information in memory."""
        if self.memory:
            self.memory.add(name, content, memory_type, importance=importance)


def create_orchestrator(
    title: str = "Task Orchestration",
    max_parallel: int = 4,
) -> Orchestrator:
    """Factory function to create an orchestrator with defaults."""
    config = OrchestratorConfig(
        report_title=title,
        max_parallel_tasks=max_parallel,
    )
    return Orchestrator(config)
