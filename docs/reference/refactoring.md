---
layout: home

hero:
  name: Harness Engineering
  text: 重构模块文档
  tagline: 任务编排、记忆系统和报告生成的解耦架构

features:
  - icon: "📊"
    title: TaskDAG
    details: 有向无环图实现，支持拓扑排序和并行执行
    link: /reference/refactoring#taskdag
  - icon: "🧠"
    title: KnowledgeGraph
    details: 持久化知识图谱，支持加权搜索和关系追踪
    link: /reference/refactoring#knowledgegraph
  - icon: "📝"
    title: ReportBuilder
    details: 结构化报告生成，支持 Mermaid 图表
    link: /reference/refactoring#reportbuilder
  - icon: "🎭"
    title: Orchestrator
    details: 整合 DAG、Memory 和 Report 的高级编排器
    link: /reference/refactoring#orchestrator
  - icon: "🔧"
    title: Rust Modules
    details: tool-executor、sandbox、vcr、verification
    link: /reference/refactoring#rust-modules
  - icon: "🔗"
    title: TypeScript Integration
    details: 现有代码库的 TypeScript 集成适配器
    link: /reference/refactoring#integration
---

# Harness Engineering 模块

本目录包含一组经过重构的模块，实现了**任务编排、记忆系统和报告生成**的解耦架构。

## 目录结构

```
src-python/                    # Python 编排层
├── coordinator/               # 任务 DAG 和编排器
│   ├── dag.py               # TaskDAG - 有向无环图调度
│   ├── orchestrator.py      # Orchestrator - 高级工作流编排
│   └── visualize.py         # DAGVisualizer - 可视化导出
├── memory/                   # 记忆系统
│   └── knowledge_graph.py   # KnowledgeGraph - 持久化知识图谱
└── exploration/              # 报告生成
    └── report_builder.py     # ReportBuilder - 结构化报告

src-rust/                     # Rust 执行层
├── tool-executor/           # 进程执行（内存/超时限制）
├── sandbox/                 # seccomp 系统调用过滤
├── vcr/                     # API 响应录制/回放
└── verification/             # 对抗性验证框架

src/integration/              # TypeScript 集成层
├── taskDagWrapper.ts       # TaskDAG TypeScript 包装器
├── knowledgeGraphWrapper.ts # KnowledgeGraph TypeScript 包装器
├── reportBuilderWrapper.ts  # ReportBuilder TypeScript 包装器
└── adapters.ts             # 现有代码库适配器
```

## TaskDAG {#taskdag}

有向无环图实现，支持拓扑排序和并行执行。

### 核心功能

- **拓扑排序**：Kahn 算法确定执行顺序
- **并行执行**：同层任务并行运行，受 max_parallel 控制
- **依赖验证**：自动检测循环依赖
- **可视化**：Mermaid、Graphviz、JSON 多格式导出

### API

```python
class TaskDAG:
    def add_task(self, task_id: str, name: str, deps: list[str] = None, metadata: dict = None) -> TaskDAG
    def get_execution_order(self) -> list[list[str]]  # [[level1], [level2_tasks], ...]
    def execute(self, executor: Callable, max_parallel: int = 4) -> dict[str, ExecutionResult]
    def visualize(self) -> str  # Mermaid 格式
```

### 示例

```python
from coordinator import TaskDAG, TaskStatus

dag = TaskDAG()
dag.add_task('analyze', '分析需求', metadata={'phase': 'research'})
dag.add_task('implement', '实现代码', deps=['analyze'], metadata={'phase': 'impl'})
dag.add_task('test', '测试', deps=['implement'], metadata={'phase': 'test'})

# 执行顺序: [['analyze'], ['implement'], ['test']]
print(dag.get_execution_order())

# 并行执行
async def executor(task_id):
    await asyncio.sleep(0.1)
    return f"done-{task_id}"

results = await dag.execute(executor, max_parallel=2)
```

## KnowledgeGraph {#knowledgegraph}

持久化知识图谱，支持加权搜索和关系追踪。

### 记忆类型

| 类型 | 说明 |
|------|------|
| `USER` | 用户偏好、角色反馈 |
| `PROJECT` | 项目特定上下文 |
| `REFERENCE` | 外部系统指针 |
| `FEEDBACK` | 指导修正 |
| `FACT` | 事实知识 |

### 核心功能

- **加权搜索**：name(3x) > description(2x) > content(1x) + importance boost
- **类型过滤**：按记忆类型筛选
- **关系追踪**：双向关联，支持 BFS 遍历
- **自动清理**：超过 10000 条时驱逐低重要性条目
- **持久化**：JSON 格式存储

### API

```python
class KnowledgeGraph:
    def add(self, name: str, content: str, memory_type: MemoryType,
            description: str = "", importance: float = 0.5, tags: list = None) -> MemoryEntry
    def search(self, query: str, memory_types: list = None,
               tags: list = None, limit: int = 10) -> list[SearchResult]
    def add_relationship(self, entry_id: str, related_id: str) -> bool
    def get_related(self, entry_id: str, depth: int = 1) -> list[MemoryEntry]
    def save(self) -> None
```

### 示例

```python
from memory import KnowledgeGraph, MemoryType

kg = KnowledgeGraph()
kg.add('user_role', 'Senior engineer', MemoryType.USER,
       importance=0.9, tags=['role', 'engineering'])
kg.add('feedback_testing', 'Use real DB', MemoryType.FEEDBACK,
       importance=0.85, tags=['testing', 'db'])

# 搜索
results = kg.search('engineer', memory_types=[MemoryType.USER])

# 关系
e1 = kg.search('role').pop().entry
e2 = kg.search('testing').pop().entry
kg.add_relationship(e1.id, e2.id)
related = kg.get_related(e1.id)
```

