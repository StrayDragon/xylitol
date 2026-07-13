---
change_id: c640-update-app-tui-session-tree-fold
title: "产品会话树：fold / 分支跳转接线"
status: purpose-draft
priority: 640
depends_on: ["c635-update-app-tui-session-tree-filter"]
author: agent
track: A
---

# c640-update-app-tui-session-tree-fold

## Why

深会话需要折叠子树与分支跳转；demo（c467）已有，产品未接线。

## Purpose

树开：Ctrl/Alt+←→ fold / 分支跳转；连接符 ⊞/⊟；可选 Shift+L/T annotation（可拆 future）。

## What Changes（实现时）

1. 产品接线包 TreeSelector fold API。
2. harness：fold 后可见行减少；跳转落在预期节点。

## Capabilities

- `app-tui-session-tree`（modify）

## Design SSOT

- [`session-tree.md`](../../../src/app/tui/design/session-tree.md) § Fold
- playground `tree-power` · fold 态

## Impact

- 产品树槽；不改 session 持久化格式

## Out of scope

- Shift+F fork（c645）；可与 annotation 同批或 future

## Ethics

- risk_level: low
- prohibited_actions: 在产品面重写包 fold 算法
- required_evidence: harness + playground fold 形状

## Depends

- **c635**（先 filter，避免键位与状态行纠缠）
