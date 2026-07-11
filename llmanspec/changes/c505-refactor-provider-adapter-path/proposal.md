---
change_id: c505-refactor-provider-adapter-path
title: "Provider 单一 XyModel 装配路径"
status: proposed
priority: 505
depends_on: []
author: agent
track: A
---

# c505-refactor-provider-adapter-path

## Why

Completions 路径存在 `OpenAIProvider: XyModel` → `OpenAiCompletionsAdapter: LlmAdapter` → `AdapterXyModel: XyModel` 双包装；与「业务只认 XyModel、方言收在适配器」冲突，也阻碍后续加厂商。

## What Changes

1. 折叠 Completions：底层 HTTP 客户端不再直接 `impl XyModel` 对外；只经 `LlmAdapter` → 统一外壳 → `XyModel`。
2. 保持 Anthropic / Responses 已有适配器模式一致。
3. 护栏：`agent/`/`domain/` 零 `async_openai` import。
4. factory 选择逻辑保持基于 `XyModelConfig`；删除 placeholder 双路径注释债务。

## Capabilities

- `infra-provider`（add pa6, pa7）

## Impact

- `src/infra/provider/{openai,adapter/*,factory}.rs`

## Out of scope

- 新增第三家厂商交付（抽象位保留即可）
- OAuth / 订阅登录
