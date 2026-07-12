---
change_id: c493-add-app-tui-compaction-retry-ui
title: "app-tui Compaction / AutoRetry 状态呈现"
status: draft
priority: 493
depends_on: ["c485-add-app-tui-vertical-slice"]
author: agent
track: B
---

# c493-add-app-tui-compaction-retry-ui

> **status: draft**（已从 purpose-draft 升格；待 `llman-sdd-apply`）

## Why

`apply_xy_event` 已消费 `CompactionStart/End` 与 `AutoRetryStart/End`，但 **End 后 sticky status**（停在 `Compacting` / `Retry n/m`）未恢复 `Working`；`design/compaction-status.md` 仍是草稿。需要把合约钉死并补单测，对齐 chrome「忙碌至多一行」。

## What Changes

1. **Compaction**：Start → status `Compacting` + scrollback System（含 reason）；End → complete/aborted 说明；若仍 Busy → status 恢复 `Working`。
2. **AutoRetry**：Start → status `Retry {attempt}/{max_retries}`；End → 失败时 System 说明；若仍 Busy → 恢复 `Working`（成功/失败皆然）。
3. **Chrome**：Compacting / Retry 仍为单行 status；细节进 scrollback，不增高 status。
4. **设计**：落地 `design/compaction-status.md`（去掉纯草稿措辞）。
5. **非目标**：改 compaction 编排 / auto-retry 策略、多行进度条、改 `XyEvent` 形状、产品 `/compact` slash（另 change）。

## Capabilities

- `app-tui-bridge`：事件→status/scrollback 语义与 End 复位
- `app-tui-chrome`：单行 status 约束（Compacting / Retry）

## Impact

- 触达：`src/app/tui/bridge.rs`（主）、可选 `tests.rs` / chrome 渲染断言、`design/compaction-status.md`、`DESIGN.md` 指针。
- 风险：低；缝已存在，主要是 End 复位 + 合约/测试。
