---
change_id: c465-add-app-tui-bridge
title: "app-tui-bridge：XyEvent→UI 模型与流生命周期"
status: proposed
priority: 465
depends_on: ["c460-add-app-tui-host", "c461-expose-steer-followup-seam"]
author: agent
track: B
---

# c465-add-app-tui-bridge

> **开闸（2026-07-11）**：Track B 可推进；本变更为产品 TUI 接线 P0（可 apply）。轨 A（含 QueueUpdate）与轨 P 包打磨已合入当前主开发分支。

## Why

渲染不得直接 match 领域事件；需单缝翻译。流生命周期需重新对齐 pi（多 TurnEnd + AgentEnd + 队列），不沿用已删旧 TUI 的临时结论。`QueueUpdate` 已在 EventStream / 线协议落地（c525/c540），bridge 可消费进程内流。

## What Changes

1. UI 类型：User/Assistant/Thinking/Tool/Diff/System/Error/Status…
2. 单缝 `apply_xy_event`；渲染层零 `XyEvent`。
3. host `select!` 合流 Tick + 输入 + **Driver::run 事件流**（不再丢弃 Driver）。
4. 生命周期：中间 `TurnEnd` ≠ 用户轮结束；`AgentEnd` + 无 follow-up 才复位 idle；steer 不打断当前工具。
5. 未处理事件：tracing 降级，不 panic。
6. 处理 `QueueUpdate`（徽章 / 状态行）。

## Capabilities

- `app-tui-bridge`

## Impact

- `src/app/tui/**`（host / bridge / UI 模型）

## Out of scope

- Codex 式 TranscriptView（c470 paused）
- 会话树活树 / slash 全量（c480+；c491 stub 不扩展）
- 具体 Markdown/Diff 行绘制细节（复用轨 P 包组件）
- Overlay 完整 focus-restore（包侧 c575，可选、不阻塞）
