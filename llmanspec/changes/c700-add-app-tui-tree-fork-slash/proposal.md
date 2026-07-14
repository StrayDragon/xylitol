---
change_id: c700-add-app-tui-tree-fork-slash
title: "产品 slash：/tree 与 /fork（对齐 pi 入口）"
status: purpose-draft
priority: 700
depends_on: ["c645-update-app-tui-session-tree-fork"]
author: agent
track: A
---

# c700-add-app-tui-tree-fork-slash

## Why

pi 除双 Esc / Shift+F 外还有 `/tree`、`/fork`；xylitol 主路径已齐，slash 入口缺席，发现性与脚本化略弱。

## Purpose

idle：`/tree` 打开 MessageHistory 树（同双 Esc）；`/fork` 对当前 leaf/约定点走产品 fork（同 c645 语义）。补全列表含二者。

## What Changes（实现时）

1. SlashCommandSource 注册 `tree` / `fork`。
2. 行为委托既有 open-tree / fork effects；busy 拒绝策略写清。
3. harness：slash 开树 / fork；与双 Esc / Shift+F 不双写冲突。

## Capabilities（planned）

- `app-tui-input`（modify）

## Out of scope

- Settings doubleEscapeAction；改 fork 语义

## Ethics

- risk_level: low
- prohibited_actions: slash fork 写坏父 JSONL
- required_evidence: harness 与既有 fork 断言复用

## Depends

- **c645**（已归档）；MAY 在 **c705** 之后 promote（E2E 主路径不阻塞于 slash）
