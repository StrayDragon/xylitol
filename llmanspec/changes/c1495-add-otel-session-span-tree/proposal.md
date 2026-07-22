---
change_id: c1495-add-otel-session-span-tree
title: Langfuse / OTEL：同一 session 内父子时间线树
status: purpose-draft
priority: 1495
depends_on:
  - c1485-add-otel-usage-io
author: agent
---

# c1495-add-otel-session-span-tree

## Why

今日（c1475–c1485）已能按 **`langfuse.session.id`** 把 traces 聚到同一 Session，generation 也有 usage。但每条 `react.turn` / `provider.request` / `tool.execute` / `token.estimate` 仍是 **分散的根 span**（各自 `Span::root`），Langfuse 里看不到「一轮用户请求 → agent → tools → LLM」的**父子树**，只能靠 session 列表横着翻。

用户期望：打开一个 session，看到**汇聚后的处理过程树**（行业常见：一层 agent/trace，下面挂 generation / tool）。

## 别人怎么做（对照）

| 路径 | 做法 | 对我们的含义 |
|---|---|---|
| **Langfuse SDK** | `start_as_current_observation` / nested `startObservation`；OTel context 自动父子 | 需要**活跃 parent context**，不能每条都 `Span::root` |
| **纯 OTLP** | 同一 `trace_id` + `parent_span_id`；session 仍用 `langfuse.session.id` 资源/属性 | fastrace 需 `set_local_parent` / 显式 child，而非全是 root |
| **仅 session 聚合（现状）** | 同 sessionId、多根 traces | 排障「有没有发生」够用；「结构」不够 |

推荐目标形态（意向）：

```text
Session (uuid)
 └─ Trace: user turn / react.stream   (root)
     ├─ Span: react.turn (agent)
     │    ├─ Generation: provider.request
     │    └─ Tool: tool.execute
     └─ …下一 turn
```

`token.estimate` 等旁路：要么挂在 turn 下作 child，要么保持独立 root 但同 session（避免污染主树）。

## What Changes（仅意向；本 draft 不写代码）

1. **ReAct / provider / tool** 观测改为父子（fastrace local parent 或显式 child span），同一 turn 共享 `trace_id`
2. 约定根 span 名（如 `react.stream` 或 `agent.run`）与 observation types（沿用 c1480）
3. 文档：architecture / roadmap 补「树 vs session 列表」；验收用 Langfuse UI 截图或 CLI 查 parent
4. **不**改默认不出口；**不**默认上传全文 I/O（仍跟 `observation_io`）

## Out of scope

- c1490 Collector/Tempo 分流
- Inspect UI
- 一次改完所有旁路 span 命名美学

## Status

**purpose-draft** — TUI 性能轨优先；本 change 延后 promote。

## Ethics

- risk_level: low（设计稿）
- prohibited_actions: 默认同开全文 I/O；为「好看树」引入 tracing 双栈
