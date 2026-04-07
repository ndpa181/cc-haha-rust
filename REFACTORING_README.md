# Refactored Modules Architecture

## Overview

This document describes the refactored architecture separating concerns into Python (orchestration), Rust (execution/safety), and TypeScript (integration) layers.

## Module Structure

```
src-python/           # Python orchestration layer
├── coordinator/      # Task DAG and orchestration
├── memory/           # Knowledge graph with persistence
└── exploration/      # Report generation with diagrams

src-rust/             # Rust execution layer
├── tool-executor/    # Safe subprocess execution
├── sandbox/          # Seccomp-based security
├── vcr/              # API response caching
└── verification/      # Adversarial verification

src/integration/       # TypeScript integration layer
├── pythonBridge.ts    # Python subprocess communication
├── taskDagWrapper.ts  # TaskDAG TypeScript bindings
├── knowledgeGraphWrapper.ts
├── reportBuilderWrapper.ts
└── adapters.ts        # Existing codebase adapters
```

## Python Modules

### coordinator/dag.py - TaskDAG

Directed Acyclic Graph for task dependency management.

```python
from coordinator import TaskDAG, TaskStatus

dag = TaskDAG()
dag.add_task('a', 'Task A')
dag.add_task('b', 'Task B', deps=['a'])

levels = dag.get_execution_order()  # [['a'], ['b']]
```

**Features:**
- Topological sort for execution ordering
- Parallel execution with configurable concurrency
- Cycle detection
- Mermaid/Graphviz/JSON visualization

### coordinator/orchestrator.py - Orchestrator

High-level workflow orchestration combining DAG, Memory, and Reporting.

```python
from coordinator import create_orchestrator

orch = create_orchestrator("My Workflow", max_parallel=4)
orch.add_tasks_batch([
    {"id": "step1", "name": "Initialize"},
    {"id": "step2", "name": "Process", "deps": ["step1"]},
])

async def executor(task_id):
    return f"done-{task_id}"

result = await orch.execute(executor)
```

### memory/knowledge_graph.py - KnowledgeGraph

Persistent memory with typed entries and relationships.

```python
from memory import KnowledgeGraph, MemoryType

kg = KnowledgeGraph()
kg.add('user_role', 'Senior engineer', MemoryType.USER,
       importance=0.9, tags=['role'])

results = kg.search('engineer', memory_types=[MemoryType.USER])
```

**Memory Types:**
- `user` - User preferences and roles
- `project` - Project-specific context
- `reference` - External system pointers
- `feedback` - Guidance and corrections
- `fact` - Factual knowledge

### exploration/report_builder.py - ReportBuilder

Report generation with findings and Mermaid diagrams.

```python
from exploration import ReportBuilder

rb = ReportBuilder('My Report', scope='analysis')
rb.add_chapter('Overview')
rb.add_finding('Issue found', 'Details here', severity='warning')

rb.add_architecture_diagram('System',
    [{'id': 'a', 'label': 'A', 'type': 'service'}],
    [('a', 'b', 'calls')]
)

rb.set_completed()
print(rb.build_markdown())
```

## Rust Modules

### tool-executor/

Safe subprocess execution with resource limits.

```rust
use tool_executor::{ToolExecutor, ToolExecutorConfig};

let config = ToolExecutorConfig::default();
let executor = ToolExecutor::new(config);

let result = executor.execute("echo", &["hello"], None).await;
```

**Features:**
- Memory and time limits
- Streaming output
- Environment variable filtering

### sandbox/

Security boundaries using seccomp on Linux.

```rust
use sandbox::{Sandbox, SandboxConfig};

let config = SandboxConfig::strict();
let sandbox = Sandbox::new(config)?;
```

**Features:**
- Syscall allow/deny lists
- Filesystem access restrictions
- Resource limits

### vcr/

Video Cassette Recorder for API response caching.

```rust
use vcr::{Vcr, VcrConfig, VcrMode};

let config = VcrConfig::default();
let mut vcr = Vcr::new(config, VcrMode::Auto)?;

let response = vcr.get_or_record(&request, |req| network_call(req))?;
```

**Features:**
- SHA-1 based fixture naming
- Cross-platform path normalization
- Memory-mapped cache

### verification/

Adversarial verification framework.

