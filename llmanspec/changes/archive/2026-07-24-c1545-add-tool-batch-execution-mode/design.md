# Design: c1545-add-tool-batch-execution-mode

## 问题与约束

- ReAct 丢弃 `tool_mode`，固定串行；`ar21` 仅 MessageEnd 后执行。
- 异于 pi：写屏障窗 + 开箱 sequential + **MCP 硬串行**。

## 已关闭决策（D8）

见 `proposal.md` D1–D8。摘要：

- `BatchMode` 默认 `Sequential`；试验档 `BarrierParallel`
- 拆 `BatchMode` / `ToolConcurrency`
- **`mcp:` 前缀永远 Barrier**——无配置、无 glob、无 annotation 放行
- bash/write/edit Barrier；读族 ParallelSafe；保留 FileMutationQueue
- 观测：属性 only；S1–S5 MUST，S6 SHOULD

## 工具盘点

### 内置（`default_tools`）

| name | 拟 `ToolConcurrency` | 备注 |
|---|---|---|
| read / grep / find / ls | ParallelSafe | |
| write / edit | Barrier | + FileMutationQueue |
| bash | Barrier | 含流式 Update |

### 动态

| 入口 | 规则 |
|---|---|
| `McpToolAdapter`（`mcp:{server}:{tool}`） | **硬 Barrier**；重载后仍硬 Barrier |
| 自注册 `XyTool` | 默认 Barrier；可显式标 ParallelSafe（**不得**用于伪装 MCP） |
| 未知名 | Barrier 位序（错误结果） |

`patch.rs` / `process.rs`：非注册工具。

## 架构

```text
                    AppConfig.tool_batch.mode
                              │
                    composition / Session
                              │
                    BatchMode (turn snapshot)
                              ▼
┌─────────────────────────────────────────────────────────┐
│ agent/runtime/tool_batch.rs                             │
│  classify(name, tool) -> ToolConcurrency                │
│    if name.starts_with("mcp:") -> Barrier  // 硬规则    │
│    else tool.concurrency()                               │
│  plan_windows(calls) -> [ParallelWindow | BarrierOne]   │
│  (纯数据；单测无 I/O)                                    │
└──────────────────────────┬──────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────┐
│ agent/runtime/tool_exec.rs（从 react 抽出）               │
│  run_one / flush_parallel_window                         │
│  preflight → exec(+Update mux) → after → persist         │
│  显式 Span parent；cancel 共享                           │
└──────────────────────────┬──────────────────────────────┘
                           │
                      react.rs 剧本
```

### 分类算法（normative）

```text
classify(name, tool_opt) -> ToolConcurrency:
  if name.starts_with("mcp:") -> Barrier          # 不可覆盖
  match tool_opt:
    Some(t) -> t.concurrency()                    # ParallelSafe | Barrier
    None -> Barrier
```

**禁止**：配置名单、glob、MCP schema annotation 改变上述结果。

### 调度算法

```text
run_batch(calls, BatchMode::Sequential):
  for c in calls: await run_one(c)

run_batch(calls, BatchMode::BarrierParallel):
  window = []
  barrier_index = 0
  for c in calls:
    if classify(c) == ParallelSafe:
      window.push(c)
    else:
      await flush_parallel_window(window, barrier_index); window.clear()
      barrier_index += 1
      await run_one(c)
      barrier_index += 1
  await flush_parallel_window(window, barrier_index)
```

### 类型迁移

| 今日 | 目标 |
|---|---|
| `XyToolExecutionMode::{Parallel,Sequential}` | 拆为 `BatchMode` + `ToolConcurrency`；或保留旧名作 `ToolConcurrency` 别名并改文档/Default |
| Session `tool_mode` 默认 Sequential | 映射 `BatchMode::Sequential` |
| trait `execution_mode()` | → `concurrency()`（可暂留旧名转发） |

`XyToolExecutionMode` 旧注释「Sequential = 整批塌缩」**删除**（那是 pi 语义）。

### 配置

```yaml
tool_batch:
  mode: sequential   # 或 barrier_parallel
```

- 仅 `mode`；**无** `parallel_safe` / `parallel_safe_patterns`。
- 非法 mode → 加载失败；缺省 = sequential。

### 模块落点

