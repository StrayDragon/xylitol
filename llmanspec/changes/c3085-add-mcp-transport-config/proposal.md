---
depends_on: [c3055-add-mcp-scope-pool]
title: "MCP transport config and legacy SSE rejection"
---

## Why

config 的 `sse` 实际映射 Streamable HTTP，命名误导；legacy HTTP+SSE 不支持；没有 per-server timeout 配置。

## What Changes

- canonical transport：`stdio` / `http`。`http` 表示 Streamable HTTP。
- `transport: sse` 直接拒绝（破坏性变更，pre-release 阶段不做兼容别名）；错误提示改用 `http`。
- legacy SSE server 连接失败并明确报错：不支持 legacy HTTP+SSE。
- remote 用 `url`；stdio 用 `command` / `args`；headers / env 保持。
- per-server connect / call timeout 配置；默认保持 8s / 120s。
- custom transport seam 预留（`websocket` / `custom` 后续）。
- config 校验与文档；transport 字段供 `/mcp` 面板显示。

## Capabilities

- `infra-mcp`：transport 配置与校验。
- `cli-*`：配置迁移错误。

## Impact

MCP plugin、config、host、TUI。

## Further Notes

- item 4 调研结论：主流用 `http` / `streamable-http` / `streamableHttp`；`sse` 留给 legacy；Claude Code 用 `http`。
- 决策：canonical `http`；`sse` 破坏性拒绝；legacy SSE 明确不支持。
- 本 change 独占 per-server timeout 配置与 legacy SSE 拒绝 / 报错；`c3075-add-mcp-diagnostics` 只显示，不重复实现。
- 依赖 `c3055-add-mcp-scope-pool`。

## Open Questions

- 无；命名决策已定。
