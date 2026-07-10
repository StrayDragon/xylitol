---
change_id: c456-demo-session-nav-keys
title: "agent_demo：双 Esc 会话树 + 导航键位原型"
status: full
priority: 456
depends_on: ["c454-add-package-tui-tree-selector", "c455-add-package-tui-input-listener"]
author: agent
track: A
---

# c456-demo-session-nav-keys

## Why

产品后置会话树，但必须先在 demo 验证包能力与键位（对齐 pi doubleEscape→tree）。

## Purpose

`agent_demo` 实现双 Esc（空编辑器、<500ms）打开树选择器槽替换；验证 travel UX。

## What Changes

1. InputListener：双 Esc 检测（空 editor + 时间窗）。
2. editor 槽替换为 `TreeSelector`（假 `TreeNode` 数据）。
3. Enter 选择 / Esc 关闭；回写 `design/session-tree.md` / keybindings。

## Out of scope

- 持久化 session / 真实 fork（c491）
