---
change_id: c1080-update-infra-mcp-client-product
title: "MCP client 产品补齐：stdio / url·sse 传输与可观测装配"
status: draft
priority: 1080
apply_band: P2-system
depends_on: []
author: agent
track: R
wave: reload-foundations
domain: infra
ethics:
  risk_level: medium
  prohibited_actions:
    - 未配置也创建 MCP client
    - 重载时删除内置非 mcp 工具
    - 连接失败拖垮整次 bootstrap
  required_evidence:
    - stdio + url/sse 装配测（或等价 mock）
    - zero-cost 回归（无 mcp_servers → 无 client / 无 mcp: 工具）
    - 单测：失败 warn/诊断可观察且 bootstrap 成功
  escalation_policy: 第三种传输协议另开 change，不 silently 扩枚举
---

# c1080-update-infra-mcp-client-product

## Why

已有 `McpTransportKind::{Stdio,Sse}`、`connect_stdio` / `connect_sse`、zero-cost 与 reload（c515）。产品仍缺：配置校验体验、失败可观测、已连接列表只读缝（供日后 `/reload`；**c1135 header 暂缓**）。

## Purpose

1. 配置清晰：stdio（command/args/env）与 url/sse 字段校验错误可读。
2. 连接失败：warn + 诊断，不拖垮 bootstrap。
3. 只读查询已连接服务器（及工具摘要）供 app/core / 日后 UI；本变更 **不** 实现启动 header（c1135 deferred）。
4. 保持 mcp1–mcp3；补齐产品化缺口。

## What Changes

- `McpServerConfig` / 校验：缺字段、坏 url 的明确错误
- 装配：失败可观测；成功可枚举已连接 server id
- app/core（若需）：只读 snapshot 缝（非 header UI）
- delta：`infra-mcp`

## Capabilities

- `infra-mcp`（modify）

## Out of scope

- MCP 安全 allowlist UI / 工具弹窗平台
- 插件式自定义 transport SDK
- `/reload` slash UI（c1120）与启动 header（c1135 deferred）
- 与 c1085 的 skills 目录（并行独立）

## Impact

- 解锁 c1120 MCP 半边；header 仍等 c1135 升格

## Depends

- []

## Downstream

- `c1120-add-app-tui-reload-slash`
- `c1135-add-app-tui-startup-header`（deferred）

## Joint apply

- 可与 `c1085-update-agent-skills-runtime` 同波 apply（无硬 depends_on；共享 reload-foundations 波次）。
