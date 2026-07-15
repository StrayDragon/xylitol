---
change_id: c1005-update-app-tui-session-slash-rename
title: "产品 slash 重命名：/tree→/session-tree，/fork→/session-fork"
status: full
priority: 1005
depends_on: []
author: agent
track: A
---

# c1005-update-app-tui-session-slash-rename

## Why

选中迁移的 session 族命令统一 `session-*` 前缀；c700 入口仍叫 `/tree`、`/fork`。产品决议：只服务新名。

## Purpose

- `/session-tree` MUST 打开 MessageHistory 树（同双 Esc；语义不变）。
- `/session-fork` MUST 对当前 leaf 产品 fork+switch（同 Shift+F / A02；**非** pi user 选择器）。
- `/tree`、`/fork` MUST NOT 再被识别。
- `SlashCommandSource` MUST 列新名、MUST NOT 列旧名；busy 仍硬拒绝。

## What Changes

1. parse + PendingSlash 文案；SlashCommandSource；unknown 提示。
2. harness / BDD / design / keybindings / PI_DELTAS 手测备忘中的产品 slash 名。
3. delta：`app-tui-commands` atm6、`app-tui-input` ati28。

## Capabilities

- `app-tui-commands`
- `app-tui-input`

## Out of scope

- 改 fork/树语义；接受旧名别名；Batch A/B。

## Ethics

- risk_level: low
- prohibited_actions: 借 rename 改 A02；静默保留旧名
- required_evidence: harness 新名绿、旧名 unknown

## Depends

- 无（c700 已归档）
- blocks: c1010、c1015

## pi 对照

见 `design.md`（`/tree`/`/fork` 行为与刻意差异 A01/A02）。
