/**
 * Integration Adapters - Connect new Python/Rust modules to existing TypeScript codebase
 *
 * These adapters provide a bridge between the original TypeScript task system
 * and the refactored Python/Rust modules.
 */

import type { TaskStateBase, TaskType, TaskStatus } from '../Task'
import type { AppState } from '../state/AppState'

// Re-export wrappers for use in existing codebase
export {
  TaskDAGWrapper,
  executeDAG,
  KnowledgeGraphWrapper,
  ReportBuilderWrapper,
  pythonCall,
  PythonSubprocess,
} from '../integration'

export type {
  TaskStatus as DagTaskStatus,
  TaskNode,
  ExecutionResult,
  MemoryType,
  MemoryEntry,
  SearchResult,
  Finding,
  Diagram,
} from '../integration'

/**
 * Adapter to use TaskDAGWrapper with existing Task types
 */
export class TaskAdapter {
  private dag = new (require('../integration').TaskDAGWrapper)()

  addTask(task: Pick<TaskStateBase, 'id' | 'description'> & { deps?: string[] }): void {
    this.dag.addTask(task.id, task.description, task.deps || [])
  }

  toTaskStateBase(taskId: string, type: TaskType, outputFile: string): Partial<TaskStateBase> {
    const node = this.dag.getTask(taskId)
    return {
      id: taskId,
      type,
      status: this.mapStatus(node?.status || 'PENDING'),
      description: node?.name || '',
      outputFile,
      outputOffset: 0,
      notified: false,
      startTime: Date.now(),
    }
  }

  private mapStatus(status: string): TaskStatus {
    const mapping: Record<string, TaskStatus> = {
      PENDING: 'pending',
      RUNNING: 'running',
      COMPLETED: 'completed',
      FAILED: 'failed',
    }
    return mapping[status] || 'pending'
  }
}

/**
 * Session memory adapter - wraps KnowledgeGraph for session-scoped memory
 */
export class SessionMemoryAdapter {
  private kg = new (require('../integration').KnowledgeGraphWrapper)()
  private sessionId: string

  constructor(sessionId: string) {
    this.sessionId = sessionId
  }

  recordUserPreference(key: string, value: string, importance = 0.7): void {
    this.kg.add(
      `pref_${key}`,
      value,
      'user',
      { importance, tags: ['preference', this.sessionId] }
    )
  }

  recordFeedback(type: 'correction' | 'confirmation', content: string): void {
    this.kg.add(
      `feedback_${Date.now()}`,
      content,
      'feedback',
      { importance: type === 'correction' ? 0.9 : 0.6, tags: [type, this.sessionId] }
    )
  }

  recordProjectContext(projectId: string, context: string): void {
    this.kg.add(
      `project_${projectId}`,
      context,
      'project',
      { importance: 0.8, tags: ['project', projectId] }
    )
  }

  searchContext(query: string, limit = 5) {
    return this.kg.search(query, { limit })
  }
}

/**
 * Verification report adapter - generates reports from task results
 */
export class VerificationReportAdapter {
  private rb = new (require('../integration').ReportBuilderWrapper)(
    'Verification Report',
    'automated-verification'
  )

  addCheck(name: string, passed: boolean, output: string, error?: string): void {
    this.rb.addFinding(
      name,
      passed ? 'Check passed' : `Check failed: ${error || 'unknown'}`,
      {
        evidence: output ? [output.substring(0, 500)] : [],
        severity: passed ? 'info' : 'critical',
      }
    )
  }

  setFilesAnalyzed(count: number): void {
    this.rb.setMetadata(count)
  }

  buildReport(): { markdown: string; json: unknown } {
    this.rb.setCompleted()
    return {
      markdown: this.rb.buildMarkdown(),
      json: this.rb.buildJSON(),
    }
  }
}

/**
 * Task execution context with integrated memory and reporting
 */
export class ExecutionContext {
  dag: InstanceType<typeof require('../integration').TaskDAGWrapper>
  memory: SessionMemoryAdapter
  report: VerificationReportAdapter

  constructor(sessionId: string) {
    const { TaskDAGWrapper, KnowledgeGraphWrapper, ReportBuilderWrapper } = require('../integration')

    this.dag = new TaskDAGWrapper()
    this.memory = new SessionMemoryAdapter(sessionId)
    this.report = new VerificationReportAdapter()
  }

  /**
   * Record a task completion in memory and report
   */
  recordTaskCompletion(taskId: string, result: unknown): void {
    this.memory.recordProjectContext(taskId, `Task completed: ${JSON.stringify(result).substring(0, 200)}`)
    this.report.addCheck(`Task ${taskId}`, true, String(result))
  }

  /**
   * Record a task failure
   */
  recordTaskFailure(taskId: string, error: string): void {
    this.report.addCheck(`Task ${taskId}`, false, '', error)
  }
}
