---
depends_on:
  - c3065-add-mcp-oauth-credentials
title: "Add MCP OAuth authorization flow"
---

## Why

c3065 只交付了安全 token store（0600 原子写、Debug 脱敏）与 credential ID 进 connection key；OAuth 2.1 授权流程、refresh / revoke、TUI 同意页与 keychain 均未实现。远程 MCP 若需要 OAuth，目前只能靠静态 headers。

## What Changes

- OAuth 2.1 + PKCE（S256）。
- Protected Resource Metadata（RFC 9728）与 AS metadata 发现。
- Client 注册：pre-registration / Client ID Metadata Documents / DCR 三选一，按 server 配置。
- Resource Indicators（RFC 8707）与 `iss` 校验（RFC 9207）。
- scope 最小化与 step-up。
- token 刷新与 revoke；refresh 不 reset 连接。
- credential ID 进 connection key；同 server 多账号隔离。
- host 发起授权；TUI 展示 URL / 同意；localhost callback。
- token 存储：OS keychain 优先，0600 文件 fallback；token 不进 config / secret.env / 日志 / 工具子进程 env。
- 日志、错误、导出、`/mcp` 面板脱敏。

## Scope

- 不做 WebSocket；legacy HTTP+SSE 仍不支持。
- 不把 token 写进模型上下文。
- 复用 c3065 的 credential key 与 token store 语义。

## 延后原因

- c3065 先交付 store 与 credential key，保证连接隔离可用；OAuth 流程跨 runtime / serve / TUI，且 keychain、同意页、callback 都需要新交互。
- 单 change 过大；主线优先让 MCP 连接、catalog、诊断可用。
- keychain 与 0600 fallback 的取舍、TUI 同意形态尚未定。

## 启动前置条件

- c3065 已归档（当前已满足）。
- 确定 keychain 优先 / 0600 fallback 策略。
- TUI 有 URL 展示与同意/拒绝槽位（可复用 Choice）。
- 有可重复的 OAuth 测试 server / fixture。
- 验证 rmcp 3.8 的 `AuthorizationManager` / `AuthClient` 与 Streamable HTTP transport 的集成方式。
- 确定 RFC 8707 / 9207 校验强度与失败提示。

## Related

- c3065-add-mcp-oauth-credentials（已归档）
- c3075-add-mcp-diagnostics（credential 状态显示）
- c3095-add-mcp-custom-transports（transport seam）

## Open Questions

- 多账号默认账号选择与 UI。
- callback 端口固定还是动态。
- refresh 失败时的重授权提示与重试策略。
- token 过期与 server 401 的交互。
