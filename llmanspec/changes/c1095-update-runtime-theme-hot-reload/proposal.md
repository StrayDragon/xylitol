---
change_id: c1095-update-runtime-theme-hot-reload
title: "Themes 热重载：磁盘主题 → 运行时 Palette/产品 theme"
status: purpose-draft
priority: 1095
apply_band: P2-system
depends_on: []
author: agent
track: R
wave: reload-foundations
domain: runtime
---

# c1095-update-runtime-theme-hot-reload

## Why

资源发现已能 list themes；包侧有 Palette；产品缺「热重载主题文件并通知 UI」与 `/theme`（c1115）的数据源。

## Purpose

主题资源可热重载：读磁盘 → 校验 → 应用到当前 TUI theme token；订阅通知；失败保留旧主题。

## What Changes（升格 full 时）

- theme 文件格式与 Trust 闸路径
- reload API + 诊断
- 产品面订阅刷新 layout theme
- delta：`runtime-resource-discovery` · `package-tui-theme` / `app-tui-*`

## Capabilities

- `runtime-resource-discovery`（modify）
- `package-tui-theme` 或 `app-tui` 相关（升格时定）

## Out of scope

- `/theme` slash UI（c1115）
- 运行时 Settings 板改主题（刻意不做）

## Ethics

- risk_level: low
- prohibited_actions: 重载清空会话；破坏 DESIGN token SSOT 双轨色板
- required_evidence: 坏主题保留旧值；成功后 UI 色变可测
- escalation_policy: 与 `DESIGN.md` / sync-tui-tokens 冲突时升级确认

## Depends

- []

## Downstream

- `c1115-add-app-tui-theme-slash`
- `c1120-add-app-tui-reload-slash`
