---
change_id: c1150-add-app-tui-thinking-level
title: "产品 TUI：thinking level 边框 + footer 回退显示"
status: purpose-draft
priority: 1150
apply_band: P3-feature
depends_on:
  - c1140-add-package-tui-thinking-level-border
  - c1145-update-runtime-model-thinking-levels
author: agent
track: R
wave: thinking
domain: app-tui
---

# c1150-add-app-tui-thinking-level

## Why

包能力（c1140）+ 模型配置（c1145）就绪后，产品面接线。不做 `/settings`；可快捷键 cycle；**可回退**到 footer 旁模型区显示当前 level（high/xhigh/max…）。

## Purpose

产品 TUI：显示并切换 thinking level（来源 c1145）；优先编辑器边框（c1140）；边框未就绪或窄宽时 footer 显示 level；切换经 Driver/protocol，busy 策略升格钉死。

## What Changes（升格 full 时）

- host/footer/editor 接线
- 快捷键（可配置；不硬编码匹配）
- harness
- delta：`app-tui-chrome` · `app-tui-input` · `app-tui-host`

## Capabilities

- `app-tui-chrome`（modify）
- `app-tui-host`（modify）

## Out of scope

- `/settings`
- 改 PI_DELTAS 刻意差异以外行为

## Ethics

- risk_level: low
- prohibited_actions: 无配置模型上强行显示伪造 level
- required_evidence: 切换后 footer/边框一致；协议事件可测
- escalation_policy: 仅 footer 分期交付可接受，须在 tasks 标明

## Depends

- c1140 · c1145
