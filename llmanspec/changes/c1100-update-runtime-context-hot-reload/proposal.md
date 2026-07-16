---
change_id: c1100-update-runtime-context-hot-reload
title: "Context 文件热重载：AGENTS.md / SYSTEM 等不改历史"
status: purpose-draft
priority: 1100
apply_band: P2-system
depends_on: []
author: agent
track: R
wave: reload-foundations
domain: runtime
---

# c1100-update-runtime-context-hot-reload

## Why

启动时已加载 context（AGENTS.md / CLAUDE.md 等）与 system prompt 文件。`/reload` 需要对齐 pi：重读 context，**影响后续轮次**，不改写已持久化历史消息。

## Purpose

提供 context / system（含 append）热重载：刷新「下一轮可见」的系统侧上下文；发出订阅通知；明确不 mutate 历史 entry。

## What Changes（升格 full 时）

- reload context API：再发现 → 重建 system 侧输入（或等价）
- 与 session 历史隔离的合约（MUST NOT 改 jsonl 既有消息）
- 诊断与 Trust 闸
- delta：`agent-prompt` · `runtime-resource-discovery`

## Capabilities

- `agent-prompt`（modify）
- `runtime-resource-discovery`（modify）

## Out of scope

- prompt templates 产品面（用 skill 代替）
- `/reload` 总控 UI（c1120）

## Ethics

- risk_level: medium
- prohibited_actions: 重载改写历史 user/assistant/toolResult；未信任加载项目 context
- required_evidence: 重载前后 session 文件历史条目不变；新一轮 system 反映新文件
- escalation_policy: 若须「中途插入 env 消息」需显式产品确认

## Depends

- []

## Downstream

- `c1120-add-app-tui-reload-slash`
