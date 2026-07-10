---
change_id: c461-expose-steer-followup-seam
title: "暴露 steer / follow-up 队列到 Driver seam"
status: purpose-draft
priority: 461
depends_on: ["c450-revise-app-tui-contract"]
author: agent
track: B
---

# c461-expose-steer-followup-seam

> **status: purpose-draft**

## Why

pi：流中 Enter=steer（下一 ReAct 迭代注入），Alt+Enter=follow-up（整轮 AgentEnd 后）。xylitol 已有 `MessageHooks` / `XyEvent::QueueUpdate` 骨架，但 **Driver/ReAct 未完整暴露队列 API**，TUI 无法接线。

## Purpose

在 agent runtime + `Driver` +（如需）`protocol::Command` 暴露 `steer` / `follow_up` / `clear_queue` / 队列计数；发出 `QueueUpdate`；hooks 扩展点保留。

## What Changes（意向）

1. 对齐 pi `agent-session` 队列语义。
2. `Driver::steer` / `follow_up` / `clear_queue`；InProcess + Remote 预留。
3. ReAct 循环在迭代边界拉取 steering；AgentEnd 后处理 follow-up。
4. 单测覆盖队列与 abort 交互。

## Capabilities

- `agent-runtime`（modify）
- 可能 `protocol-app` / `app-tui-bridge`

## Out of scope

- TUI 键位接线（c480）
