---
depends_on: [c3055-add-mcp-scope-pool]
title: "MCP transport config and legacy SSE rejection"
---

## Why

config 的 `sse` 实际映射 Streamable HTTP，命名误导；legacy HTTP+SSE 不支持；没有 per-server timeout 配置。

## What Changes

- canonical transport：`stdio` / `streamable_http`。
- `transport: sse` 作为迁移别名，映射 `streamable_http`，打 deprecation warning；legacy SSE server 连接失败并明确报错。
- remote 用 `url`；stdio 用 `command` / `args`；headers / env 保持。
- per-server connect / call timeout 配置；默认保持 8s / 120s。
- custom transport seam 预留（`websocket` / `custom` 后续）。
- config 校验与文档；`/mcp` 面板显示 transport。

## Capabilities

- `infra-mcp`：transport 配置与校验。
- `cli-*`：配置迁移错误。
- `app-tui-commands`：`/mcp` 显示 transport。

## Impact

MCP plugin、config、host、TUI。

## Further Notes

- item 4 调研结论：主流用 `http` / `streamable-http` / `streamableHttp`；`sse` 留给 legacy；MCP registry 推荐 `"streamable-http"`。
- 选择：canonical `streamable_http` + `sse` 迁移别名；legacy SSE 明确拒绝。
- 依赖 `c3055-add-mcp-scope-pool`。

## Open Questions

- `sse` 别名保留多久；是否在下一个 major 移除。
- 是否增加 `http` 别名：不推荐。
