---
change_id: c310-add-thinking-delta-protocol
title: 把 Event::ThinkingDelta 写入 wire protocol spec
status: proposed
priority: 310
depends_on:
  - c300-add-provider-adapter-layer
author: agent
created: 2026-06-29
---

# 把 Event::ThinkingDelta 写入 wire protocol spec

## 背景与动机

commit `03f8866` 给 `src/protocol/event.rs` 的 `Event` 枚举新增了 `ThinkingDelta { text }` 变体，并修正了 `XyEvent::ThinkingDelta` 的 `to_wire_event` 映射（此前被错误映射成空的 `MessageUpdate`，导致 reasoning 内容在 client↔core 边界被丢弃）。

但 `app-protocol` spec 的 `ip7`（event-variants-complete）只列出了 `TurnStart/TurnEnd/MessageStart/MessageEnd/MessageUpdate/ToolExecutionUpdate/CompactionEnd`，没有 `TextDelta` 也没有 `ThinkingDelta`。spec 与实现已经脱节。本变更把 `ThinkingDelta`（顺带补 `TextDelta`）写进 spec。

注：这是对既有 commit 的 spec 补登记，不含代码改动。

## 变更内容

更新 `app-protocol` spec 的 `ip7`：

- `protocol::Event` 必须覆盖 `TextDelta` 与 `ThinkingDelta` 两个流式增量变体；
- `ThinkingDelta` 必须能正确往返 `XyEvent::ThinkingDelta`（不被降级为空 `MessageUpdate`）。

## 受影响能力

- `app-protocol`（更新）：ip7 补充 thinking/text 增量事件。

## 影响面

- **代码**：无改动（`Event::ThinkingDelta` 已在 `03f8866` 实现）。
- **测试**：`src/protocol/event.rs::tests::thinking_delta_roundtrips_through_wire_event` 已覆盖往返。

## 验收标准

- `llman sdd validate c310-add-thinking-delta-protocol --strict --no-interactive` 通过。
- `app-protocol` 的 ip7 与 `src/protocol/event.rs` 实际变体一致。
