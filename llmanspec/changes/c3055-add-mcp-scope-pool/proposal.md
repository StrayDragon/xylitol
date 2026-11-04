---
depends_on: [c3050-add-host-workspace-binding]
title: "MCP scope pool and connection lifecycle"
---

## Why

MCP hub 是 kernel 级单例；每次 SessionPrepare（Switch/Resume/Reload）都 `reset_join` 并 `shutdown` 全部连接。正在使用的 session 会被重连，stateful server 状态丢失；并发 session 互相干扰。

## What Changes

- 新增 `McpScope`：`Shared` / `Workspace` / `Session`。
- connection key = `(server config fingerprint, scope, credential)`；同 key 共用一个连接。
- 懒连接 + 引用计数；最后一个 claim 释放才关闭。
- session 对其 workspace set 内每个 workspace 建 claim；session 结束释放；1:N 时持有多个 claim。
- workspace set 的人类变更触发 claim 增删：新增 workspace 建 claim，移除 workspace 释放 claim；LLM 不得触发。
- Switch/Resume 不 reset；Reload 才 reset；连接失败按 server 重连。
- 默认 `Workspace`；`shared` opt-in；`session` 字段预留。
- `SessionPrepare` 携带 workspace set；prepare hub 按 scope 聚合 `view` / `cue`。
- tool adapter 用 `XyToolCtx.workspace` 路由；workspace 不在 session set 内则拒绝；session scope 需要 session id（后续）。
- stdio 设 `current_dir(workspace)`；Streamable HTTP 每 scope 一条 transport。

## Capabilities

- `infra-mcp`：scope/pool/lifecycle。
- `agent-runtime`：SessionPrepare workspace set 与 tool 路由。
- `server-core`：host 侧 scope 生命周期。

## Impact

MCP plugin、runtime prepare、serve host、TUI snapshot。

## Further Notes

- 依赖 `c3050-add-host-workspace-binding`。
- 后续：`c3060-add-mcp-tool-catalog`、`c3065-add-mcp-oauth-credentials`。
- 现有 `view()` 的 `try_lock` 兜底会返回 configured=0，是闪烁的直接来源；本 change 需保留上次已知快照，避免把“未知”当“未配置”。
- 1:N workspace set 下，pool 需要按 workspace 分别 claim/release；不能假设 session 只有一个 workspace。

## Open Questions

- session scope 现在实现还是先预留字段：推荐先预留。
- `XyToolCtx` 目前没有 session id；session scope 需补。
- 1:N set 下 tool 调用如何选择非 primary workspace：由 `XyToolCtx.workspace` 决定，还是显式 tool 参数。
