---
change_id: c1060-update-ai-bridge-openai-remote-count
title: "ai-bridge：OpenAI Responses RemoteCount 全量接线"
status: full
priority: 1060
depends_on:
  - "c1030-add-package-ai-bridge"
author: agent
track: B
---

# c1060-update-ai-bridge-openai-remote-count

> 产品优先 **Api > RemoteCount > LocalTokenizer > Heuristic**。
> 本地 tokenizer 要等 turn end / 锚点才稳，不能替代远程精确 count。

## Why

c1030 首期以 Anthropic `count_tokens`（或 stub）满足 RemoteCount 合约。
OpenAI 现有官方 `POST /v1/responses/input_tokens`，应与 Anthropic 对称接入，避免 OpenAI 路径只能落 LocalTokenizer/Heuristic。

## Purpose

1. 实现 `OpenAiResponsesRemoteCounter`（`POST {base}/v1/responses/input_tokens`），复用 Responses `input` 消息转换。
2. `RemoteCounter` port 下 Anthropic / OpenAI / Stub 并列；registry 标明 Responses 路径可 RemoteCount。
3. estimate 装配：可注入预取 `remote_count_tokens` 或经 helper 先 await count 再 estimate；失败降级，provenance 非 Api。
4. wiremock / stub 单测覆盖成功与失败降级。

## What Changes

- `provider/remote_count.rs`：OpenAI input_tokens 客户端
- `openai_responses`：导出 input 转换供 count 复用
- delta：`package-ai-bridge-accounting`（RemoteCount 覆盖 OpenAI）
- 可选：`token_estimator` / 文档说明装配

## Capabilities

- `package-ai-bridge-accounting`（modify）

## Out of scope

- 改变优先级顺序
- 每个 TextDelta 打 count API
- 抽 llm-types；AgentMessage 双轨消解（另案）

## Ethics

- risk_level: low–medium
- prohibited_actions: RemoteCount 失败标成 Api；默认每 delta 远程 count
- required_evidence: OpenAI 成功路径 provenance=RemoteCount；失败降级非 Api

## Depends

- **c1030-add-package-ai-bridge**（已归档）