| 职责 | 路径 |
|---|---|
| 切窗 / classify | `agent/runtime/tool_batch.rs` |
| 单工具执行协程 | `agent/runtime/tool_exec.rs`（新；瘦 react） |
| 配置 | `infra/config/types.rs` → 组合根 |
| MCP Barrier | `McpToolAdapter::concurrency` 固定 Barrier + classify 硬前缀 |
| 内置类 | write/bash/edit Barrier；读族 Safe |
| OTEL | `ToolExecuteSpan` 强制 parent；属性 |

## 观测

```text
agent.turn → agent.iteration → [ llm.request | tool.execute* ]
```

并行窗内 tool span 时间可重叠；同 parent；属性含 `tool_batch.mode`、`tool_batch.barrier_index`、`tool_id`。

## Harness 设计（S1–S6）

### 原则

- BDD 走既有 `AgentRuntime` / mock 模型词表；并行用**可控假工具**（`sleep_ms`），不测真实磁盘竞态。
- 调度纯函数与 I/O 执行分离：S2 不启 runtime。
- MCP：单测断言 classify / adapter；**不**测「放行成功」（无此路径）。

### Fake 工具（测试专用，可放 `tests` / runtime 测模块）

| 名 | concurrency | 行为 |
|---|---|---|
| `slow_safe` | ParallelSafe | sleep N ms，记录 start/end 墙钟 |
| `slow_barrier` | Barrier | sleep N ms |
| `mcp:fake:x` | （名触发硬 Barrier） | 可与真 adapter 单测分立 |

### 场景矩阵

| ID | Seam | Given | Then | 层 |
|---|---|---|---|---|
| S1a | BDD `@req:ar27` | 未配 mode；两 `slow_safe` | 无时间重叠；源序 End | feature |
| S1b | BDD `@req:ar28` overlap | `barrier_parallel`；两 safe + 一 barrier | 两 safe 重叠；均在 barrier start 前结束 | feature |
| S1c | BDD `@req:ar28` windows | safe → barrier → safe | 第二 safe 不与第一同窗 | feature |
| S1d | BDD `@req:ar29` | 并行窗后发先完成 | history toolResult **源序** | feature |
| S2 | 单测 `tool_batch` | 名序列含 `mcp:…` | plan 中 mcp 均为 BarrierOne；切窗表断言 | unit |
| S3 | 单测/集成墙钟 | 两 slow_safe @100ms | 总耗时 ≪ 200ms（阈值） | unit |
| S4 | 配置单测 | 缺省 / 非法 mode | sequential / Err | unit |
| S5 | MCP 单测 | adapter + `classify("mcp:…")` | 恒 Barrier；改 trait 返回 Safe **仍** Barrier | unit |
| S6 | obs SHOULD | 并行两 tool + trace 闸 | 同 iteration parent；非 random root | unit/obs |

### BDD step 词表意向（apply 接线）

- `假如 工具批模式为 barrier_parallel`
- `假如 mock 模型同 turn 发出工具序 …`（或复用现有多 tool mock + 注册假工具）
- `那么 工具 {id} 与 {id} 执行时间重叠`
- `那么 工具 {id} 在工具 {id} 开始前已结束`
- `那么 history toolResult 顺序为 …`

具体措辞以既有 `tests/bdd.rs` 风格为准；优先扩展而非平行第二套 harness。

## pi 对照与体感（调研结论）

- 默认 `toolExecution = "parallel"`：预检（Start + prepare）**串行**，execute 才 `Promise.all`。
- 同批任一工具 `executionMode: "sequential"` → **整批塌缩串行**。
- 内置 read/bash/edit/write/… **未**标 sequential → 机制上可并发。
- 体感常像串行：模型一 turn 单 tool、Start 顺序冒烟、快工具看不出重叠、同 path `FileMutationQueue`、扩展工具标 sequential。

## Bang（`!` / `!!`）

- **不在**本调度器范围：TUI host → `XyDriver::execute_bash`；`!!` 仅 `exclude_from_context`。
- **恒串行**（用户一条跑完再下一条）；MUST NOT 与 agent 工具并行窗混调度。

## Non-goals

- MCP 并行放行；pi 塌缩；流中执行；产品 UI；去 FileMutationQueue；改 bang 路径
