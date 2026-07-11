---
change_id: c540-update-package-tui-diff-edges
title: "package-tui-diff：边角打磨（CJK/空半栏/snapshot）"
status: full
priority: 540
depends_on: []
author: agent
track: P
---

# c540-update-package-tui-diff-edges

## Why

Diff 主路径（unified / SBS / edit-format / SBS 无行底）已归档；轨 P 仍缺一批**可单测的边角**：窄宽 CJK 折行不漂、SBS 空半栏不伪造行号、edit-format 与 SBS 关键 snapshot 回归，便于开闸前肉眼+机器双验。

## Purpose

在既有 `package-tui-diff` 上增补边角合约与 harness/snapshot，不改产品 host。

## What Changes

1. 增补 Diff 边角 requirements（CJK 窄宽、空半栏、snapshot 护栏）。
2. 补/扩 `packages/xylitol-tui` Diff 单测与必要 snapshot。
3. 对照 `src/app/tui/design/diff-block.md` MUST，缺口只补包侧。

## Capabilities

- `package-tui-diff`（修改）

## Impact

- `packages/xylitol-tui/src/components/diff.rs` 及 Diff 相关测试/snapshot
- 可选：`design/diff-block.md` 一句指针（非产品接线）

## Out of scope

- 产品 expandable Diff 壳 / 轨 B host
- 默认捆绑 syntect

## Ethics

- risk_level: low
- prohibited_actions: 不改主 crate agent/infra；不把 SBS 行底加回
- required_evidence: Diff 单测绿；`llman sdd validate` 通过
