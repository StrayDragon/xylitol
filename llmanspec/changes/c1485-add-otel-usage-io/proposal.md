---
change_id: c1485-add-otel-usage-io
title: Langfuse generation usage + optional observation I/O 档
status: in-progress
priority: 1485
depends_on:
- c1480-add-otel-genai-langfuse
author: agent
branch: feature/c1485-add-otel-usage-io
base_sha: c8fa127776ea9cd8bb8ddcc23dfd7c9661721249
checkpointed: false
---

# c1485-add-otel-usage-io

## Why

1. c1480 已让 Langfuse 按 session 聚合并把 `provider.request` 标成 generation，但 **usage 仍为空**，无法看成本/吞吐。
2. roadmap **M3**：需要可选 observation input/output；默认仍只元数据，避免「开了 OTEL = 全文上云」。
3. 用量来自流结束的 `AiBridgeChunk::Done`；须在 fastrace span drop 前写入属性。

## What Changes

### Phase A — Usage（默认开启，随既有 span 闸）

- `provider.request` 在收到 `Done { usage: Some(..) }` 时写入：
  - `gen_ai.usage.input_tokens` / `gen_ai.usage.output_tokens`
  - `langfuse.observation.usage_details`（JSON：`input`/`output`/`total`，可选 cache）
- 无 usage 时不写这些属性（不伪造 0）

### Phase B — Observation I/O 档（配置 opt-in）

- `[otel].observation_io = none | truncated | full`（默认 **none**）
- `none`：MUST NOT 写 `langfuse.observation.input` / `output`（延续 otel8）
- `truncated`：截断文本（复用既有 provider-trace 上限量级）写入 input/output
- `full`：尽量完整（仍受合理上限保护，避免单 span 撑爆 OTLP）
- 装配经组合根 → 静态闸（对齐 `provider_trace_active`），**不**让 ai-bridge 依赖 infra/config 类型

## Out of scope

- 父子 span 树 / 合并分散的 `Span::root`
- 成本价目表 / Langfuse model pricing 配置
- c1490 Collector 分流
- 默认开启完整 prompt/completion

## Capabilities

- `infra-otel`（otel9 usage、otel10 I/O 档）

## Impact

- Langfuse generation 显示 token usage
- 仅当用户显式设 `observation_io` 才有 observation I/O
