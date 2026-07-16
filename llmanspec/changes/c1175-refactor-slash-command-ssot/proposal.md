---
change_id: c1175-refactor-slash-command-ssot
title: "重构：收敛 slash 命令 SSOT（死短名表 vs session-*）"
status: purpose-draft
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

单一 SSOT：产品对外名以 `app-tui-commands` 为准；agent 表要么删除、要么生成自同一清单；Server GetCommands 与 TUI 补全一致；不回退 A03。

## What Changes（升格 full 时）

- 删或改写 `BUILTIN_COMMANDS`；接线 GetCommands
- 更新过时注释（stdio rpc / c355）
- delta：`cli-entry` · `app-tui-commands` · `agent-prompt`

## Out of scope

- 实现 `/reload` 等新命令（另有 draft）
- 恢复 pi 短名作为产品入口

## Ethics

- risk_level: medium
- prohibited_actions: 静默让产品重新接受 /tree 等旧名（违反 A03）
- required_evidence: GetCommands 与 TUI SlashCommandSource 一致测
- escalation_policy: 若嵌入方依赖旧短名需用户确认

## Depends

- []
