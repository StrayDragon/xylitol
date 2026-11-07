---
depends_on:
  - c3065-add-mcp-oauth-credentials
title: "Add MCP OAuth authorization flow"
branch: main
base_branch: origin/main
base_sha: 4e422320e5d2e5648f48d06ca1321f78d3c569d9
---

## Why

c3065 已交付安全 token store（0600 原子写、Debug 脱敏）与 credential ID 进 connection key；但远程 MCP 需要 OAuth 时仍只能靠静态 headers。OAuth 2.1 授权流程、refresh / revoke、host callback 与 TUI 同意页均未实现。

## What Changes

- OAuth 2.1 + PKCE（S256）。
- Protected Resource Metadata（RFC 9728）与 AS metadata 发现。
- Client 注册：pre-registration / Client ID Metadata Documents / DCR 三选一，按 server 配置。
- Resource Indicators（RFC 8707）与 `iss` 校验（RFC 9207）。
- scope 最小化与 step-up。
- token 刷新与 revoke；refresh 不 reset 连接。
- credential ID 进 connection key；同 server 多账号隔离。
- host 发起授权并监听 localhost callback；TUI 展示 URL / 同意。
- token 存储：OS keychain 优先，0600 文件 fallback；token 不进 config / secret.env / 日志 / 工具子进程 env。
- 日志、错误、导出、`/mcp` 面板脱敏。

## Scope

- 不做 WebSocket；legacy HTTP+SSE 仍不支持。
- 不把 token 写进模型上下文。
- 复用 c3065 的 credential key 与 token store 语义。

## Impact

- `xylitol-official-plugin-mcp`：AuthorizationManager、AuthClient、credential store adapter。
- `xylitol-serve`：host callback listener 与 OAuth unary / event。
- `xylitol-wire` / `xylitol-protocol`：OAuth 方法与事件映射。
- `xylitol-app-tui`：URL / 同意 / 状态展示。

## Further Notes

- 2026-11-07 从 delayed 提升为 active；c3065 已归档。
- rmcp 3.8 已确认 `AuthClient<C>` 实现 `StreamableHttpClient`，可经 `StreamableHttpClientTransport::with_client` 接入。
- keychain 用 `keyring`；不可用时回退 0600 文件，不阻塞连接。
- callback 默认 127.0.0.1 临时端口；pre-registered client 可配置固定 redirect URI。
