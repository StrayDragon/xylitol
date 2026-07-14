---
change_id: c635-update-app-tui-session-tree-filter
title: "产品会话树：filter / 搜索接线（对齐 pi）"
status: full
priority: 635
depends_on: ["c625-update-app-tui-design-next-wave"]
author: agent
track: A
---

# c635-update-app-tui-session-tree-filter

## Why

c615 活树已通，但 filter / 增量搜索仍只在 demo。分支一多就难找——对齐 pi / demo 的 filter 是树 power 第一刀，并解锁 c640 / c645。

## Purpose

树开时：增量搜索 + Ctrl+D/T/U/L/A（及 Ctrl+O 循环）过滤；状态行 `(i/n)` + 非 default 时 `[filter]`；谓词与键语义对齐 pi（见 design）。

## What Changes

1. 产品 `TreeSelector`：`include_node` + `status_suffix` 接线五种 `FilterMode`（default / no-tools / user-only / labeled-only / all）。
2. 键位：Ctrl+D → default；Ctrl+T/U/L/A → **toggle** 该模式 ↔ default（对齐 pi，非 demo 的纯 set）；Ctrl+O → cycle forward。树开时这些键优先于 thinking / 工具视口。
3. **default** 藏 bookkeeping（对齐 pi）：mapper 给 meta 类 entry `kind=meta`，default 谓词排除之；**all** 全显。
4. mapper：`SessionTreeNode.label` → `TreeNode.annotation`（labeled-only 与展示需要）。
5. 增量搜索：沿用包 `TreeSelector`（与 `include_node` AND）；Esc 先清搜索再关树（既有）。
6. harness：开树 → filter → 可见节点变化；toggle / cycle；Esc 清搜索；labeled 依赖 annotation 映射。

## Capabilities

- `app-tui-session-tree`（modify：filter + mapper）
- `app-tui-input`（modify：树开键）

## Design SSOT

- [`session-tree.md`](../../../src/app/tui/design/session-tree.md) § Filter
- [`session-tree-vs-pi.md`](../../../src/app/tui/design/session-tree-vs-pi.md)
- [`keybindings.md`](../../../src/app/tui/design/keybindings.md)「树开」
- 本 change [`design.md`](./design.md)

## Impact

- `src/app/tui/layout/{root,session_tree}` / host 键路由；**不**改 Driver travel 语义
- **不**改包内硬编码 FilterMode（仍用 `include_node`）

## Out of scope

- fold / fork / Shift+L annotation 编辑（c640 / c645）
- Shift+Ctrl+O cycle backward（MAY 后续）
- Settings 持久化 `treeFilterMode`（pi 有；本仓 Settings 槽未开）

## Ethics

- risk_level: low
- prohibited_actions: 忙碌时开树；把 filter 做成 Codex transcript 面；在包内硬编码产品 FilterMode
- required_evidence: harness 覆盖谓词 / toggle / 搜索清；对照 playground filter 形状
- escalation_policy: 无

## Depends

- **c625**；与 c630/c650 **无硬依赖**
