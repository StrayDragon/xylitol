---
change_id: c1600-update-prefer-openai-responses
title: OpenAI 兼容默认 Responses；Completions 降为遗留；方言可观测
status: in-progress
priority: 1600
depends_on:
- c1598-fix-bootstrap-honor-model-api
author: agent
branch: feat/c1598-c1600-model-api
base_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
checkpointed: true
checkpoint_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
---

# c1600-update-prefer-openai-responses

## Why

开箱与文档应对齐 Responses；Completions 仅作遗留显式档。manifest 缺省仍写死 Completions，与 `AdapterKind::default_for` 不一致。

## Product intent

| | 目标 |
|---|---|
| 缺省 `api`（OpenAI 兼容） | `openai-responses` |
| Completions | 遗留显式档 |
| 配置面 | 仅 `models.*.api` |
| 观测 | `llm.request.api` 已有；`agent.turn` SHOULD 带 `xylitol.model.api` |

## Decisions

1. 依赖已归档的 `c1598`（显式 api 接线）。
2. JSON manifest 省略 `api` → 按 provider 选 `AdapterKind::default_for` 字符串（OpenAI→responses，Anthropic→messages）。
3. 示例/产品文档：推荐 Responses；Completions 仅注释为遗留逃生。
4. `agent.turn` 写入 `xylitol.model.api`（启动时当前模型）。
5. Pre-1.0 保留 Completions 代码路径。

## Non-Goals

删 Completions adapter；流中抢跑（`c1615` shelved）；改 tool_batch。

## Status

**applying** on `feat/c1598-c1600-model-api`。

## Ethics

- risk_level: medium（端点无 Responses 时须显式 `openai-completions`）
- required_evidence: manifest 单测；pa4；Langfuse/trace 可见 api
