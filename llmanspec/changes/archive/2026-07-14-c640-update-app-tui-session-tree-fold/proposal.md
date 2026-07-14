---
change_id: c640-update-app-tui-session-tree-fold
title: "产品会话树：fold / 分支跳转接线（对齐 pi）"
status: full
priority: 640
depends_on: ["c635-update-app-tui-session-tree-filter"]
author: agent
track: A
---

# c640-update-app-tui-session-tree-fold

## Why

深会话需要折叠子树与分支跳转。包 `TreeSelector`（c467）与 demo 已有 ⊞/⊟ + Ctrl/Alt+←→；产品树槽目前只转发裸 ←→（翻页），**吞掉** fold 和弦。

## Purpose

树开时：Ctrl/Alt+←→ 走包 `tui.tree.foldOrUp` / `unfoldOrDown`（fold 或分支跳转）；可见连接符 ⊞/⊟；不改 session 持久化。

## What Changes

1. 产品 `EditorSlot::Tree`：`handle_input` **MUST** 将 `ctrl+left|alt+left|ctrl+right|alt+right`（及包已绑定的 fold 键）转发给 `TreeSelector::handle_input`；**MUST NOT** 在 app 重写 fold 集合算法。
2. 挂载/过滤后连接符仍由包渲染（⊞ 折叠 / ⊟ 可折）。
3. harness：选中可折节点 → Ctrl+Left fold → 后代不可见且有 ⊞；Ctrl+Right unfold 恢复；不可折时分支跳转不崩。
4. **本 change 不做** Shift+L/T annotation 编辑（keybindings 标 future；可随后续或并入小 change）。

## Capabilities

- `app-tui-session-tree`（modify：fold 接线）
- `app-tui-input`（modify：树开 Ctrl/Alt+←→）

## Design SSOT

- [`design.md`](./design.md)
- [`session-tree.md`](../../../src/app/tui/design/session-tree.md) § Fold
- [`keybindings.md`](../../../src/app/tui/design/keybindings.md)
- 包默认键：`tui.tree.foldOrUp` = ctrl+left|alt+left；`unfoldOrDown` = ctrl+right|alt+right

## Impact

- `src/app/tui/layout/root.rs` 树输入转发（最小）；可能 harness / sample 树加深一级便于断言

## Out of scope

- Shift+F fork（c645）；Shift+L/T annotation 编辑；Settings 持久化 fold 状态

## Ethics

- risk_level: low
- prohibited_actions: 产品面重写包 fold 算法；把裸 ←→ 从翻页改成 fold
- required_evidence: harness fold 隐藏后代 + ⊞；unfold 恢复
- escalation_policy: 无

## Depends

- **c635**（已归档；filter 键与 fold 键不冲突）
