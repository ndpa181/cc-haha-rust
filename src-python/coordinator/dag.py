"""
TaskDAG: Directed Acyclic Graph for task dependency management.
Manages task ordering, parallel execution, and dependency resolution.
"""

from __future__ import annotations

import asyncio
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Any, Callable, Coroutine
from collections import deque


class TaskStatus(Enum):
    """Task execution status."""
    PENDING = auto()
    RUNNING = auto()
    COMPLETED = auto()
    FAILED = auto()
    BLOCKED = auto()


@dataclass
class TaskNode:
    """Represents a single task in the DAG."""
    id: str
    name: str
    deps: list[str] = field(default_factory=list)
    status: TaskStatus = TaskStatus.PENDING
    result: Any = None
    error: str | None = None
    metadata: dict[str, Any] = field(default_factory=dict)

    def __hash__(self) -> int:
        return hash(self.id)


@dataclass
class ExecutionResult:
    """Result of a DAG execution."""
    task_id: str
    status: TaskStatus
    result: Any = None
    error: str | None = None
    duration_ms: float = 0.0


class TaskDAG:
    """
    Manages task dependencies and execution ordering.

    Features:
    - Topological sorting for execution order
    - Parallel execution of independent tasks
    - Dependency validation (no cycles)
    - Status tracking and error propagation
    """

    def __init__(self) -> None:
        self._nodes: dict[str, TaskNode] = {}
        self._adjacency: dict[str, list[str]] = {}  # task -> tasks that depend on it
        self._reverse_adjacency: dict[str, list[str]] = {}  # task -> tasks it depends on

    def add_task(
        self,
        task_id: str,
        name: str,
        deps: list[str] | None = None,
        metadata: dict[str, Any] | None = None,
    ) -> TaskDAG:
        """Add a task to the DAG. Returns self for chaining."""
        if task_id in self._nodes:
            raise ValueError(f"Task '{task_id}' already exists")

        node = TaskNode(
            id=task_id,
            name=name,
            deps=deps or [],
            metadata=metadata or {},
        )
        self._nodes[task_id] = node
        self._adjacency[task_id] = []
        self._reverse_adjacency[task_id] = []

        # Build adjacency lists
        for dep in node.deps:
            if dep not in self._nodes:
                raise ValueError(f"Dependency '{dep}' for task '{task_id}' not found")
            self._adjacency[dep].append(task_id)
            self._reverse_adjacency[task_id].append(dep)

        return self

    def get_execution_order(self) -> list[list[str]]:
        """
        Get tasks grouped by execution level (BFS topological sort).
        Tasks in the same group can be executed in parallel.
        """
        in_degree: dict[str, int] = {tid: len(self._reverse_adjacency.get(tid, [])) for tid in self._nodes}
        levels: list[list[str]] = []

        queue: deque[str] = deque([tid for tid, deg in in_degree.items() if deg == 0])

        while queue:
            level: list[str] = []
            next_queue: deque[str] = deque()

            for task_id in queue:
                level.append(task_id)
                for dependent in self._adjacency.get(task_id, []):
                    in_degree[dependent] -= 1
                    if in_degree[dependent] == 0:
                        next_queue.append(dependent)

            levels.append(level)
            queue = next_queue

        # Check for cycles
        if sum(len(level) for level in levels) != len(self._nodes):
            raise ValueError("Cycle detected in task dependency graph")

        return levels

    def get_ready_tasks(self) -> list[str]:
        """Get tasks that are ready to execute (all deps satisfied)."""
        ready = []
        for task_id, node in self._nodes.items():
            if node.status != TaskStatus.PENDING:
                continue
            deps_satisfied = all(
                self._nodes[dep].status == TaskStatus.COMPLETED
                for dep in node.deps
            )
            if deps_satisfied:
                ready.append(task_id)
        return ready

    def is_blocked(self, task_id: str) -> bool:
        """Check if a task is blocked by unfinished dependencies."""
        node = self._nodes.get(task_id)
        if not node:
            return False
        return any(
            self._nodes[dep].status not in (TaskStatus.COMPLETED, TaskStatus.FAILED)
            for dep in node.deps
        )

    async def execute(
        self,
        executor: Callable[[str], Coroutine[Any, Any, Any]],
        max_parallel: int = 4,
    ) -> dict[str, ExecutionResult]:
        """
        Execute all tasks respecting dependencies.

        Args:
            executor: Async function that takes task_id and returns result
            max_parallel: Maximum parallel task executions

        Returns:
            Dict mapping task_id to ExecutionResult
        """
        results: dict[str, ExecutionResult] = {}
        active_tasks: set[str] = set()

        # Initialize all tasks to pending
        for node in self._nodes.values():
            node.status = TaskStatus.PENDING

        execution_levels = self.get_execution_order()

        for level in execution_levels:
            # Execute all tasks in this level (they're independent)
            level_tasks = [tid for tid in level if self._nodes[tid].status == TaskStatus.PENDING]

            if not level_tasks:
                continue

            # Run in parallel with semaphore for max_parallel
            semaphore = asyncio.Semaphore(max_parallel)

            async def run_task(task_id: str) -> ExecutionResult:
                import time
                start = time.perf_counter()
                node = self._nodes[task_id]
                node.status = TaskStatus.RUNNING

                try:
                    result = await executor(task_id)
                    node.status = TaskStatus.COMPLETED
                    node.result = result
                    duration = (time.perf_counter() - start) * 1000
                    return ExecutionResult(
                        task_id=task_id,
                        status=TaskStatus.COMPLETED,
                        result=result,
                        duration_ms=duration,
                    )
                except Exception as e:
                    node.status = TaskStatus.FAILED
                    node.error = str(e)
                    duration = (time.perf_counter() - start) * 1000
                    return ExecutionResult(
                        task_id=task_id,
                        status=TaskStatus.FAILED,
                        error=str(e),
                        duration_ms=duration,
                    )

            async def run_with_semaphore(tid: str) -> ExecutionResult:
                async with semaphore:
                    return await run_task(tid)

            tasks = [run_with_semaphore(tid) for tid in level_tasks]
            level_results = await asyncio.gather(*tasks)

            for result in level_results:
                results[result.task_id] = result

        return results

    def get_failed_tasks(self) -> list[str]:
        """Get list of failed task IDs."""
        return [
            tid for tid, node in self._nodes.items()
            if node.status == TaskStatus.FAILED
        ]

    def get_completed_tasks(self) -> list[str]:
        """Get list of completed task IDs."""
        return [
            tid for tid, node in self._nodes.items()
            if node.status == TaskStatus.COMPLETED
        ]

    def visualize(self) -> str:
        """Generate Mermaid flowchart representation."""
        lines = ["flowchart TD"]
        for task_id, node in self._nodes.items():
            status_color = {
                TaskStatus.PENDING: "#gray",
                TaskStatus.RUNNING: "#yellow",
                TaskStatus.COMPLETED: "#green",
                TaskStatus.FAILED: "#red",
                TaskStatus.BLOCKED: "#orange",
            }.get(node.status, "#gray")

            lines.append(f'    {task_id}["{node.name}"]:::{"status"}')
            lines.append(f'    class {task_id} {{"status"}}\n')

        # Add dependency edges
        for task_id, node in self._nodes.items():
            for dep in node.deps:
                lines.append(f"    {dep} --> {task_id}")

        lines.append("    classDef pending fill:#gray")
        lines.append("    classDef running fill:#yellow")
        lines.append("    classDef completed fill:#green")
        lines.append("    classDef failed fill:#red")
        lines.append("    classDef blocked fill:#orange")

        return "\n".join(lines)
