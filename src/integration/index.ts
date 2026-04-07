/**
 * Integration Layer - TypeScript wrappers for Python/Rust modules
 *
 * Exports:
 * - TaskDAGWrapper: Task dependency management
 * - KnowledgeGraphWrapper: Persistent memory
 * - ReportBuilderWrapper: Report generation
 * - PythonBridge: Python subprocess communication
 * - Adapters: Existing codebase integration
 */

export { pythonCall, PythonSubprocess } from './pythonBridge'
export {
  TaskDAGWrapper,
  executeDAG,
  type TaskStatus,
  type TaskNode,
  type ExecutionResult,
  type TaskDAGVisualization,
} from './taskDagWrapper'

export {
  KnowledgeGraphWrapper,
  createMemoryEntry,
  type MemoryType,
  type MemoryEntry,
  type SearchResult,
  type KnowledgeGraphStats,
} from './knowledgeGraphWrapper'

export {
  ReportBuilderWrapper,
  type SectionLevel,
  type Finding,
  type Diagram,
  type ReportSection,
  type ExplorationMetadata,
  type ReportJSON,
  type ReportStats,
} from './reportBuilderWrapper'

export {
  TaskAdapter,
  SessionMemoryAdapter,
  VerificationReportAdapter,
  ExecutionContext,
} from './adapters'
