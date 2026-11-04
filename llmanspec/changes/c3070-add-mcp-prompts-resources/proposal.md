---
depends_on: [c3060-add-mcp-tool-catalog]
title: "MCP prompts and resources"
---

## Why

当前只接 tools/list + tools/call；prompts 与 resources 是 MCP server 的一等能力，用户与 agent 都需要。

## What Changes

- prompts/list + prompts/get；TUI 以 slash 命令暴露（用户已定）。
- resources/list + resources/read + resources/templates/list；订阅 `subscriptions/listen`。
- resources 作为 context：`@` 选择器 / 上下文附件；读取需权限。
- per-scope catalog；prompts/resources 按 scope 隔离。
- 权限/trust：读文件、外部资源走 trust 门禁。
- TUI 固定区：slash 命令、资源选择器、错误提示。

## Capabilities

- `infra-mcp`：prompts/resources。
- `app-tui-commands`：slash 命令。
- `app-tui-fixed-zone`：资源选择器 / 提示。

## Impact

MCP plugin、runtime context、TUI。

## Further Notes

- 依赖 `c3060-add-mcp-tool-catalog`。
- prompts 是 user-controlled；resources 是 application-controlled。
- 后续：`c3075-add-mcp-diagnostics`。

## Open Questions

- resources 的 `@` 选择器交互与缓存策略。
