# Design — c1080-update-infra-mcp-client-product

## 边界

| 本变更 | 不做 |
|---|---|
| 配置校验 + 装配可观测 | 启动 header（c1135 deferred） |
| 已连接列表只读 API | `/reload` slash UI（c1120） |
| stdio + Sse/url | 第三传输协议 |

## 失败策略

```text
bootstrap:
  for each mcp_servers entry:
    try connect → register mcp:{server}:{tool}
    on err → warn + diagnostic；继续其余服务器
  MUST NOT Err-out entire bootstrap solely for one MCP failure
```

## 只读缝

`connected_mcp_servers()`（或等价）：`Vec<{ id, transport, tool_count }>`；空配置 → 空列表且无 client（mcp1）。

## 与 c1085

并行：skills 走 ResourceLoader / Trust；MCP 走 config。共享「可查询目录」形态供 c1120，UI 各自下游。
