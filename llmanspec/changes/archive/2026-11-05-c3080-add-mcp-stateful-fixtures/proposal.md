---
depends_on: []
title: "MCP stateful fixtures and multi-client tests"
branch: main
base_branch: origin/main
base_sha: 78d077258664b34c5b9147336d76994cb6f448b7
---

## Why

当前没有 stateful MCP fixture；无法验证 scope 隔离、no-reset、多 client / 多 workspace、OAuth、tool list changed、legacy SSE 拒绝。

## What Changes

- 新写 stateful fixture：`tests/support/mcp_stateful_fixture.py`，用 Python 官方 SDK `mcp`（FastMCP）实现；不复用现有 minimal fixture。
- 现有 `tests/support/mcp_fixture_server.py` 与 plugin 内副本保持原样，继续服务 reload / provider-tools 简单测试；不强行合并。
- stdio fixture：按连接维护计数器/状态；可配置延迟与失败。
- 覆盖 tools / resources / prompts / elicitation form+url / tool list changed；支持 legacy SSE 拒绝测试。
- BDD/E2E：fixture 先行；多 client、多 workspace、no-reset 断言随 `c3055` / `c3060` / `c3065` 行为落地后再接；workspace set 人类变更（后续）。
- OAuth mock authorization server：授权、refresh、revoke、多账号、脱敏。
- 测试 harness 接线：`test-bdd` / `test-mcp-fixtures`。

## SDK 与 CI

- 官方 SDK 版本：TypeScript `@modelcontextprotocol/sdk` 1.32.2（参考实现）；Python `mcp` 2.3.0；Rust `rmcp` 3.8.0（workspace 已用）。
- 选择 Python 官方 SDK：FastMCP 高层 API 最短路径；与 Rust client 跨实现验证，避免同库盲区。
- 单脚本实现：`uv add --script tests/support/mcp_stateful_fixture.py mcp==2.3.0` 写入 PEP 723 inline metadata；`uv run tests/support/mcp_stateful_fixture.py` 直接运行，不初始化 venv。
- CI 允许 `uv`：前置工具必须已安装；apply 时必须在 `.github/workflows/ci.yaml` 安装 `uv`；脚本用 `requires-python >=3.11`；`uv run` 按 inline metadata 解析依赖。
- 若 CI 环境缺失 `uv`，fallback 到 Rust `rmcp` server（官方 Rust SDK，启用 `server` + `transport-io`）。
- TypeScript 作为参考实现备选；不默认，避免 Node 运行时依赖。

## Capabilities

- `test-bdd`：多 client / 多 workspace 场景。
- `test-mcp-fixtures`：stateful fixture。

## Impact

测试基础设施、CI；不改产品行为。

## Further Notes

- 依赖：无；纯测试基建，fixture 可先行；no-reset / 多 workspace / OAuth / tool list changed 断言依赖 `c3055` / `c3060` / `c3065` / `c3070` / `c3090` 行为落地。
- 新 fixture 只落 `tests/support`；现有 minimal fixture 不动。
- CI 前置：`uv` 必须在测试前安装；Python 3.11 由 `uv run` 按脚本 `requires-python` 处理；依赖由 inline metadata 固定；不依赖测试时联网。

## Open Questions

- CI setup 的 `uv` / Python 3.11 安装落点。
- stateful fixture 的 state 形状与重置方式。
