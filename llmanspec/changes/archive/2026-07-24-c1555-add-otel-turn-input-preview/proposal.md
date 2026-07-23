---
change_id: c1555-add-otel-turn-input-preview
title: agent.turn 根挂用户提示摘要作 Session 预览
status: full
priority: 1555
depends_on:
- c1485-add-otel-usage-io
- c1495-add-otel-session-span-tree
branch: feat/c1550-c1555-otel-observation-io
author: agent
base_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
checkpointed: true
checkpoint_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
---

# c1555-add-otel-turn-input-preview

## Why

Langfuse Sessions 列表以 **trace 根** 的 input/output 作预览。当前根是 `agent.turn`，即使开启 `observation_io`，I/O 只挂在子节点 `llm.request` 上，Session 仍显示 “This trace has no input or output.”，不便扫多轮对话。

## What Changes

1. 当 `[otel].observation_io` 为 `truncated` 或 `full` 且低频观测闸激活时，`agent.turn` 根 MUST 写入 `langfuse.observation.input` = 本轮用户提示文本摘要（按同档硬顶截断）
2. `observation_io` 缺省或 `none` 时 MUST NOT 在 `agent.turn` 上写 observation input/output
3. 本切片 **不** 强制写 turn 根 output（assistant 全文仍看 generation）；Session 预览以用户提示为主

## Out of scope

- tool observation I/O（c1550）
- 改变 `llm.request` 的 generation I/O 语义
- `token.estimate` 双 root

## Capabilities

- `infra-otel`（otel15 turn root input preview）

## Impact

- 开启既有 `observation_io` 后，Session 列表可读用户提示
- 默认 none 行为不变

## Ethics

- risk_level: medium（用户提示上云）
- prohibited_actions: 默认写入全文；绕过 observation_io 闸
- required_evidence: none 单测；truncated 截断；validate
