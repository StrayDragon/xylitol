---
change_id: c1205-update-app-tui-reload-ux
title: "/reload 进行中交互体验（方案待议）"
status: purpose-draft
priority: 1205
apply_band: P9-deferred
depends_on:
  - c1120-add-app-tui-reload-slash
author: agent
track: R
wave: reload
domain: app-tui
---

# c1205-update-app-tui-reload-ux

## Why

c1120 已接线 `/reload`，但重载（尤其含 MCP）期间 UI 易像「卡住」：缺少进行中反馈与输入锁定策略，体验差。

## Purpose（**暂缓 / deferred · 最低优先级**）

本变更 **P9 暂缓**：功能正确性以 c1120 为准；本草案只覆盖**进行中交互**。升格前讨论视觉/锁定形态，不预设唯一 UI。

**问题空间**：

- 用户如何感知 reload 正在进行且未死锁
- 进行中是否禁止二次 `/reload`、普通提交、steer
- 失败/超时时如何解锁并说明

**候选方向（非合约）**：status/loader 条、禁用或变形 editor、系统块进度行、短超时+可取消——升格时钉 DESIGN。

## What Changes（升格 full 时）

- host 输入闸 + layout/chrome 反馈（按选定方案）
- harness：进行中可见状态；历史不变；结束后可再输入
- delta：`app-tui-host` / 相关 chrome capability

## Capabilities

- `app-tui-host`（modify）
- 视方案：`app-tui-chrome` / layout

## Out of scope

- 本波实现（deferred）
- 改变 c1120 重载步骤语义
- MCP 启动策略（见 c1200）

## Ethics

- risk_level: low
- prohibited_actions: 用动画掩盖永久挂起；锁死无法退出且无说明
- required_evidence: harness + 手测最短路径
- escalation_policy: UI 形态有歧义时先问用户再升格

## Depends

- c1120（已归档）

## Notes

- 2026-07-16：自 `c1205-update-app-tui-reload-progress-ux` 泛化重命名，避免过早锁定 spinner 单一解。
