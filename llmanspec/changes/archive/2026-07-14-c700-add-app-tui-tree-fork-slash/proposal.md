---
change_id: c700-add-app-tui-tree-fork-slash
title: "产品 slash：/tree 与 /fork（对齐 pi 入口）"
status: full
priority: 700
depends_on: ["c645-update-app-tui-session-tree-fork"]
author: agent
track: A
---

# c700-add-app-tui-tree-fork-slash

## Why

pi 除双 Esc / Shift+F 外还有 `/tree`、`/fork`；xylitol 主路径已齐，slash 入口缺席，发现性与脚本化略弱。

## Purpose

idle：`/tree` 打开 MessageHistory 树（同双 Esc）；`/fork` 对**当前 leaf** 走产品 fork（同 c645 Before/At 语义）。SlashCommandSource 列出二者。busy 时硬拒绝并提示。

## What Changes

1. `PendingSlash::OpenTree` / `ForkAtLeaf`；parse `/tree` `/fork`。
2. effects：OpenTree → 既有 `session_tree` mount；ForkAtLeaf → 解析 leaf → 既有 fork+switch。
3. SlashCommandSource 注册；harness。
4. 文档：与 pi 差异 — xylitol `/fork` **不**开 user 选择器（选节点仍用树 + Shift+F）。

## Capabilities

- `app-tui-commands`（atm6）
- `app-tui-input`（ati28）

## Out of scope

- Settings doubleEscapeAction；`/clone`；改 fork 语义；user-message selector（pi 完整 `/fork` UX）

## Ethics

- risk_level: low
- prohibited_actions: slash fork 写坏父 JSONL
- required_evidence: harness

## Depends

- **c645**（已归档）
