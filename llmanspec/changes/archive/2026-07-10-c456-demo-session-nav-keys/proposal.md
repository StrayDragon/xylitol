---
change_id: c456-demo-session-nav-keys
title: "agent_demo 会话树：搜索 + filter + ←→ 翻页"
status: full
priority: 456
depends_on: ["c454-add-package-tui-tree-selector", "c455-add-package-tui-input-listener"]
author: agent
track: A
---

# c456-demo-session-nav-keys

> **优先路径第二步**（双 Esc 树）。c454 已冒烟槽替换；本变更补齐相对 pi 的操作差距第一批。

## Why

会话树是产品历史/分支主 UX。c454 只有 ↑↓/Enter/Esc；pi 还有增量搜索、五档 filter、←→ 翻页与状态 `[filter]`。缺这些则 demo 无法验证真实 travel 工作流。

## Purpose

在包 `TreeSelector` 增加通用搜索与 ←→ 翻页；在 `agent_demo` 用假数据验证五档 filter（`include_node` 谓词，不硬编码进包）与树开时 Ctrl+O 循环 filter；回写 `design/session-tree.md` / keybindings / vs-pi 差距表。

## What Changes

1. **包**：`TreeSelector` 增量搜索（打字 / Backspace；Esc 先清搜索）；←→ 等同 pageUp/pageDown；状态行可附 `status_suffix`（如 `[no-tools]`）。
2. **Demo**：假树标签可过滤（user/tool/assistant/labeled）；Ctrl+D/T/U/L/A 切 filter；树开时 Ctrl+O 循环 filter（关树时仍为工具视口 Ctrl+O）。
3. **文档**：`session-tree.md` / `keybindings.md` / `session-tree-vs-pi.md` 对齐已验证项。

## Out of scope

- fold / branch jump / label / 水平平移（后续）
- 持久化 session / 真实 fork（c491）
- 产品 `UiRoot` 接线（c491）
