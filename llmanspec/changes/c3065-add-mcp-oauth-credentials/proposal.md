---
depends_on: [c3055-add-mcp-scope-pool]
title: "MCP OAuth credentials and secure token store"
branch: main
base_branch: origin/main
base_sha: 78d077258664b34c5b9147336d76994cb6f448b7
---

## Why

远程 MCP 当前只支持静态 headers；没有 OAuth、token 刷新、多账号与安全存储。多凭据必须进 connection key，否则会串。

## What Changes

- OAuth 2.1 + PKCE；Protected Resource Metadata（RFC 9728）、AS metadata、Client ID Metadata Documents / pre-registration / DCR、Resource Indicators（RFC 8707）、`iss` 校验（RFC 9207）、scope 最小化 + step-up。
- token 存储：OS keychain 优先；fallback 0600 加密文件；token 不得进 config / secret.env / 日志 / 工具子进程 env。
- credential ID 进 connection key；同 server 多账号；refresh / revoke；按 credential 隔离。
- host 发起 OAuth；TUI 显示 URL / 同意；localhost callback；refresh 不 reset 连接。
- 日志、错误、导出、`/mcp` 面板脱敏。

## Capabilities

- `infra-mcp`：OAuth 与 credential key。
- `domain-security`：token 存储与脱敏。
- `app-tui-trust`：用户同意与 URL 展示。

## Impact

MCP plugin、host、TUI、secret store、connection pool。

## Further Notes

- 依赖 `c3055-add-mcp-scope-pool`。
- 规范：MCP authorization 2026-07-28。
- credential 状态由 `c3075-add-mcp-diagnostics` 显示；本 change 提供数据，`c3075` 依赖本 change。

## Open Questions

- keychain vs 0600 文件：推荐 keychain + fallback。
- 多账号 UI 与默认账号选择。
