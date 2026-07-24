---
change_id: c1590-fix-otel-generation-request-input
title: 补齐 llm.request generation 请求体 input，并在 abort 时 flush partial + ERROR
status: active
priority: 1590
depends_on:
- c1485-add-otel-usage-io
- c1555-add-otel-turn-input-preview
author: agent
branch: feat/c1590-otel-generation-io
base_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
checkpointed: true
checkpoint_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
---

# c1590-fix-otel-generation-request-input

## Why

实测（session `c791ab6e-…` / Responses 流）：`observation_io=truncated` 时 `llm.request` 仍 **input/output 皆空、usage=0**。

根因：

1. generation **input** 只在 raw 事件名 ∈ `{response.json, chat.completion.json, message.json}` 时缓冲；**流式**路径从不 emit 这些名。
2. `attach_observation_io` 仅在 `Done { usage: Some }` 触发；**abort 无 Done** → `output_buf` 有 partial 也不上云；且无 `level`/`status_message`。

Session JSONL 已有 abort partial（c1595）；Langfuse 应提供**可过滤的过程预览**，不镜像整份 transcript。

## What Changes

1. `observation_io ≠ none` 时：adapter 在发出 HTTP 前把 **截断后的完整 request JSON**（`build_body` / 等价序列化）写入 generation `langfuse.observation.input`（**不**依赖 raw 事件名）。
2. 任意 `Done`（有无 usage）以及 span **提前结束（Drop / abort）** 时：按档 flush 已缓冲的 input/output；usage 仍仅在有真实 usage 时写入（**不**伪造零用量）。
3. 提前结束（无成功 Done）时：generation MUST 写 `langfuse.observation.level=ERROR` 与 `langfuse.observation.status_message=aborted`。
4. **不**改变默认 `observation_io=none`；**不**镜像 session JSONL；**不**强制 turn 根 ERROR / thinking 进 output。

## Capabilities

- `infra-otel`（修订 otel10；新增 abort/finalize 与 request-body input req）

## Impact

- `packages/xylitol-ai-bridge`：`ProviderRequestTrace` + openai / responses / anthropic adapters
- Langfuse：流式与 abort 轮次可读 request + partial output，可按 ERROR 过滤

## Ethics

- risk_level: medium（请求体可能含路径/密钥片段；受 truncated/full 硬顶）
- prohibited_actions: 默认 full；绕过 `observation_io` 闸；伪造 usage
- required_evidence: none 单测；truncated 硬顶；Responses 路径带 input；abort Drop 带 ERROR + partial output
- escalation_policy: input 形状锁定为「截断完整 request JSON」（已确认）
