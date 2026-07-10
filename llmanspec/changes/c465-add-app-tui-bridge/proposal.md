---
change_id: c465-add-app-tui-bridge
title: "app-tui-bridge：XyEvent→UI 模型与流生命周期"
status: purpose-draft
priority: 465
depends_on: ["c460-add-app-tui-host", "c461-expose-steer-followup-seam"]
author: agent
track: B
---

# c465-add-app-tui-bridge

> **status: purpose-draft**

## Why

渲染不得直接 match 领域事件；需单缝翻译。流生命周期需重新对齐 pi（多 TurnEnd + AgentEnd + 队列），不沿用已删旧 TUI 的临时结论。

## Purpose

定义 UI-only 模型与 bridge：消费 `Driver` 事件流；`ToolExecutionEnd` 抽取 `display_diff`；处理 `QueueUpdate`；明确 turn/agent 边界何时清 streaming 态。

## What Changes（意向）

1. UI 类型：User/Assistant/Thinking/Tool/Diff/System/Error/Status…
2. 单缝 `apply_xy_event`；渲染层零 `XyEvent`。
3. 生命周期：以 pi 为准重读——中间 `TurnEnd` ≠ 用户轮结束；`AgentEnd` + 无 follow-up 才复位 idle；steer 不打断当前工具。
4. 未处理事件：tracing 降级，不 panic。
5. RemoteDriver 路径保留类型，默认 InProcess。

## Capabilities

- `app-tui-bridge`

## Out of scope

- Codex 式 TranscriptView / 专用消息浏览面（c470 已搁置）
- 会话树 UI（c454/c456/c491）
- 具体 Markdown/Diff 行绘制细节（包组件 + demo；产品 live 行另议）
