---
change_id: c1010-add-app-tui-session-io-slash
title: "产品 slash：/session-compact /session-export /session-import（接线 dispatch）"
status: full
priority: 1010
depends_on: ["c1005-update-app-tui-session-slash-rename"]
author: agent
track: A
---

# c1010-add-app-tui-session-io-slash

## Why

`Compact` / `ExportHtml` / `ExportJsonl` / `ImportJsonl` 与 dispatch 已通；产品 TUI 未接线。对齐 pi 的 export/import/compact 主路径（新名）。

## Purpose

- `/session-compact`：无参 → `Command::Compact`；带参 MUST usage 错误；busy 拒绝。
- `/session-export` [path]：**默认 HTML**（对齐 pi）；路径以 `.jsonl` 结尾 → JSONL；否则 HTML；回报写入路径。
- `/session-import <path>`：缺 path → usage；有 path → **Yes/No 槽确认** → `ImportJsonl` → switch + 重建 transcript。
- 旧名 `/compact` `/export` `/import` 无效；执行经 dispatch。

## What Changes

1. PendingSlash + parse + SlashCommandSource。
2. effects：dispatch + 系统行；import 确认槽。
3. harness；delta `app-tui-commands`（+ 必要时 host）。

## Capabilities

- `app-tui-commands`

## Out of scope

- compact custom instructions（pi 有，协议无字段）
- `/session` 板、`/session-resume`（c1015）
- Batch C；剪贴板/reload/trust
- MissingSessionCwd 向导

## Ethics

- risk_level: medium
- prohibited_actions: 无确认静默 import；无 path 乱写主目录
- required_evidence: harness + dispatch 不回归

## Depends

- c1005

## pi 对照

见 `design.md`（默认 HTML、import 确认、compact 无参）。
