---
change_id: c515-add-mcp-config-reload
title: "MCP 配置驱动装配与重载"
status: proposed
priority: 515
depends_on: []
author: agent
track: A
---

# c515-add-mcp-config-reload

## Why

个人开箱需要 MCP **可配即用**，但未配置时不得背负成本。现有 `infra/mcp` 未进 composition；需配置化装配 + 动态重载，且经 `XyTool` 接入、rmcp 不泄漏。

## What Changes

1. 无 / 空 `mcp_servers` → 不创建 client、不注册 `mcp:` 工具（zero-cost）。
2. 有配置 → 装配并将工具以 `mcp:{server}:{name}` 注册为 `XyTool`。
3. 支持进程内按新配置**重载**工具集（分阶段可接受：先启动装配，再热更新）。
4. 新增 capability `infra-mcp`；修订 `runtime-config` / `agent-tools`。

## Capabilities

- `infra-mcp`（new）
- `runtime-config`（add rc9）
- `agent-tools`（add r36）

## Impact

- `src/infra/mcp/*`、`src/infra/config/types.rs`、`src/app/core/{bootstrap,composition}.rs`、工具集热更新缝（Driver 或 Agent API，设计见 tasks）

## Out of scope

- MCP 安全 allowlist 产品 UI（可复用 domain-security 已有意向，本变更不实现弹窗）
- 产品 TUI
