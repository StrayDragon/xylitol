---
change_id: c491-add-app-tui-session-tree
title: "app-tui 双 Esc 会话树 travel/fork"
status: purpose-draft
priority: 491
depends_on: ["c460-add-app-tui-host", "c456-demo-session-nav-keys"]
author: agent
track: B
---

# c491-add-app-tui-session-tree

> **status: purpose-draft**（**优先路径**：替代原 Codex 式 transcript 浏览；demo 先经 c454/c456）

## Why

不做 Codex 式 TranscriptView。会话回看、分支跳转、fork 统一走 **双 Esc → 树选择器替换 editor 槽**（对齐 pi，非 overlay 跳转）。

## Purpose

产品面接线包 `TreeSelector`（c454）+ Driver / session API；双 Esc（空编辑器）打开；Enter travel、fork 对齐 pi；关闭还原 editor。

## What Changes（意向）

1. InputListener：双 Esc（空 editor + 时间窗），与 demo c456 同契约。
2. `UiRoot` editor 槽替换为树；假数据可先、真 session 随后。
3. travel / fork 经 `Driver`（禁止 reach `agent::session`）。
4. 回写 `design/session-tree.md` / `keybindings.md` 为已验证 MUST。

## Depends

- **c456**（demo 键位与槽替换已验证）硬依赖意向；升格前须 c454+c456 落地或并行 apply。
- **不再**依赖 c470 / c485 垂直切片。

## Out of scope

- Codex 式 transcript 浏览器
- 完整 compaction / bash UI
