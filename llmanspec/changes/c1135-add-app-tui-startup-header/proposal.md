---
change_id: c1135-add-app-tui-startup-header
title: "启动 header：已加载 skills / MCP（不做 prompt）"
status: purpose-draft
priority: 1135
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

pi 启动 header 列出 context / skills / prompts / extensions / themes。xylitol：**明确不做 prompt 产品清单**；用 skills + MCP（+ 可选 themes）代替。依赖 skills/MCP 运行时就绪。

## Purpose

产品 TUI 启动（及 `/reload` 后）展示已加载 skills 与 MCP 服务器/工具摘要（可折叠）；不列出 prompt templates；视觉形态可与 c1085 视觉 TBD 统筹。

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

## Ethics

- risk_level: low
- prohibited_actions: header 泄露完整密钥/env
- required_evidence: 有/无 skills、有/无 MCP 快照
- escalation_policy: 与 DESIGN 壳布局冲突时先更新 design/

## Depends

- c1080 · c1085
