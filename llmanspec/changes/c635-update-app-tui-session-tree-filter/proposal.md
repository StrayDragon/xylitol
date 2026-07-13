---
change_id: c635-update-app-tui-session-tree-filter
title: "产品会话树：filter / 搜索接线（demo→host）"
status: purpose-draft
priority: 635
depends_on: ["c625-update-app-tui-design-next-wave"]
author: agent
track: A
---

# c635-update-app-tui-session-tree-filter

## Why

c615 活树已通，但 filter / 增量搜索仍只在 demo。分支一多就难找——对齐 pi / demo 的 filter 是第一刀树 power。

## Purpose

树开时：增量搜索 + Ctrl+D/T/U/L/A（及 Ctrl+O 循环）过滤；状态行 `(i/n) [filter]`。

## What Changes（实现时）

1. 产品 `TreeSelector` 接线：`include_node` / kind 谓词（包能力已有）。
2. 键位按 [`keybindings.md`](../../../src/app/tui/design/keybindings.md)「树开」表。
3. harness：开树 → filter → 可见节点变化；Esc 清搜索再关树。

## Capabilities

- `app-tui-session-tree`（modify）
- `app-tui-input`（modify：树开键）

## Design SSOT

- [`session-tree.md`](../../../src/app/tui/design/session-tree.md) § Filter / 搜索
- playground `tree-power` · filter 态

## Impact

- `src/app/tui/layout/session_tree*` / host 键路由
- **不**改 Driver travel 语义（c615）

## Out of scope

- fold / fork（c640 / c645）；annotation 可后置

## Ethics

- risk_level: low
- prohibited_actions: 忙碌时开树；把 filter 做成 Codex transcript 面
- required_evidence: harness + 对照 playground filter 形状

## Depends

- **c625**；与 c630/c650 **无硬依赖**（可并行 promote/apply）
