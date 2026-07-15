---
change_id: c1020-add-app-tui-session-lifecycle-slash
title: "产品 slash：/session-new /session-clone /session-name（生命周期）"
status: full
priority: 1020
depends_on: ["c1015-add-app-tui-session-panel-slash"]
author: agent
track: A
---

# c1020-add-app-tui-session-lifecycle-slash

## Why

pi 提供 `/new`（空会话）、`/clone`（leaf 上 fork `at`）、`/name`（显示/设置会话名）。xylitol 已落地 session-tree/fork/io/resume；缺生命周期三条，且命名须遵守 A03 `session-*` 前缀。

## Purpose

- `/session-new`（仅无参）：经 Driver `new_session`（或等价）创建空会话并 switch，清 transcript；busy 拒绝。
- `/session-clone`（仅无参）：对当前 leaf 以 `ForkPosition::At` fork + switch（对齐 pi `/clone`；**不同于** `/session-fork` 的 Before/At 规则）；无 leaf 时提示；busy 拒绝。
- `/session-name`：无参显示当前名（无则 usage）；有参经 Driver `set_session_name` 写入（换行→空格 trim，对齐 pi sanitize）；若规范化后与输入不同 MUST 提示；busy 拒绝。
- 旧名 `/new` `/clone` `/name` MUST NOT 被识别；SlashCommandSource 列出三者。
- TUI MUST NOT reach `infra::session`。

## What Changes

1. Driver seam：`new_session`、`get_session_name`、`set_session_name`（clone 复用既有 `fork_session(..., At)`）。
2. `PendingSlash` + parse + effects + SlashCommandSource。
3. harness；delta `app-tui-commands` / `app-tui-host`；`PI_DELTAS` 记 clone≠fork（A07）。

## Capabilities

- `app-tui-commands`
- `app-tui-host`

## Out of scope

- 列表内 rename；protocol/REST 对称（可仅本地 Driver）；pi extension cancel 钩子
- 改 `/session-fork` 语义

## Ethics

- risk_level: medium
- prohibited_actions: TUI 直读 sessions 目录；把 clone 做成 pi user 选择器；解冻 Trust Choice stub
- required_evidence: harness + arch_guard

## Depends

- c1015（已归档）

## pi 对照

见 `design.md`。
