---
depends_on: []
title: "MCP stateful fixtures and multi-client tests"
---

## Why

当前没有 stateful MCP fixture；无法验证 scope 隔离、no-reset、多 client/多 workspace、OAuth、tool list changed、legacy SSE 拒绝。

## What Changes

- fixture 用官方 MCP SDK 实现；推荐 Python 官方 SDK `mcp`（FastMCP）并用 `uv` 隔离、pin 版本。
- stdio fixture：按连接维护计数器/状态；可配置延迟与失败。
- 覆盖 tools / resources / prompts / elicitation form+url / tool list changed；支持 legacy SSE 拒绝测试。
- BDD/E2E：多 client、多 workspace、一个 client 启动不 reset 另一个；workspace set 人类变更（后续）。
- OAuth mock authorization server：授权、refresh、revoke、多账号、脱敏。
- 测试 harness 接线：`test-bdd` / `test-mcp-fixtures`。

## SDK 选择

- 官方 SDK 版本：TypeScript `@modelcontextprotocol/sdk` 1.32.2（参考实现）；Python `mcp` 2.3.0；Rust `rmcp` 3.8.0（workspace 已用）。
- 推荐 Python 官方 SDK：FastMCP 高层 API 最短路径；与 Rust client 跨实现验证，避免同库盲区；`uv run --python 3.11 --with mcp==2.3.0` 隔离。
- 若 CI 无法安装 Python 依赖，fallback 到 Rust `rmcp` server（官方 Rust SDK，启用 `server` + `transport-io`）。
- TypeScript 作为参考实现备选；不默认，避免 Node 运行时依赖。

## Capabilities

- `test-bdd`：多 client/多 workspace 场景。
- `test-mcp-fixtures`：stateful fixture。

## Impact

测试基础设施、CI；不改产品行为。

## Further Notes

- 依赖：无；横切支撑 `c3055` / `c3060` / `c3065` / `c3070` / `c3090`。
- fixture 只落 `tests/support` 或 `scripts/lab_*`，不新增 just recipe。
- 官方 SDK：TypeScript / Python / Rust 均为官方；Python FastMCP 最简单。

## Open Questions

- CI 是否允许 `uv` 安装 `mcp`；不允许则用 Rust rmcp。
- fixture 是否复用现有 `mcp_fixture_server.py`，还是新写 stateful server。
