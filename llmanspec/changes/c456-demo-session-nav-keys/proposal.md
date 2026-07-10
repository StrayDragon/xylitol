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

> **优先路径第二步**（双 Esc 树）。c454 已含假树冒烟；本变更收紧键位契约与文档回写。

## Why

会话树是产品历史/分支主 UX；c454 已在 `agent_demo` 验证双 Esc 槽替换可行，本变更补齐时间窗/文档与导航细节对齐。

## Purpose

`agent_demo` 双 Esc（空编辑器、<500ms）打开树选择器槽替换；验证 travel UX；回写 `design/session-tree.md` / keybindings。

## What Changes

1. InputListener：双 Esc 检测（空 editor + 时间窗）。
2. editor 槽替换为 `TreeSelector`（假 `TreeNode` 数据）。
3. Enter 选择 / Esc 关闭；回写 `design/session-tree.md` / keybindings。

## Out of scope

- 持久化 session / 真实 fork（c491）
