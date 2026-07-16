---
change_id: c1080-update-infra-mcp-client-product
title: "MCP client 产品补齐：stdio / url·sse 传输与可观测装配"
status: purpose-draft
priority: 1080
depends_on: []
author: agent
track: R
wave: reload-foundations
domain: infra
---

# c1080-update-infra-mcp-client-product

## Why

代码已有 `McpTransportKind::{Stdio,Sse}` 与 `connect_stdio` / `connect_sse`（StreamableHttp），以及 zero-cost + reload 路径（c515）。产品侧仍缺：传输配置体验、失败可观测、与 startup header / `/reload` 的稳定契约，以及 url 类传输的边界说明（headers / 超时等可分阶段）。

## Purpose

把 MCP client 从「能连」推进到「可产品化」：stdio 与 url/sse（及后续等价 HTTP 传输）配置清晰、装配可测、失败可诊断；为 header（c1135）与 `/reload`（c1120）提供稳定枚举/状态缝。

## What Changes（升格 full 时）

- 配置 / `McpServerSpec`：stdio（command/args/env）与 url 类传输字段、校验错误信息
- 连接失败：warn + 诊断面（doctor / log），不拖垮整次 bootstrap
- 已连接服务器列表可供 UI/header 只读查询
- delta：`infra-mcp`（modify）

## Capabilities

- `infra-mcp`（modify）

## Out of scope

- MCP 安全 allowlist UI / 工具弹窗平台
- 插件式自定义 transport SDK
- 本 change 不实现 `/reload` / header（下游 draft）

## Ethics

- risk_level: medium
- prohibited_actions: 未配置也创建 client；重载时删除内置工具
- required_evidence: stdio + url 装配测；zero-cost 回归；失败不崩 bootstrap
- escalation_policy: 需支持第三种传输协议时另开 change，不 silently 扩枚举

## Depends

- []

## Downstream

- `c1120-add-app-tui-reload-slash`
- `c1135-add-app-tui-startup-header`
