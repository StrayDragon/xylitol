---
change_id: c545-refactor-mcp-config-seam
title: "嵌入缝去掉 MCP 配置类型泄漏"
status: proposed
priority: 545
depends_on: []
author: agent
track: A3
---

# c545-refactor-mcp-config-seam

## Why

`BootstrappedRuntime.mcp_servers` / `McpSession::reload` 仍暴露 `infra::mcp::McpServerConfig`，与「embed 不依赖 infra 具体类型」冲突（c530 已知泄漏）。

## What Changes

1. 在应用缝（`embed` / `app::core`）引入文档化的 MCP 服务器描述类型（或对配置的 opaque 句柄），供 bootstrap → reload 传递。
2. `McpSession::reload` 与 `BootstrappedRuntime` 不再在公开字段上命名 infra 路径。
3. 更新 `library-and-clients.md` / `embed` 文档，去掉该泄漏条目。

## Capabilities

- `architecture`
- `layer-architecture`

## Impact

- `src/app/core/{bootstrap,composition}.rs`、`src/embed.rs`、可能薄包装在 `infra/mcp`

## Out of scope

- 改变 MCP 连接语义 / 传输种类
- 产品 TUI
