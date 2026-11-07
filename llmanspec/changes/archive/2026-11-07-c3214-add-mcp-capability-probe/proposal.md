---
depends_on:
  - c3075-add-mcp-diagnostics
title: "Add MCP capability probe and reconnect count"
branch: main
base_branch: origin/main
base_sha: 4e422320e5d2e5648f48d06ca1321f78d3c569d9
---

## Why

c3075 已交付每 server 诊断（transport / scope / credential presence / counts / timeouts / last error），但 capability probe（`server/discover` 现代路径、legacy `initialize` + `tools/list`）与重连次数未实现。诊断只能看当前连接结果，不能主动探测能力或观察重连稳定性。

## What Changes

- capability probe：initialize 后调用 `server/discover`；记录 supported versions / capabilities / instructions / ttl / cache scope；失败保留连接并诊断。
- legacy 路径：不支持 `server/discover` 时用现有 initialize + `tools/list` 结果，不重复探测。
- probe 时机：连接成功后异步探测；按 connection key 缓存；TTL 取 server `ttl_ms`（夹取 60s–1h）；`/mcp probe` 手动强制刷新。
- 重连计数：pool 按 connection key 统计自动重连次数；`/mcp` 展示；手动 `/reload` 重置。
- 错误分类：unsupported / timeout / transport / auth / invalid_response。

## Scope

- 不改连接建立顺序；probe 不阻塞 UI 或首回合。
- 不做 pre-initialize 探测（rmcp 高层 client 不支持）。
- 不把 capabilities 写进模型上下文。

## Impact

- `xylitol-official-plugin-mcp`：pool probe 状态、缓存、重连计数。
- `xylitol-serve` / `/mcp` 面板：诊断字段。
- `xylitol-app-tui`：`/mcp probe` 命令与展示。

## Further Notes

- 2026-11-07 从 delayed 提升为 active。
- c3075 已归档；本 change 只补 probe 与计数。
