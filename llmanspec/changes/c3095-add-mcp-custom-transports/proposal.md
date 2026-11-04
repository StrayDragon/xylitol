---
depends_on: [c3085-add-mcp-transport-config]
title: "MCP custom transports"
---

## Why

MCP 标准只有 stdio 与 Streamable HTTP；WebSocket / custom transport 未接线。需要扩展点，但不应污染核心传输。

## What Changes

- 新增 custom transport 扩展 seam；核心仍只支持 stdio / streamable_http。
- WebSocket / custom 作为可选 transport；复用 stdio framing（byte stream）与 JSON-RPC。
- config 预留 `transport: websocket` / `custom`；校验与文档。
- legacy SSE 仍不支持。
- 诊断与测试：custom transport 的连接/取消/关闭。

## Capabilities

- `infra-mcp`：custom transport seam。
- `cli-*`：config 校验。

## Impact

MCP plugin、config、host。

## Further Notes

- 低优先级；标准传输先稳定。
- 依赖 `c3085-add-mcp-transport-config`。

## Open Questions

- WebSocket 是否作为内置 transport，还是插件扩展。
- custom transport 的配置形状与安全边界。
