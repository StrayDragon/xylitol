---
change_id: c665-update-app-tui-abort-feedback
title: "产品 TUI：abort / 工具取消的 status 反馈"
status: purpose-draft
priority: 665
depends_on: ["c660-fix-agent-tool-abort-process"]
author: agent
track: B
---

# c665-update-app-tui-abort-feedback

## Why

内核取消可靠后，用户仍需在 status / scrollback **看见**「已中止」，避免以为还在 Working。

## Purpose

abort 后：status 短提示（或系统行）→ 回到 idle；与 c493 Compacting/Retry 同为「一行 status」族，不堆墙。

## What Changes（实现时）

1. bridge / status：消费 abort / tool-cancelled 事件（或等价）。
2. 形状对齐 [`status.md`](../../../src/app/tui/design/status.md) / [`errors.md`](../../../src/app/tui/design/errors.md)。
3. harness：busy → Esc → 可见中止反馈 → idle。

## Capabilities

- `app-tui-bridge` / `app-tui-chrome`（modify）

## Design SSOT

- [`status.md`](../../../src/app/tui/design/status.md)
- [`errors.md`](../../../src/app/tui/design/errors.md)
- [`keybindings.md`](../../../src/app/tui/design/keybindings.md) Esc abort

## Impact

- 产品 TUI 呈现；依赖 c660 语义正确

## Out of scope

- computer-use；重做 abort 键位

## Ethics

- risk_level: low
- prohibited_actions: 用多行 debug 墙冒充反馈
- required_evidence: harness 一条 abort 反馈路径

## Depends

- **c660**（松：UI 可先 mock 事件，但 archive 前须真取消）
