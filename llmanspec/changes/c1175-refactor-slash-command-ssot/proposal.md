---
change_id: c1175-refactor-slash-command-ssot
title: "重构：收敛 slash 命令 SSOT（死短名表 vs session-*）"
status: full
priority: 1175
apply_band: P1-refactor
depends_on: []
author: agent
track: QA
wave: smell
domain: app-tui
---

# c1175-refactor-slash-command-ssot

## Why

`agent/prompt/commands.rs` 仍挂 pi 短名 22 条且 `dead_code`；产品 TUI 为 `session-*`（A03）。GetCommands / 文档 / 心智三套故事，specs compact 已承认 MAY 保留，但长期债。

## Purpose

单一 SSOT：产品对外名以 `app-tui-commands` 为准；agent 内建表与 TUI `SlashCommandSource` / `parse_slash_command` **同源**；Server `GetCommands` 与 TUI 补全一致；**不**回退 A03（旧短名 `/tree` `/fork` 等 MUST NOT 作为产品入口）。

## What Changes

- 引入产品 builtin 清单 SSOT（agent 可见模块；app 可复用；**禁止** app→agent 反向依赖倒置以外的 reach）
- 改写/删除 `BUILTIN_COMMANDS` 死短名表；去掉过时 c355/stdio rpc 注释
- `layout/slash_catalog` 与 `commands::parse_slash_command` 消费同一名称集（补全 ⊆ 可解析，或显式标注未实现项不进补全）
- `Driver::get_commands` / Server GetCommands 返回产品名
- 更新过时 specs：`cli-entry` sc3、`agent-session` a23；必要时强化 `app-tui-commands` atm7

## Capabilities

- `cli-entry`（modify sc3）
- `agent-session`（modify a23）
- `app-tui-commands`（modify atm7 / 增 SSOT 要求）

## Out of scope

- 实现 `/reload` `/trust` `/theme` 等新命令（另有 draft）
- 恢复 pi 短名作为产品入口
- 合并 Host/UiRoot

## Ethics

- risk_level: medium
- prohibited_actions: 静默让产品重新接受 `/tree` `/fork` `/export` 等旧名（违反 A03）
- required_evidence: GetCommands 名称集与 TUI SlashCommandSource 一致测；旧短名仍 unknown
- escalation_policy: 若嵌入方依赖旧短名 GetCommands 列表需用户确认（默认不保留）

## Depends

- []（c1170 已归档，非硬依赖）
