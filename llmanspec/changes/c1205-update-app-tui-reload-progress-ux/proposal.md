---
change_id: c1205-update-app-tui-reload-progress-ux
title: "/reload 进行中：锁定输入 + 进度/spinner 视觉反馈"
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

# c1205-update-app-tui-reload-progress-ux

## Why

c1120 已接线 `/reload`，但重载（尤其 MCP 重连）期间 UI 易「卡住」：无进行中反馈、输入区仍像可编辑，体验差。

## Purpose（**暂缓 / deferred · 最低优先级**）

本变更 **P9 暂缓**：不挡其它产品 slash。功能正确性以 c1120 为准；本草案只补 **进行中 UX**。

升格后目标（备忘，方案待钉）：

1. `/reload` 开始 → 结束期间：**明确锁定**（拒绝二次 reload / 普通提交，或排队策略写死一种）。
2. **视觉动态**：Loader/spinner，或把 editor 槽重绘为「Reloading…」禁用态（二选一或组合；升格时钉 DESIGN）。
3. 完成后恢复 idle 输入，并保留现有系统块分步报告。

## What Changes（升格 full 时）

- host：`reload_in_progress`（或等价）闸输入
- layout/chrome：spinner / 禁用 editor 形态
- effects：async reload 前后置位；失败仍解锁
- delta：`app-tui-host` / `app-tui-chrome`（或 `app-tui-commands` 补充 busy-reload 场景）
- harness：进行中有可见状态 + 不丢历史

## Capabilities

- `app-tui-host`（modify）
- `app-tui-chrome`（modify，若动壳层）

## Out of scope

- 本波实现（deferred）
- 改变 c1120 重载步骤语义 / 历史不变合约
- MCP 懒加载（见 c1200）

## Ethics

- risk_level: low
- prohibited_actions: 用「看起来在转」掩盖永久挂起无超时；锁死后无法 Esc/退出而无说明
- required_evidence: harness 断言进行中 UI + 结束后可再输入；历史条数不变
- escalation_policy: spinner vs 改 editor 形态有产品歧义时先问用户再升格 full

## Depends

- c1120（`/reload` 行为已落地）

## Notes

- 2026-07-16：目的草案；priority 1205 = 紧随 c1200 的延后 UX 项。候选视觉：Loader 条 / 禁用 editor / 两者兼有。
