---
change_id: c1060-update-ai-bridge-openai-remote-count
title: "ai-bridge：OpenAI Responses RemoteCount 全量接线"
status: purpose-draft
priority: 1060
depends_on:
  - "c1030-add-package-ai-bridge"
author: agent
track: B
---

# c1060-update-ai-bridge-openai-remote-count

> **status: purpose-draft** — 待 OpenAI/兼容端路径上 RemoteCount 成为刚需（本地 tokenizer 不足）时 promote。

## Why

c1030 accounting 优先级含 `RemoteCount`，首期以 Anthropic `count_tokens`（或 stub）满足合约。OpenAI Responses 的 input token count API（及兼容端能力差）需单独补强，避免在 c1030 内无限扩大。

## Purpose

1. 对 OpenAI Responses（及文档化的兼容端）实现 RemoteCount 路径。
2. registry 标明哪些 model/api 支持；失败降级不变。
3. 单测/集成测覆盖成功与降级。

## Capabilities（promote 时）

- modify `package-ai-bridge-accounting`

## Out of scope

- 改变优先级顺序；Heuristic 语义

## Depends

- **c1030-add-package-ai-bridge**
