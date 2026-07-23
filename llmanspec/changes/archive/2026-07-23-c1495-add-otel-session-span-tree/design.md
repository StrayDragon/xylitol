# Design: c1495-add-otel-session-span-tree

## 问题

今日低频 span 一律 `Span::root(name, SpanContext::random())` → 每条独立 `trace_id`。Session 属性只能横向聚合，Langfuse 看不到父子树。`obs.rs` 已注明：`LocalParentGuard` 是 `!Send`，不能跨 ReAct `async_stream` 的 `.await` 依赖 local parent。

## 决策

### 1. 显式 parent，而非跨 await 的 local parent

| 机制 | 用法 |
|---|---|
| 根 | `Span::root("agent.turn", SpanContext::random())`，生命周期覆盖整次用户触发处理 |
| 子 | `Span::enter_with_parent(name, &parent_span)`，或 `Span::root(name, SpanContext::from_span(&parent).unwrap())` 在需跨任务边界时 |
| 禁止 | 在活跃 turn 内对主路径再 `SpanContext::random()` 开无关 root |
| await | **持有**父 `Span`（或可 `Send` 的 `SpanContext` 拷贝）；**不**跨 `.await` 持有 `LocalParentGuard`。`fastrace-futures::in_span` 仅在同步 `poll` 窗口设 local parent，父 context 须事先绑到正确 parent |

### 2. 树形与所有权

```text
driver / 面：一次用户消息 → 创建并持有 AgentTurnSpan (agent.turn)
  └─ ReAct 每步 → AgentIterationSpan (agent.iteration)  parent=turn
       ├─ LlmRequestSpan (llm.request)  parent=iteration   ← bridge 接受 Optional<&Span> 或 SpanContext
       └─ ToolExecuteSpan (tool.execute) parent=iteration
  └─ token.estimate  parent=turn（若估计发生在 turn 内）
```

- **根落点**：组合根 / `XyDriver` 用户 turn 边界创建 `agent.turn`（产品语义），不是「包一层 chunk stream」。
- **退役 `react.stream`**：流生命周期并入 `llm.request`（或作为其子，但不作为独立导出名/根）。
- **bridge**：`ProviderRequestTrace::start` 增加可选 parent；agent ↛ infra 不变，parent 经调用参数或 bridge 侧 turn context（与现有 `obs_session` 同层进程态，慎用；优先参数传递以免隐式全局）。

### 3. 导出名（产品词汇）

| 旧 | 新 | observation.type |
|---|---|---|
| （无统一根） | `agent.turn` | agent 或 span（根建议 agent） |
| `react.turn` | `agent.iteration` | agent |
| `provider.request` | `llm.request` | generation |
| `tool.execute` | `tool.execute` | tool |
| `react.stream` | 退役 | — |
| `token.estimate` | `token.estimate` | span |

不保留长期双名兼容；本地 JSONL / 窄读脚本随实现改名。

### 4. `token.estimate`

- 有活跃 turn parent → `enter_with_parent` / 同 trace child
- 无 turn → 独立 root + `langfuse.session.id`（若已知）
- 禁止伪造「挂到上一轮 turn」

### 5. 验收 seam

1. **单测 / 收集 Reporter**：一轮假对话后，`SpanRecord` 中 iteration / llm / tool 的 `trace_id` 等于 turn，且 `parent_id` 链正确。
2. **命名断言**：导出/记录中的 span name 为新产品名，无 `react.stream`/`react.turn`/`provider.request`。
3. **手工 / CLI（apply 后）**：Langfuse 同 Session 下 traces 数量 ≈ 用户轮次；打开一条可见树。不作为默认 BDD 硬闸（远端依赖），tasks 中记验收步骤。

## 非目标

- Session 级单根超长树
- tracing 双栈
- 改变 otel1 默认不出口
