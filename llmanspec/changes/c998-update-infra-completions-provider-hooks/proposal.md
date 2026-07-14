---
change_id: c998-update-infra-completions-provider-hooks
title: "Completions adapter 完整 provider HTTP 三缝"
status: purpose-draft
priority: 998
depends_on: ["c735-update-agent-hooks-pi-parity"]
author: agent
track: A
wave: hooks-pi-parity-followup
domain: c995
---

# c998-update-infra-completions-provider-hooks

> **status: purpose-draft** — 承接 `c735`：OpenAI Completions 仅存 hooks 句柄、未跑 HTTP 三缝。
> **depends_on c735**：归档后再 apply。

## Why

c735 在 **Responses** 与 **Anthropic** 上落地 `before_provider_headers` / `before_provider_request` / `after_provider_response`。Completions 走 async-openai，无裸 `HeaderMap`/易取的 pre-SSE status，故只存 `_hooks`。当用户显式 `api: openai-completions`（或网关只提供 chat completions）时，prefix-cache 与 header 观测会静默失效。

## Purpose

1. Completions 路径 MUST 与 Responses/Anthropic **同序**调用三缝（或文档化的等价行为）。
2. 实现策略（升格时择一写死）：
   - **A**：Completions 改 raw reqwest（对齐 Responses），复用 `infra/hooks/http.rs`；或
   - **B**：在 async-openai 可插点尽量接 body/headers；缺口 MUST 在 design 标明并测。
3. 默认 `provider: openai` 仍可为 Responses；本 change 不强制改 default。
4. 单测：注入 dispatcher 后 Completions 发送前 body 可被 Modify；空 hooks noop。

## What Changes（升格后预期）

- `openai_completions` / `openai.rs` 改造或包装
- `agent-hooks` 或 infra 测；勾销 c735 future「Completions HTTP 三缝」行

## Capabilities

- `agent-hooks` 和/或新建/既有 provider capability（升格时声明）

## Out of scope

- 改默认 adapter 为 Completions
- SSE 抓包（`c999`）
- Driver session / bang hooks（`c995`–`c997`）

## Ethics

- risk_level: medium（改 HTTP 客户端路径）
- prohibited_actions: 破坏现有 Completions 流式/工具调用回归而不测
- required_evidence: Completions + hooks modify 单测；既有 provider 流测不回归
- escalation_policy: async-openai 无法取 headers 时升为 A（raw reqwest）或缩小 MUST

## Depends

- **c735-update-agent-hooks-pi-parity**
