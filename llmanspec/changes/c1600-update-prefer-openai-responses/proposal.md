---
change_id: c1600-update-prefer-openai-responses
title: OpenAI 兼容默认 Responses；Completions 降为遗留；方言可观测
status: purpose-draft
priority: 1600
depends_on:
  - c1598-fix-bootstrap-honor-model-api
author: agent
---

# c1600-update-prefer-openai-responses

## Discussion context

见 [`../DEFERRED-responses-tool-batch-CONTEXT.md`](../DEFERRED-responses-tool-batch-CONTEXT.md)。

方向：逐步放弃 Completions 日常路径；llama.cpp 等高版本已支持 Responses。本提交已将仓库 `.xylitol/config.yaml` 全部改为 `openai-responses`（在 c1598 接线生效后配置=实际）。

## Why

产品方向：逐步放弃 Chat Completions 作为日常路径。Responses 上工具块 `ToolCallEnd` 更早，也为 `c1615` 抢跑留窗口。

今日问题曾包括：配置写 Completions 却静默 Responses（`c1598`）；方言已在 `llm.request.api` 导出但未升格为一等排障信号。

## Product intent（开箱）

| | 目标 |
|---|---|
| OpenAI 兼容缺省 `api` | **`openai-responses`** |
| Completions | 遗留显式档 |
| 多面配置 | 一个 `models.*.api`，禁止第二开关 |
| 观测 | 每次 `llm.request` MUST 带 dialect；文档教看 `api=` |

## Decisions（意向）

1. 先 `c1598`（已落地代码）。
2. 示例/开发配置 → Responses（本提交开发 yaml 已改）。
3. `default_api()` / 文档对齐 Responses。
4. OTEL：保持 `api`；SHOULD 在 turn/session 带 `xylitol.model.api`。
5. Pre-1.0 保留 Completions 代码路径。

## Non-Goals

- 删除 Completions adapter；流中抢跑（`c1615`）；提示词 / tool_batch 默认（`c1605`/`c1610`）

## Status

**purpose-draft** — 开发配置已切 Responses；产品默认/文档/观测加强待 promote。

## Ethics

- risk_level: medium（端点无 Responses 时须 Completions 逃生舱）
- required_evidence: 本机 `/v1/responses` 烟雾；Langfuse `api` 与配置一致
