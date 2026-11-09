---
depends_on:
  - c3065-add-mcp-oauth-credentials
title: "Add MCP OAuth authorization flow"
branch: sdd/c3212-add-mcp-oauth-authorization
base_branch: main
base_sha: d9b1a23775b14dd5ac543106af13b3cb874f1604
---

## Why

c3065 已交付安全 token store（0600 原子写、Debug 脱敏）与 credential ID 进 connection key；但远程 MCP 需要 OAuth 时仍只能靠静态 headers。OAuth 2.1 授权流程、refresh / revoke / step-up、host callback 与 TUI 同意页均未实现。

## What Changes

- OAuth 2.1 + PKCE（S256）；只支持 public client，不使用 client_secret。
- Protected Resource Metadata（RFC 9728）与 AS metadata 发现；优先使用 401 challenge 的 resource_metadata。
- Client 注册：pre-registration / CIMD / DCR 三选一，按 server 配置与 AS 能力选择。
- Resource Indicators（RFC 8707）与 `iss` 校验（RFC 9207）；不提供显式 `resource` 覆盖，使用发现值或 MCP server URL。
- scope 最小化与 step-up。
- token 刷新、refresh rejected 重授权与 revoke；refresh 不 reset 连接。
- credential ID 进 connection key；缺省生成稳定默认账号 ID；同 server 多账号隔离。
- 插件持有 OAuth 状态，Host/进程内 gateway 发起授权并监听 localhost 一次性 callback；TUI 展示 URL / 同意 / 取消；远程 TUI 不支持。
- token 存储：OS keychain 优先，0600 文件 fallback；token 不进 config / secret.env / 日志 / 工具子进程 env。
- 日志、错误、导出、`/mcp` 面板脱敏；`/mcp` 显示 transport / scope / credential 是否存在 / 超时 / last error。

## Scope

- 不做 WebSocket；legacy HTTP+SSE 仍不支持。
- 不把 token 写进模型上下文。
- 不做 confidential client / client_secret。
- 不做远程 TUI callback；TUI 与 Host 须同机，或由用户自行建立 SSH 隧道。
- 不提供显式 `resource` 配置覆盖；以 RFC 9728 发现值或 MCP server URL 为 resource。
- 复用 c3065 的 credential key 与 token store 语义。

## Impact

- `xylitol-official-plugin-mcp`：OAuthState / AuthClient、credential store adapter、keychain backend。
- `xylitol-plugin-sdk`：McpOAuthGateway 端口与 OAuth 事件词表。
- `xylitol-serve` / `xylitol-kernel`：localhost callback listener 与 OAuth unary / event 接线。
- `xylitol-protocol` / `xylitol-wire`：OAuth 方法与事件映射。
- `xylitol-app-tui`：URL / 同意 / 取消 / 状态展示；`/mcp` 诊断字段。

## Further Notes

- 2026-11-07 从 delayed 提升为 active；c3065 已归档。
- 2026-11-07 决策：插件持有 OAuthState；Host/进程内 gateway 只提供 callback listener；远程 TUI 不支持；只支持 public client；不提供 `resource` 配置；revoke 与 step-up 本期实现。
- rmcp 3.8 已确认 `AuthClient<C>` 实现 `StreamableHttpClient`，可经 `StreamableHttpClientTransport::with_client` 接入。
- keychain 用 `keyring`；不可用时回退 0600 文件，不阻塞连接。
- callback 默认 127.0.0.1 临时端口；pre-registered client 可配置固定 redirect URI。
