---
change_id: c1495-add-otel-session-span-tree
title: Langfuse / OTEL：同一 turn 父子时间线树 + 产品级 span 命名
status: in-progress
priority: 1495
depends_on:
- c1485-add-otel-usage-io
author: agent
branch: feature/c1495-add-otel-session-span-tree
base_sha: 62f3ec3222a03dc9e17f9b39b481522ff607439e
checkpointed: true
checkpoint_sha: 62f3ec3222a03dc9e17f9b39b481522ff607439e
---

# c1495-add-otel-session-span-tree

## Why

c1475–c1485 已能按 `langfuse.session.id` 聚会话，generation 也有 usage。但每条观测仍是独立 `Span::root` + `SpanContext::random()`，Langfuse Traces 列表平铺（`tool.execute` / `react.stream` / `provider.request` / `react.turn` 各成一棵），排障看不到「一轮用户请求 → agent → tools → LLM」的父子过程树。

额外问题：导出名带 `react.*` / `provider.*` 实现词，不利于未来 `xylitol-web` / `xylitol-server` 与本仓共用同一套 Langfuse 心智。

## What Changes

1. **父子树**：一次用户触发的处理导出为**一条** OTel/fastrace trace，根为 `agent.turn`；其子级共享同一 `trace_id` 并以 `parent_span_id` 挂接（禁止在活跃 turn 下再各自 random root）。
2. **产品级导出名**（跨面统一）：
   - `agent.turn` — 导出根（一次用户→agent 周期）
   - `agent.iteration` — ReAct 一步（原 `react.turn`）
   - `llm.request` — generation（原 `provider.request`）
   - `tool.execute` — 保留
   - 退役导出：`react.stream` / `react.turn` / `provider.request`（本地 JSONL 事件名随实现一并迁，不保留双名长期兼容）
3. **`token.estimate`**：有活跃 `agent.turn` 时作 child；无 turn 上下文时独立 root + 同 `langfuse.session.id`（不伪造父节点）。
4. **不变**：默认不出口；`observation_io` 档；fastrace 单栈；session id / GenAI / usage 语义沿用。

目标形态：

```text
Session (uuid)
 ├─ Trace: agent.turn
 │    ├─ agent.iteration          (type=agent)
 │    │    ├─ llm.request         (type=generation)
 │    │    └─ tool.execute        (type=tool)
 │    └─ token.estimate           (若发生在该 turn)
 ├─ Trace: agent.turn             (下一轮用户输入)
 └─ …
```

## Out of scope

- c1490 Collector/Tempo 分流
- Inspect UI
- 旁路 span 美学大扫除（error 等可随主路径改名，不单开切片）
- 把整段 Session 合成一棵无限长树（Session 仍聚合多条 Trace）

## Capabilities

- `infra-otel`（otel11 树 / otel12 命名 / otel13 estimate；同步修订 otel6–10 中的旧 span 名）
- `infra-provider-trace`（ipt4 低频 span 名与父子关联）

## Impact

- Langfuse：按 Session 打开后，每轮是一棵可读过程树，而非数百条散落根 traces
- 多面（TUI / Print / 未来 web·server）：同一套导出名与父子习惯
- 实现：显式 parent `Span` / `SpanContext` 传递（不跨 await 持有 `LocalParentGuard`）

## Ethics

- risk_level: low
- prohibited_actions: 默认同开全文 I/O；为好看树引入 tracing 双栈；用 display name 顶替 session id
- required_evidence: 同 turn 下子 span 与根共享 trace_id；导出名符合 otel12；validate 通过
- refusal_contract: 拒绝「默认上传全文 prompt」或「引入 tracing」类需求
- escalation_policy: 若 fastrace→OTLP 父子映射与 Langfuse UI 表现不一致，先用 CLI/导出记录核对再改映射
