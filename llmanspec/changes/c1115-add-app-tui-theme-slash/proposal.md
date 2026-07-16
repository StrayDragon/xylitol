---
change_id: c1115-add-app-tui-theme-slash
title: "产品 slash：/theme 切换主题"
status: purpose-draft
priority: 1115
depends_on: ["c1095-update-runtime-theme-hot-reload"]
author: agent
track: R
wave: slash-simple
domain: app-tui
---

# c1115-add-app-tui-theme-slash

## Why

包侧 Palette/`/theme` 在 demo 有验证；产品 TUI 无 `/theme`。依赖主题热重载缝（c1095）作为数据源。

## Purpose

产品 idle `/theme`：无参打开主题选择槽（或 cycle）；有参按名切换；应用当前 layout theme；busy 拒绝。

## What Changes（升格 full 时）

- slash + editor 槽 SelectList（或等价）
- 调用 c1095 应用主题
- harness
- delta：`app-tui-commands` · theme 相关

## Capabilities

- `app-tui-commands`（modify）

## Out of scope

- Settings 板
- 编辑 DESIGN tokens 运行时

## Ethics

- risk_level: low
- prohibited_actions: 静默回退 D19/包侧刻意差异
- required_evidence: 切换后 footer/scrollback 色可测
- escalation_policy: 槽形态 vs cycle 快捷键未定时拍板

## Depends

- `c1095-update-runtime-theme-hot-reload`
