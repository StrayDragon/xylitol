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

## Discussion context（2026-07-24）

- 方向：逐步放弃 Completions 日常路径；llama.cpp 等高版本已支持 Responses。
- `c1598` 前配置字面与实际方言脱节；修后开发仓 `.xylitol/config.yaml` 已全部改为 `openai-responses`。
- Responses 上 `ToolCallEnd` 更早，才值得谈流中抢跑（`c1615`）。
- `llm.request` 已有属性 `api`；本 change 将其升格为排障一等信号（文档 / turn 摘要）。

## Why

开箱与文档应对齐 Responses；Completions 仅作遗留显式档，避免多开关。

## Product intent

| | 目标 |
|---|---|
| 缺省 `api` | `openai-responses` |
| Completions | 遗留显式档 |
| 配置面 | 仅 `models.*.api` |
| 观测 | 每次 `llm.request` 带 dialect；文档教看 `api=` |

## Decisions（意向）

1. 依赖 `c1598`（已接线）。
2. 示例/产品默认/文档 → Responses。
3. OTEL：保持 `api`；SHOULD 在 turn/session 带 `xylitol.model.api`。
4. Pre-1.0 保留 Completions 代码路径。

## Non-Goals

删 Completions adapter；流中抢跑（`c1615`）；提示/tool_batch 默认（`c1605`/`c1610`）。

## Status

**purpose-draft** — 开发 yaml 已切 Responses；产品默认/文档/观测加强待 promote。

## Ethics

- risk_level: medium（端点无 Responses 时须 Completions 逃生舱）
- required_evidence: `/v1/responses` 烟雾；Langfuse `api` 与配置一致