## ReportBuilder {#reportbuilder}

结构化报告生成，支持 Mermaid 图表。

### 核心功能

- **层级结构**：Chapter > Section > Subsection
- **发现追踪**：title、description、evidence、severity、code_refs
- **图表支持**：架构图、流程图、时序图
- **多格式导出**：Markdown、JSON

### API

```python
class ReportBuilder:
    def add_chapter(self, title: str, content: str = "") -> self
    def add_section(self, title: str, content: str = "") -> self
    def add_subsection(self, title: str, content: str = "") -> self
    def add_finding(self, title: str, description: str,
                    evidence: list = None, severity: str = "info",
                    code_refs: list = None) -> self
    def add_architecture_diagram(self, title: str,
                                  components: list, relationships: list) -> self
    def add_flow_diagram(self, title: str, steps: list) -> self
    def build_markdown(self) -> str
    def build_json(self) -> dict
```

### 示例

```python
from exploration import ReportBuilder

rb = ReportBuilder('Verification Report', scope='e2e-test')
rb.add_chapter('Executive Summary')
rb.add_section('Overview', 'All tests passed.')

rb.add_finding(
    'Critical Issue',
    'Found a bug',
    evidence=['log error #1234'],
    severity='critical',
    code_refs=['src/main.rs:42']
)

rb.add_architecture_diagram(
    'System Architecture',
    [
        {'id': 'api', 'label': 'API Gateway', 'type': 'service'},
        {'id': 'db', 'label': 'Database', 'type': 'database'},
    ],
    [('api', 'db', 'queries')]
)

rb.set_completed()
print(rb.build_markdown())
```

## Orchestrator {#orchestrator}

整合 DAG、Memory 和 Report 的高级编排器。

### 核心功能

- **一键编排**：DAG + Memory + Report 自动整合
- **上下文存储**：自动记录任务执行到记忆系统
- **报告生成**：执行完成后自动生成完整报告
- **配置灵活**：可单独启用/禁用 Memory 或 Report

### API

```python
class Orchestrator:
    def add_task(self, task_id: str, name: str, deps: list = None, metadata: dict = None) -> self
    def add_tasks_batch(self, tasks: list[dict]) -> self
    async def execute(self, executor_fn: Callable) -> dict
    def generate_report(self) -> str
    def get_context(self, query: str, limit: int = 5) -> list
    def store_context(self, name: str, content: str, memory_type: MemoryType) -> None
```

### 示例

```python
from coordinator import create_orchestrator

orch = create_orchestrator("Build Workflow", max_parallel=3)
orch.add_tasks_batch([
    {'id': 'setup', 'name': 'Setup'},
    {'id': 'build', 'name': 'Build', 'deps': ['setup']},
    {'id': 'test', 'name': 'Test', 'deps': ['build']},
])

async def executor(task_id):
    await asyncio.sleep(0.1)
    return f"done-{task_id}"

result = await orch.execute(executor)
print(orch.generate_report())
```

## Rust Modules {#rust-modules}

### tool-executor

安全进程执行，支持内存/超时限制：

```rust
use tool_executor::{ToolExecutor, ToolExecutorConfig};

let config = ToolExecutorConfig::default();
let executor = ToolExecutor::new(config);
let result = executor.execute("echo", &["hello"], None).await;
```

### sandbox

seccomp 系统调用过滤（Linux）：

```rust
use sandbox::{Sandbox, SandboxConfig};

let config = SandboxConfig::strict();
let sandbox = Sandbox::new(config)?;
```

### vcr

API 响应录制/回放：

```rust
use vcr::{Vcr, VcrConfig, VcrMode};

let mut vcr = Vcr::new(config, VcrMode::Auto)?;
let response = vcr.get_or_record(&request, |req| network_call(req))?;
```

### verification

对抗性验证框架：

```rust
use verification::{run_check, Check, CheckKind};

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

## TypeScript Integration {#integration}

### 集成适配器

```typescript
import { ExecutionContext } from './integration/adapters'

const ctx = new ExecutionContext('session-123')

// 添加任务
ctx.dag.addTask('step1', 'Do something')
ctx.dag.addTask('step2', 'Next step', deps=['step1'])

// 执行
const results = await executeDAG(ctx.dag, async (taskId) => {
    const result = await doWork(taskId)
    ctx.recordTaskCompletion(taskId, result)
    return result
})

// 生成报告
const { markdown, json } = ctx.report.buildReport()
```

### Python Bridge

Python 子进程通信：

```typescript
import { pythonCall } from './integration/pythonBridge'

const result = await pythonCall('coordinator', 'TaskDAG')
```

## 工作流架构图

```
┌─────────────────────────────────────────────────────────────┐
│                    Orchestrator (编排器)                       │
│  整合 TaskDAG + KnowledgeGraph + ReportBuilder              │
└─────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│    TaskDAG      │  │ KnowledgeGraph  │  │  ReportBuilder  │
│  (任务调度)      │  │   (记忆系统)     │  │   (报告生成)     │
└─────────────────┘  └─────────────────┘  └─────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│ Rust Executor    │  │   JSON 文件      │  │   Mermaid       │
│ (安全执行)       │  │   (持久化)       │  │   (图表)         │
└─────────────────┘  └─────────────────┘  └─────────────────┘
```
