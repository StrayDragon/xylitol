---
change_id: c1135-add-app-tui-startup-header
title: "启动 header：已加载 skills / MCP（不做 prompt）"
status: purpose-draft
priority: 1135
apply_band: P3-feature
depends_on:
  - c1080-update-infra-mcp-client-product
  - c1085-update-agent-skills-runtime
author: agent
track: R
wave: editor-ux
domain: app-tui
---

# c1135-add-app-tui-startup-header

## Why

pi 启动 header 列出 context / skills / prompts / extensions / themes。xylitol：**明确不做 prompt 产品清单**；用 skills + MCP（+ 可选 themes）代替。

## Purpose（**暂缓 / deferred**）

本变更 **P3 暂缓**：不挡 c1080 / c1085 apply。目录级「已加载」可见性待运行时与 A10（调用时用户消息内 `$` 高亮、静默注入）稳定后再升格。

升格后目标（备忘，非本波）：启动及 `/reload` 后展示 skills + MCP 摘要（可折叠）；不做 prompt templates；与 A10 调用呈现正交。

## What Changes（升格 full 时）

- header / loaded-resources 区 widget
- 数据源：skills 目录 + MCP 已连接列表
- `/reload` 后刷新（订阅）
- delta：`app-tui-chrome` / `app-tui-host`

## Capabilities

- `app-tui-chrome`（modify）或 `app-tui-host`

## Out of scope

- prompt templates 行
- extensions / packages 行
- 本波实现（deferred）

## Ethics

- risk_level: low
- prohibited_actions: header 泄露完整密钥/env；用 header 替代 A10 注入验收
- required_evidence: 有/无 skills、有/无 MCP 快照（升格后）
- escalation_policy: 与 DESIGN 壳布局冲突时先更新 design/；与 A10 冲突时先改 PI_DELTAS

## Depends

- c1080 · c1085（运行时就绪后再议升格）

## Notes

- 2026-07-16：显式 deferred，优先 c1080+c1085 joint apply。
