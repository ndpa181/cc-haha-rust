/**
 * TaskDAG Integration - TypeScript wrapper for Python TaskDAG
 *
 * Provides TaskDAG functionality to TypeScript code with full type safety.
 */

import { pythonCall, PythonSubprocess } from './pythonBridge'

export type TaskStatus = 'PENDING' | 'RUNNING' | 'COMPLETED' | 'FAILED' | 'BLOCKED'

export interface TaskNode {
  id: string
  name: string
  deps: string[]
  status: TaskStatus
  result: unknown
  error: string | null
  metadata: Record<string, unknown>
}

export interface ExecutionResult {
  task_id: string
  status: TaskStatus
  result: unknown
  error: string | null
  duration_ms: number
}

export interface TaskDAGVisualization {
  mermaid: string
  graphviz: string
  json: unknown
  markdownTable: string
}

/**
 * TypeScript TaskDAG that wraps the Python implementation
 */
export class TaskDAGWrapper {
  private tasks = new Map<string, TaskNode>()
  private adjacency = new Map<string, string[]>()
  private reverseAdjacency = new Map<string, string[]>()

  addTask(
    taskId: string,
    name: string,
    deps: string[] = [],
    metadata: Record<string, unknown> = {}
  ): void {
    if (this.tasks.has(taskId)) {
      throw new Error(`Task '${taskId}' already exists`)
    }

    // Validate dependencies exist
    for (const dep of deps) {
      if (!this.tasks.has(dep)) {
        throw new Error(`Dependency '${dep}' for task '${taskId}' not found`)
      }
    }

    this.tasks.set(taskId, {
      id: taskId,
      name,
      deps,
      status: 'PENDING',
      result: null,
      error: null,
      metadata,
    })

    this.adjacency.set(taskId, [])
    this.reverseAdjacency.set(taskId, deps)

    // Update adjacency lists
    for (const dep of deps) {
      this.adjacency.get(dep)!.push(taskId)
    }
  }

  getExecutionOrder(): string[][] {
    const inDegree = new Map<string, number>()
    for (const [tid, node] of this.tasks) {
      inDegree.set(tid, node.deps.length)
    }

    const levels: string[][] = []
    const queue: string[] = []

    // Start with tasks that have no dependencies
    for (const [tid, deg] of inDegree) {
      if (deg === 0) queue.push(tid)
    }

    while (queue.length > 0) {
      const level: string[] = []
      const nextQueue: string[] = []

      for (const taskId of queue) {
        level.push(taskId)
        for (const dependent of this.adjacency.get(taskId) || []) {
          const newDeg = (inDegree.get(dependent) || 0) - 1
          inDegree.set(dependent, newDeg)
          if (newDeg === 0) nextQueue.push(dependent)
        }
      }

      levels.push(level)
      queue.length = 0
      queue.push(...nextQueue)
    }

    // Check for cycles
    if (levels.flat().length !== this.tasks.size) {
      throw new Error('Cycle detected in task dependency graph')
    }

    return levels
  }

  isBlocked(taskId: string): boolean {
    const node = this.tasks.get(taskId)
    if (!node) return false
    return node.deps.some((dep) => {
      const depNode = this.tasks.get(dep)
      return depNode && depNode.status !== 'COMPLETED' && depNode.status !== 'FAILED'
    })
  }

  getReadyTasks(): string[] {
    return Array.from(this.tasks.entries())
      .filter(([tid, node]) => {
        if (node.status !== 'PENDING') return false
        return node.deps.every((dep) => {
          const depNode = this.tasks.get(dep)
          return depNode?.status === 'COMPLETED'
        })
      })
      .map(([tid]) => tid)
  }

  setTaskStatus(taskId: string, status: TaskStatus, result?: unknown, error?: string): void {
    const node = this.tasks.get(taskId)
    if (node) {
      node.status = status
      if (result !== undefined) node.result = result
      if (error !== undefined) node.error = error
    }
  }

  getFailedTasks(): string[] {
    return Array.from(this.tasks.values())
      .filter((n) => n.status === 'FAILED')
      .map((n) => n.id)
  }

  getCompletedTasks(): string[] {
    return Array.from(this.tasks.values())
      .filter((n) => n.status === 'COMPLETED')
      .map((n) => n.id)
  }

  getTask(taskId: string): TaskNode | undefined {
    return this.tasks.get(taskId)
  }

