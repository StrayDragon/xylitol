---
change_id: c454-add-package-tui-tree-selector
title: "package-tui-tree-selector：会话树/列表选择器（对齐 pi TreeSelector）"
status: purpose-draft
priority: 454
depends_on: []
author: agent
track: A
---

# c454-add-package-tui-tree-selector

> **status: purpose-draft**

## Why

双 Esc 会话树 / travel / fork 需要可导航树 UI；`SelectList` 不够表达 indent/connector/横向裁剪。pi `tree-selector.ts` 是参考实现。

## Purpose

在包内提供通用 Tree/Session 选择器组件（或 SelectList 树模式扩展），供 `agent_demo` 与日后 `app-tui` 槽替换使用。

## What Changes（意向）

1. 新 `package-tui-tree-selector`（或扩展 `package-tui-engine`——升格时二选一，优先独立 capability）。
2. 扁平化树导航、缩进连接符、选中行、过滤；复制友好（可 ascii 档）。
3. `agent_demo`：双 Esc 打开树（假数据），Enter 选择 / Esc 关闭。

## Out of scope

- 真实 session store 接线（c491）
- 完整 fork 业务
