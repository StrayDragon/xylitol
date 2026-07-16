---
change_id: c1140-add-package-tui-thinking-level-chrome
title: "xylitol-tui / agent_demo：thinking level 编辑器边框能力"
status: purpose-draft
priority: 1140
apply_band: P3-feature
depends_on: []
author: agent
track: R
wave: thinking
domain: package-tui
---

# c1140-add-package-tui-thinking-level-chrome

## Why

pi 用编辑器边框色表示 thinking level。产品不做 `/settings`。能力应先在 `packages/xylitol-tui` + `agent_demo` 验证，再进 app TUI（c1150）。

## Purpose

包层（或 demo 可注入 API）：Editor（或边框容器）按 thinking level 换边框/强调色；demo 可手动 cycle 验证。不绑定产品 slash。

## What Changes（升格 full 时）

- Editor theme / border token 按 level
- `agent_demo` 快捷键或命令验证
- 包测 / snapshot
- delta：`package-tui-editor` · `package-tui-theme` · `package-tui-agent-demo`

## Capabilities

- `package-tui-editor`（modify）
- `package-tui-theme`（modify）
- `package-tui-agent-demo`（modify）

## Out of scope

- `/settings`
- 产品 footer 接线（c1150）
- 模型配置枚举（c1145）

## Ethics

- risk_level: low
- prohibited_actions: 把产品壳逻辑塞进包
- required_evidence: demo 切换边框可测；包测绿
- escalation_policy: 边框 vs 仅 footer 回退策略由 c1150 消费

## Depends

- []

## Downstream

- `c1150-add-app-tui-thinking-level`
