# Design — 修复 MCP 欢迎行闪烁

`LoadedResourcesSnapshot` 合并规则：

- 若 prev 有 MCP 证据（configured / servers / connected / connecting / diag），next 全无，且 `next.mcp_bootstrap_complete == false`：保留 prev 的 MCP 字段与 bootstrap/frozen 标志。
- 若 next 已 `mcp_bootstrap_complete == true`：以 next 为准，允许隐藏。
- 其他情况照常覆盖。
