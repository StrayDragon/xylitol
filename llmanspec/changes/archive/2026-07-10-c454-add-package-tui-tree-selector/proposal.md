---
change_id: c454-add-package-tui-tree-selector
title: "package-tui-tree-selector：会话树/列表选择器（对齐 pi TreeSelector）"
status: full
priority: 454
depends_on: []
author: agent
track: A
---

# c454-add-package-tui-tree-selector

> **优先路径第一步**（产品决策：用双 Esc 树替代 Codex 式 transcript 浏览；见 c470 paused / c491）。

## Why

双 Esc 会话树 / travel / fork 需要可导航树 UI；`SelectList` 不够表达 indent/connector/横向裁剪。pi `tree-selector.ts` 是参考实现。

## Purpose

在包内提供通用 Tree 选择器组件，供 `agent_demo` 与产品 `app-tui`（c491）槽替换使用；本变更含 **agent_demo 假数据双 Esc 冒烟**以验证可行（完整键位契约可与 c456 对齐）。

## What Changes

1. 新 capability `package-tui-tree-selector`。
2. `TreeNode` / `TreeSelector` / flatten + connectors + `include_node` 过滤 + `tui.select.*` 导航。
3. 主题闭包；ascii/unicode 连接符可注入；复制友好。
4. 单测：flatten、filter、connector、导航。
5. `agent_demo`：双 Esc（空 editor）打开假树槽；Enter/Esc 关闭。

## Out of scope

- 真实 session store 接线（c491）
- 完整 fork 业务
- pi 五档 FilterMode / label 编辑 / fold（用 `include_node` 钩子代替 FilterMode）
