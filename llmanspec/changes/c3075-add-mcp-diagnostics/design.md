# Design — MCP diagnostics

## 1. /mcp 面板数据

- SessionPrepareView 增加 server_details；每 server 一行：
  server / transport / scope / credential（set|none）/ connected / tools / prompts / resources / connect_timeout_ms / call_timeout_ms / last_error。
- credential 只显示是否存在，不显示值。
- last_error 来自 pool diags；legacy SSE 错误由 c3085 产生。

## 2. 输出路径

- runtime mcp_status_summary 在连接摘要后逐行输出 server_details。
- 产品 TUI /mcp 面板沿用现有 armed 计数与 summary；本 change 不改面板布局。
- 日志与面板均不输出 token / credential 值。

## 3. 未做（明确边界）

- capability probe（server/discover 或 legacy initialize + tools/list）未接。
- 重连次数未统计。
- 上述未做项不得写成已交付。

## 4. 测试

- 无连接时 server_details 为空。
- 有连接时字段齐全、credential 只显示 set/none。
