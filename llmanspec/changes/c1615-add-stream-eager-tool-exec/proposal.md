---
change_id: c1615-add-stream-eager-tool-exec
title: 流式 ToolCallEnd 后抢跑 ParallelSafe（Responses/Anthropic）
status: purpose-draft
priority: 1615
depends_on:
  - c1598-fix-bootstrap-honor-model-api
  - c1600-update-prefer-openai-responses
  - c1545-add-tool-batch-execution-mode
author: agent
---

# c1615-add-stream-eager-tool-exec

## Discussion context

见 [`../DEFERRED-responses-tool-batch-CONTEXT.md`](../DEFERRED-responses-tool-batch-CONTEXT.md)。

前提：非异步流水线回灌；仅批内/流中执行重叠。Completions End 过晚 → 方言门闩只要 Responses/Anthropic。实验 2 空隙数据决定是否 promote。

## Why

今日 `ar21`：仅 MessageEnd 后执行。批内并行（`c1545`）无法与「模型还在生成后续 token」重叠。

Anthropic / OpenAI Responses 在**单个 tool 块结束**时即可 `ToolCallEnd`；Completions 常在 `finish_reason` 才刷 End——抢跑收益接近 0。故本 change **绑定 Responses 优先路线（`c1600`）**，Completions 保持 MessageEnd 批执行。

## Product intent

| | |
|---|---|
| 目标 | ParallelSafe 在意图完整后尽早执行，与后续 stream 重叠 |
| Barrier | 仍不可与未完成的前序并行窗重叠抢跑；MCP/write/bash 纪律不变 |
| 回灌 LLM | 仍须 MessageEnd + 批汇聚后，源序 toolResult 再开下一轮 |
| 默认 | 开箱开启（若实验证明空隙≥工具耗时）；否则随 Responses 默认附带且可关 |

## Decisions（意向 · 实验 2 边界）

1. **改合约**：放宽 `ar21` / `a3`——允许 MessageEnd 前对已 `ToolCallEnd` 的 ParallelSafe 发 `ToolExecutionStart`；禁止对未 End 的意图执行。
2. **方言门闩**：`api=openai-responses` | `anthropic-messages` 可抢跑；`openai-completions` MUST 仍 MessageEnd 批跑（避免假预期）。
3. **取消/截断**：stream abort、`length`、错误 → cancel 未完成抢跑；截断 tool 不执行（对齐 pi truncated fail）。
4. **非异步流水线回灌**：不把部分 toolResult 塞进仍在飞的同一 HTTP 请求。
5. **配置**：尽量零旋钮；若需逃生用单一 `tool_batch.eager: bool`（默认 true），禁止方言×eager 矩阵配置爆炸。

## Experiment 2（promote 前必做）

在 Responses 下量：`ToolCallEnd(t0)` → `Done/MessageEnd(t1)` 空隙 vs 工具墙钟。仅当空隙经常 ≥ 工具耗时再 promote。

## What Changes（promote 后）

- ReAct 流式抢跑调度 + 汇聚
- specs 修订 `ar21` 及 BDD
- OTEL：抢跑 span 仍挂 `agent.iteration`；可选属性 `tool_batch.eager=true`

## Non-Goals

- Completions 抢跑
- 结果流式回灌同轮 LLM
- 放开 MCP 并行

## Status

**purpose-draft** — 后置；先 `c1598`/`c1600` + 实验 2 数据。

## Ethics

- risk_level: high（副作用时序、取消、合约大改）
- required_evidence: 方言门闩测试；abort 不留脏写；实验 2 空隙分布
