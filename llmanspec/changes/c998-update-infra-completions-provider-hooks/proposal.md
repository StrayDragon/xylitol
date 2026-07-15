---
change_id: c998-update-infra-completions-provider-hooks
title: "Completions adapter：HTTP 三缝对齐 Responses"
status: full
priority: 998
depends_on: ["c990-add-test-hooks-wiring-bdd"]
author: agent
track: A
wave: hooks-wiring-bdd
domain: c995
---

# c998-update-infra-completions-provider-hooks

## Why

c735 在 Responses/Anthropic 落地三缝；Completions 仅存 `_hooks`，显式 `openai-completions` 时 prefix-cache / header 观测静默失效。

## Purpose

1. Completions MUST 与 Responses **同序**调用：`before_provider_headers` → `before_provider_request` → send → `after_provider_response`（成功流消费前）。
2. **策略 A（写死）**：Completions 走 raw reqwest（或等价可取 `HeaderMap` 的路径），复用 `infra/hooks/http.rs`；不依赖 async-openai 插点凑合。
3. 默认 provider 仍可为 Responses；本 change 不改 default。
4. 空 hooks noop；Modify body/headers 可测；启用 wiring 操作「Completions 发送流式请求」或等价单测+BDD。

## What Changes

- `openai_completions` / `openai.rs` HTTP 路径
- `agent-hooks`（h12 扩 Completions）+ 测；勾销 Completions 缺口

## Capabilities

- `agent-hooks`
- `infra-provider`（若需补合约）
- `test-hooks-wiring`（可选例子）

## Out of scope

- 改默认 adapter；SSE body 进 hook（c999）；Driver session/bang hooks

## Ethics

- risk_level: medium
- prohibited_actions: 破坏 Completions 流式/工具调用而不测；把成功 SSE body 默认塞进 hook
- required_evidence: Modify + noop 测；既有 Completions/流测不回归
- escalation_policy: raw reqwest 风险过大时 STOP 报告，不退回假 B

## Depends

- **c990-add-test-hooks-wiring-bdd**
