---
change_id: c491-add-app-tui-session-tree
title: "app-tui 双 Esc 会话树 travel/fork"
status: full
priority: 491
depends_on: ["c460-add-app-tui-host"]
author: agent
track: B
---

# c491-add-app-tui-session-tree

> **优先路径**：替代 Codex 式 transcript 浏览；demo 已验证包能力（c454/c456/c467）。

## Why

不做 Codex 式 TranscriptView。会话回看、分支跳转、fork 统一走 **双 Esc → 树选择器替换 editor 槽**（对齐 pi，非 overlay）。

## Purpose

产品面 `UiRoot` 接线包 `TreeSelector`：双 Esc（空编辑器、<500ms）打开假树；Esc 关闭；Enter travel stub；槽替换贴底。真 session 图 / Driver travel 另批。

## What Changes

1. InputListener：双 Esc（空 editor + 时间窗）；树开时 Esc 关树。
2. `UiRoot` editor 槽替换为 `TreeSelector`（假 `TreeNode`）。
3. Enter → travel stub（transcript/system 行记录 id）并关树；不 reach `agent::session`。
4. Harness：双 Esc 开、Esc 关、Enter stub；回写 `design/session-tree.md`。

## Capabilities

- `app-tui-session-tree`（新建）
- `app-tui-input`（delta：双 Esc 开树）

## Out of scope

- 真 session 图 / `Driver` travel·fork seam（后续）
- Codex TranscriptView
- 水平平移（包侧另批）
- bridge / slash / steer（其它 change）
