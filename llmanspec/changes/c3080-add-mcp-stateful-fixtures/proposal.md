---
depends_on: []
title: "MCP stateful fixtures and multi-client tests"
---

## Why

当前没有 stateful MCP fixture；无法验证 scope 隔离、no-reset、多 client/多 workspace、OAuth、tool list changed、legacy SSE 拒绝。

## What Changes

- stdio fixture：按连接维护计数器/状态；可配置延迟与失败。
- BDD/E2E：多 client、多 workspace、一个 client 启动不 reset 另一个。
- OAuth mock authorization server：授权、refresh、revoke、多账号、脱敏。
- tool list changed fixture；elicitation form/url；prompts/resources；legacy SSE 拒绝。
- 测试 harness 接线：`test-bdd` / `test-mcp-fixtures`。

## Capabilities

- `test-bdd`：多 client/多 workspace 场景。
- `test-mcp-fixtures`：stateful fixture。

## Impact

测试基础设施、CI；不改产品行为。

## Further Notes

- 依赖：无；横切支撑 `c3055` / `c3060` / `c3065` / `c3070`。
- fixture 只落 `tests/support` 或 `scripts/lab_*`，不新增 just recipe。

## Open Questions

- fixture 是否复用现有 `mcp_fixture_server.py`，还是新写 stateful server。
