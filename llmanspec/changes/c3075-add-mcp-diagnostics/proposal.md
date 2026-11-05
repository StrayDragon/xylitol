---
depends_on: [c3055-add-mcp-scope-pool, c3065-add-mcp-oauth-credentials]
title: "MCP diagnostics and capability probe"
branch: main
base_branch: origin/main
base_sha: 78d077258664b34c5b9147336d76994cb6f448b7
---

## Why

当前 `/mcp` 只有连接数与短诊断；没有 scope/key/credential 视图；timeout 固定；legacy SSE 没有明确错误。

## What Changes

- `/mcp` 面板：scope、connection key、credential、连接状态、tool count、last error、重连次数。
- capability probe：`server/discover`（现代）/ legacy `initialize` + `tools/list`；按 config fingerprint 缓存；只用于诊断，不判状态性。
- 面板展示 legacy SSE 错误；拒绝与错误分类由 `c3085-add-mcp-transport-config` 负责。
- 面板展示 per-server timeout 配置；配置与默认值由 `c3085-add-mcp-transport-config` 负责。
- 日志与面板脱敏；错误分类。

## Capabilities

- `infra-mcp`：诊断与 probe。
- `app-tui-commands`：`/mcp` 面板。

## Impact

MCP plugin、host、TUI。

## Further Notes

- 依赖 `c3055-add-mcp-scope-pool`。
- 不做 statefulness 自动探测；probe 只取能力。
- 依赖 `c3065-add-mcp-oauth-credentials` 提供 credential 状态；本 change 只显示。

## Open Questions

- probe 时机：启动 / 首次使用 / 手动。
- `server/discover` 在 rmcp 3.8.0 的可用性：未验证，design 前查一次。
