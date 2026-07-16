---
change_id: c1105-add-app-tui-trust-slash
title: "产品 slash：/trust 保存项目信任决策"
status: purpose-draft
priority: 1105
apply_band: P3-feature
depends_on: []
author: agent
track: R
wave: slash-simple
domain: app-tui
---

# c1105-add-app-tui-trust-slash

## Why

pi `/trust` 可在交互内写入信任决策。xylitol Trust 在 CLI gate / ChoicePrompt 启动路径已有；产品 idle slash 表缺 `/trust`。

## Purpose

产品 TUI idle 解析 `/trust`：保存当前 cwd（及约定父级策略）的信任决策到 trust 存储；busy 拒绝；与启动 gate 语义一致。本会话是否立即重载项目资源：对齐 pi「写盘、重载需重启或 `/reload`」——升格时钉死。

## What Changes（升格 full 时）

- `commands.rs` + SlashCommandSource + effects → trust store
- harness / BDD
- delta：`app-tui-commands` · `app-tui-trust`

## Capabilities

- `app-tui-commands`（modify）
- `app-tui-trust`（modify）

## Out of scope

- 解冻 Plate/Settings/Choice 活板
- 工具 permission popup

## Ethics

- risk_level: medium
- prohibited_actions: 未确认就 always-trust 全局；绕过 Trust 闸静默加载项目扩展
- required_evidence: 写盘测；busy 拒绝；与 trust_gate 一致
- escalation_policy: 「是否立即 /reload」产品歧义时先问用户

## Depends

- []
