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

## Why

双 Esc 会话树 / travel / fork 需要可导航树 UI；`SelectList` 不够表达 indent/connector/横向裁剪。pi `tree-selector.ts` 是参考实现。

## Purpose

在包内提供通用 Tree 选择器组件，供 `agent_demo` 与日后 `app-tui` 槽替换使用。

## What Changes

1. 新 capability `package-tui-tree-selector`。
2. `TreeNode` / `TreeSelector` / flatten + connectors + filter + `tui.select.*` 导航。
3. 主题闭包；ascii/unicode 连接符可注入；复制友好。
4. 单测：flatten、filter、connector；`agent_demo` 假数据可后置到 c456。

## Out of scope

- 真实 session store 接线（c491）
- 完整 fork 业务
- pi 五档 FilterMode（用 `include_node` 钩子代替）
