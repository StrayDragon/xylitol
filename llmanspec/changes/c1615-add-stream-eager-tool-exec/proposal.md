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

## Discussion context（2026-07-24）

- 今日 `ar21`：仅 MessageEnd 后执行；与 pi 同构，不是异步流水线回灌。
- Completions 常 finish 时才批量 `ToolCallEnd` → 抢跑 ROI≈0；Responses/Anthropic 可逐块 End。
- 依赖 `c1598`/`c1600`；**实验 2** 量 End→Done 空隙后再决定是否 promote。

## Why

批内并行无法与「模型还在生成后续 token」重叠；Responses 上可抢跑 ParallelSafe。

## Product intent

| | |
|---|---|
| 目标 | 意图完整后尽早执行 ParallelSafe，与后续 stream 重叠 |
| Barrier / MCP | 纪律不变 |
| 回灌 LLM | 仍 MessageEnd + 批汇聚后下一轮 |
| Completions | 仍 MessageEnd 批跑（方言门闩） |

## Decisions（意向）

1. 放宽 `ar21`/`a3`：允许 MessageEnd 前对已 End 的 ParallelSafe 执行。
2. 方言门闩：仅 `openai-responses` | `anthropic-messages`。
3. abort/截断：cancel 未完成抢跑；截断 tool 不执行。
4. 不把部分结果塞进仍在飞的同一 HTTP 请求。
5. 尽量零旋钮；逃生至多 `tool_batch.eager: bool`。

## Experiment 2（promote 前必做）

在 Responses 下量 `ToolCallEnd(t0)` → `Done/MessageEnd(t1)` 空隙 vs 工具墙钟。仅当空隙经常 ≥ 工具耗时再 promote。

用户提示词见会话交付。

## Non-Goals

Completions 抢跑；结果流式回灌同轮 LLM；放开 MCP 并行。

## Status

**purpose-draft** — 后置；等实验 2。

## Ethics

- risk_level: high
- required_evidence: 方言门闩；abort 不留脏写；空隙分布
