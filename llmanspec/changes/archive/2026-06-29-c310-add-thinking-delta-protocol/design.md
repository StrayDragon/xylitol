---
change_id: c310-add-thinking-delta-protocol
title: wire protocol ThinkingDelta spec 补登记
---

# Design — wire protocol ThinkingDelta spec 补登记

## 背景

本变更是对既有 commit `03f8866` 的 **spec 补登记**，不含任何代码改动。`Event::ThinkingDelta` 已经实现并修正了往返映射，只是 `app-protocol` spec 的 `ip7` 当时没有同步更新。

## 为什么没有设计权衡

- 新增 `ThinkingDelta` 而非复用 `MessageUpdate`：commit `03f8866` 已决定，与 `TextDelta` 对称，避免把增量事件塞进累计型 `MessageUpdate` 造成语义混淆。
- 往返正确性：已实现，本变更只把它写进 spec。

因此本变更没有新的设计决策需要记录，design.md 仅用于满足 llman stage 守卫。

## 受影响 spec

- `app-protocol`：修改 `ip7`，补充 `TextDelta` 与 `ThinkingDelta` 变体及往返约束。

## 验证

- `llman sdd validate c310-add-thinking-delta-protocol --strict --no-interactive` 通过。
- 归档后 `llman sdd validate app-protocol --strict --no-interactive` 通过。
