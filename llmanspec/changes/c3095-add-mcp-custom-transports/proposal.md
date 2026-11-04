---
depends_on: [c3085-add-mcp-transport-config]
title: "MCP custom transport insertion point"
---

## Why

MCP 标准只有 stdio 与 Streamable HTTP；WebSocket / custom transport 很少见。自定义供应商应自己贡献 xylitol 插件来支持，不需要核心内置。

## What Changes

- **暂不实现** custom transport；只保留插入点。
- 核心仍只支持 stdio / http。
- 预留 transport extension seam：插件可注册自定义 transport；复用 JSON-RPC framing。
- config 预留 `transport` 扩展位；校验与文档。
- legacy SSE 仍不支持。
- 诊断与测试：插入点可用性；不承诺内置 WebSocket。

## Capabilities

- `infra-mcp`：custom transport seam。
- `cli-*`：config 校验。

## Impact

MCP plugin、config、host。

## Further Notes

- 用户决策：先不做；自定义供应商通过插件贡献。
- 依赖 `c3085-add-mcp-transport-config`。
- fixture 只需验证插入点，不需要实现 WebSocket。

## Open Questions

- 插入点的插件 API 形状与安全边界。
- 是否允许第三方 transport 插件进入默认构建。
