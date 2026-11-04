---
depends_on: [c3055-add-mcp-scope-pool]
title: "MCP diagnostics and capability probe"
---

## Why

当前 `/mcp` 只有连接数与短诊断；没有 scope/key/credential 视图；timeout 固定；legacy SSE 没有明确错误。

## What Changes

- `/mcp` 面板：scope、connection key、credential、连接状态、tool count、last error、重连次数。
- capability probe：`server/discover`（现代）/ legacy `initialize` + `tools/list`；按 config fingerprint 缓存；只用于诊断，不判状态性。
- legacy SSE 明确报错：提示使用 Streamable HTTP。
- per-server timeout 配置：connect / call；默认保持 8s / 120s。
- 日志与面板脱敏；错误分类。

## Capabilities

- `infra-mcp`：诊断与 probe。
- `app-tui-commands`：`/mcp` 面板。
- `cli-*`：配置校验。

## Impact

MCP plugin、host、TUI、config。

## Further Notes

- 依赖 `c3055-add-mcp-scope-pool`。
- 不做 statefulness 自动探测；probe 只取能力。
- 后续：`c3065-add-mcp-oauth-credentials` 显示 credential 状态。

## Open Questions

- probe 时机：启动 / 首次使用 / 手动。