```rust
use verification::{run_check, Check, CheckKind, VerificationReport};

let check = Check {
    name: "build".to_string(),
    check_kind: CheckKind::Command {
        program: "cargo".to_string(),
        args: vec!["build".to_string()],
        expected_pattern: Some("Compiling".to_string()),
        must_have_command: true,
    },
};

let result = run_check(&check);
```

## TypeScript Integration

### taskDagWrapper.ts

TypeScript wrapper for Python TaskDAG with full type safety.

```typescript
import { TaskDAGWrapper, executeDAG } from './integration'

const dag = new TaskDAGWrapper()
dag.addTask('task1', 'Do something')
dag.addTask('task2', 'Do next', deps=['task1'])

const viz = dag.visualize()
// viz.mermaid, viz.graphviz, viz.json, viz.markdownTable
```

### knowledgeGraphWrapper.ts

TypeScript wrapper for Python KnowledgeGraph.

```typescript
import { KnowledgeGraphWrapper } from './integration'

const kg = new KnowledgeGraphWrapper()
kg.add('my_memory', 'Memory content', 'project', { importance: 0.8 })

const results = kg.search('memory')
```

### reportBuilderWrapper.ts

TypeScript wrapper for Python ReportBuilder.

```typescript
import { ReportBuilderWrapper } from './integration'

const rb = new ReportBuilderWrapper('Test Report')
rb.addChapter('Overview')
rb.addFinding('Issue', 'Details', { severity: 'warning' })
rb.setCompleted()

const markdown = rb.buildMarkdown()
```

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    TypeScript Layer                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ TaskAdapter │  │MemoryAdapter│  │VerificationAdapter │  │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
└─────────┼────────────────┼────────────────────┼─────────────┘
          │                │                    │
          ▼                ▼                    ▼
┌─────────────────────────────────────────────────────────────┐
│              Integration Layer (TypeScript)                  │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  pythonBridge.ts  │  taskDagWrapper.ts               │   │
│  │  knowledgeGraphWrapper.ts  │  reportBuilderWrapper.ts │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
          │                │                    │
          ▼                ▼                    ▼
┌─────────────────────────────────────────────────────────────┐
│                 Python Layer (Orchestration)                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │  TaskDAG    │  │KnowledgeGraph│  │   ReportBuilder     │ │
│  │  Orchestrator│  │             │  │                     │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
          │                                    ▲
          ▼                                    │
┌─────────────────────┐                        │
│   Rust Layer        │                        │
│ ┌───────┐ ┌───────┐ │  ┌─────────────┐       │
│ │ Tool  │ │Sandbox│ │  │ Verification│       │
│ │ Exec  │ │       │ │  │             │       │
│ └───────┘ └───────┘ │  └─────────────┘       │
│ ┌───────┐ ┌───────┐ │                        │
│ │  VCR  │ │       │ │                        │
│ └───────┘ └───────┘ │                        │
└─────────────────────┘ ────────────────────────┘
```

## Usage Example

```typescript
import { ExecutionContext } from './integration/adapters'

async function runWorkflow() {
  const ctx = new ExecutionContext('session-123')

  // Define tasks
  ctx.dag.addTask('research', 'Research architecture')
  ctx.dag.addTask('implement', 'Implement code', deps=['research'])
  ctx.dag.addTask('test', 'Run tests', deps=['implement'])

  // Execute with integrated memory and reporting
  const { executeDAG } = await import('./integration')

  const results = await executeDAG(ctx.dag, async (taskId) => {
    // Execute task
    const result = await doWork(taskId)
    ctx.recordTaskCompletion(taskId, result)
    return result
  })

  // Generate report
  const { markdown, json } = ctx.report.buildReport()
  console.log(markdown)

  return results
}
```

## Testing

```bash
# Python modules
PYTHONPATH=/tmp/cc-haha/src-python python3 -c "
from coordinator import TaskDAG
from memory import KnowledgeGraph
from exploration import ReportBuilder
print('All Python modules imported successfully')
"

# Rust modules (requires cargo)
cd /tmp/cc-haha/src-rust && cargo check
```

## Dependencies

### Python
- Python 3.11+

### Rust
- Rust 1.70+
- tokio (async runtime)
- serde (serialization)
- memmap2 (memory-mapped files)
- sha1 (hashing)

### TypeScript
- TypeScript 5.0+
- Node.js 18+ (for subprocess execution)