  visualize(): TaskDAGVisualization {
    const mermaidLines = ['flowchart TD']
    const graphvizLines = ['digraph TaskDAG {', '    rankdir=TB;', '    node [shape=box, style="rounded,filled"];']

    const statusColor: Record<TaskStatus, string> = {
      PENDING: '#E8E8E8',
      RUNNING: '#FFF3CD',
      COMPLETED: '#D4EDDA',
      FAILED: '#F8D7DA',
      BLOCKED: '#FFE5CC',
    }

    const statusSuffix: Record<TaskStatus, string> = {
      PENDING: '',
      RUNNING: '{O}',
      COMPLETED: '((',
      FAILED: '{/}',
      BLOCKED: '[>]',
    }

    for (const [tid, node] of this.tasks) {
      // Mermaid
      mermaidLines.push(`    ${tid}${statusSuffix[node.status]}["${node.name}"]`)
      // Graphviz
      graphvizLines.push(`    "${tid}" [label="${node.name}", fillcolor="${statusColor[node.status]}"]`)
    }

    // Dependencies
    for (const [tid, node] of this.tasks) {
      for (const dep of node.deps) {
        mermaidLines.push(`    ${dep} --> ${tid}`)
        graphvizLines.push(`    "${dep}" -> "${tid}"`)
      }
    }

    mermaidLines.push('```')
    graphvizLines.push('}')

    // Markdown table
    const tableLines = [
      '| Task ID | Name | Status | Dependencies |',
      '|----------|------|--------|---------------|',
    ]
    for (const [tid, node] of this.tasks) {
      const depsStr = node.deps.join(', ') || '-'
      tableLines.push(`| ${tid} | ${node.name} | ${node.status} | ${depsStr} |`)
    }

    return {
      mermaid: mermaidLines.join('\n'),
      graphviz: graphvizLines.join('\n'),
      json: {
        nodes: Array.from(this.tasks.values()),
        execution_levels: this.getExecutionOrder(),
        completed: this.getCompletedTasks(),
        failed: this.getFailedTasks(),
      },
      markdownTable: tableLines.join('\n'),
    }
  }

  get size(): number {
    return this.tasks.size
  }
}

/**
 * Simple async executor for TaskDAG
 */
export async function executeDAG<T>(
  dag: TaskDAGWrapper,
  executor: (taskId: string) => Promise<T>,
  maxParallel = 4
): Promise<Map<string, ExecutionResult>> {
  const results = new Map<string, ExecutionResult>()
  const startTime = Date.now()

  // Reset all tasks to pending
  for (const [tid, node] of dag['tasks']) {
    node.status = 'PENDING'
  }

  const executionLevels = dag.getExecutionOrder()

  for (const level of executionLevels) {
    const readyTasks = level.filter((tid) => dag.getTask(tid)?.status === 'PENDING')

    if (readyTasks.length === 0) continue

    // Execute tasks in parallel with semaphore
    const semaphore = new Semaphore(maxParallel)

    const promises = readyTasks.map(async (taskId) => {
      const taskStart = Date.now()
      dag.setTaskStatus(taskId, 'RUNNING')

      try {
        const result = await semaphore.acquire(() => executor(taskId))
        const duration = Date.now() - taskStart
        dag.setTaskStatus(taskId, 'COMPLETED', result)
        results.set(taskId, {
          task_id: taskId,
          status: 'COMPLETED',
          result,
          error: null,
          duration_ms: duration,
        })
      } catch (error) {
        const duration = Date.now() - taskStart
        const errorMsg = error instanceof Error ? error.message : String(error)
        dag.setTaskStatus(taskId, 'FAILED', undefined, errorMsg)
        results.set(taskId, {
          task_id: taskId,
          status: 'FAILED',
          result: null,
          error: errorMsg,
          duration_ms: duration,
        })
      }
    })

    await Promise.all(promises)
  }

  return results
}

class Semaphore {
  private permits: number
  private queue: Array<() => void> = []

  constructor(permits: number) {
    this.permits = permits
  }

  async acquire<T>(fn: () => Promise<T>): Promise<T> {
    if (this.permits > 0) {
      this.permits--
      try {
        return await fn()
      } finally {
        this.release()
      }
    } else {
      return new Promise((resolve) => {
        this.queue.push(async () => {
          try {
            resolve(await fn())
          } finally {
            this.release()
          }
        })
      })
    }
  }

  private release(): void {
    this.permits++
    if (this.queue.length > 0) {
      const next = this.queue.shift()!
      // Execute without awaiting to prevent stack overflow
      next()
    }
  }
}
