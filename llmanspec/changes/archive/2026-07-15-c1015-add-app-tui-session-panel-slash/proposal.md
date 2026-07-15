---
change_id: c1015-add-app-tui-session-panel-slash
title: "产品 slash：/session 信息转储 + /session-resume 列表切换"
status: full
priority: 1015
depends_on: ["c1005-update-app-tui-session-slash-rename"]
author: agent
track: A
---

# c1015-add-app-tui-session-panel-slash

## Why

pi `/session` 展示 info/stats；`/resume` 开会话列表切换。dispatch 已有 stats/switch；缺 TUI 与 list seam。

## Purpose

- `/session`（仅无参）：`GetSessionStats`（+ 必要 state）→ scrollback/系统文本块（**非**操作菜单；对齐 pi）。
- `/session-resume`（仅无参）：editor 槽 SelectList（mtime 降序）→ `SwitchSession` → 重建 transcript；Esc 取消。
- 旧名 `/resume` 无效；busy 拒绝开板/列表。
- 列表经 **Driver seam**（新增 `list_sessions` 或等价），禁止 infra reach。

## What Changes

1. Driver（± protocol 论证）list API。
2. PendingSlash + parse + 槽 UI + effects。
3. harness；delta `app-tui-commands` / `app-tui-host`。

## Capabilities

- `app-tui-commands`
- `app-tui-host`

## Out of scope

- `/session <subcommand>`；`/session-resume <id>` 直参（future）
- 列表内 rename；Batch C；操作菜单版 `/session`
- c1010 IO 命令实现

## Open（已钉死，见 design.md）

1. 无子命令 — 仅 dump
2. mtime 降序；name + id
3. switch 后重建 transcript并清槽

## Ethics

- risk_level: medium
- prohibited_actions: TUI 直读 sessions 目录；解冻 Trust Choice stub
- required_evidence: harness + arch_guard

## Depends

- c1005（与 c1010 无硬依赖）

## pi 对照

见 `design.md`（`/session`=dump，`/resume`=列表无参）。
