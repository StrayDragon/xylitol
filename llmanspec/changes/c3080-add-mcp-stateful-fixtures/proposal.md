---
depends_on: []
title: "MCP stateful fixtures and multi-client tests"
---

## Why

当前没有 stateful MCP fixture；无法验证 scope 隔离、no-reset、多 client/多 workspace、OAuth、tool list changed、legacy SSE 拒绝。

## What Changes

- fixture 用官方 MCP SDK 实现；推荐 Rust `rmcp` server（与 client 同一官方 Rust SDK，无额外 runtime），启用 server + transport-io。
- stdio fixture：按连接维护计数器/状态；可配置延迟与失败。
- 覆盖 tools / resources / prompts / elicitation form+url / tool list changed；支持 legacy SSE 拒绝测试。
- BDD/E2E：多 client、多 workspace、一个 client 启动不 reset 另一个；workspace set 人类变更。
- OAuth mock authorization server：授权、refresh、revoke、多账号、脱敏。
- 测试 harness 接线：`test-bdd` / `test-mcp-fixtures`。
- 若 Python 官方 SDK 更合适，用 `uv` 隔离；默认 Rust rmcp。

## Capabilities

- `test-bdd`：多 client/多 workspace 场景。
- `test-mcp-fixtures`：stateful fixture。

## Impact

测试基础设施、CI；不改产品行为。

## Further Notes

- 依赖：无；横切支撑 `c3055` / `c3060` / `c3065` / `c3070` / `c3090`。
- fixture 只落 `tests/support` 或 `scripts/lab_*`，不新增 just recipe。
- 官方 SDK：modelcontextprotocol/rust-sdk（rmcp）为官方 Rust SDK；server 功能需显式 feature。

## Open Questions

- fixture 用 Rust rmcp 还是 Python 官方 SDK：推荐 Rust rmcp。
- fixture 是否复用现有 `mcp_fixture_server.py`，还是新写 stateful server。
