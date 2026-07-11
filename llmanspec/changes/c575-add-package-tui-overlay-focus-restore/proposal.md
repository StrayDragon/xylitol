---
change_id: c575-add-package-tui-overlay-focus-restore
title: "package-tui-overlay：完整 focus-restore（eligible/blocked）状态机"
status: purpose-draft
priority: 575
depends_on: []
author: agent
track: P
---

# c575-add-package-tui-overlay-focus-restore

> **status: purpose-draft**
>
> **闸门**：本草案仅立项意向；**合入 `main`（当前 `polish/tui-components` PR）之后**再 `llman-sdd-continue` → specs/tasks → apply。合入前勿实现。

## Why

pi-tui Overlay 有 eligible / blocked / resume 完整 focus-restore；xylitol-tui 目前仅最小 `hide` / `focus` / `unfocus`（`PI_DELTAS` **D08**）。产品开闸后多 overlay（trust / settings / confirm）需要可预测的焦点恢复，否则会丢焦点或抢焦点。

## Purpose

在 `packages/xylitol-tui` 的 `TUI` Overlay 栈上补齐与 pi 语义对齐的 focus-restore（可测、host 驱动友好），不改变「产品路径 host 驱动 / `start()` 仅 demo」决议。

## What Changes（意向）

1. Overlay 栈记录隐藏前焦点（root vs overlay id）与 blocked 原因。
2. `hide` / `show` / 栈顶变化时按 eligible 规则恢复焦点；blocked 时排队至条件解除。
3. 单测 + harness（无真实 TTY）；可选 demo 内联/overlay 槽验证。
4. 更新 `PI_DELTAS` D08：延后 → 已落地（或缩小「不得回退」范围）。

## Capabilities

- `package-tui-overlay`（或现有引擎 spec 增量；continue 时定名）

## Out of scope

- 产品 `src/app/tui` 接线（轨 B）
- stdin-buffer / Image 组件（D02 / D11 仍不移植）
- 合入 main 前的任何实现提交

## Ethics

- risk_level: medium（焦点状态机易回归）
- prohibited_actions: 合入 main 前 apply；把 coding-agent 产品壳塞进包
- required_evidence: continue 后 `validate`；实现后 `cargo test -p xylitol-tui` 相关绿
