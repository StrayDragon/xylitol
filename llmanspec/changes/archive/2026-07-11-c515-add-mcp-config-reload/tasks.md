# Tasks — c515-add-mcp-config-reload

- [x] 1. 确认 `McpServerConfig` 字段与文档；缺省/空 = 未启用
- [x] 2. composition/bootstrap：仅当有服务器时创建 `McpClientManager` 并注入 ToolSet
- [x] 3. 无配置路径单测：零 MCP 类型构造 / 工具名无 `mcp:` 前缀
- [x] 4. 有配置路径：假 MCP 或集成测试验证 `XyTool` 注册与命名
- [x] 5. 设计并实现重载 API（例如 `reload_mcp_tools(&new_config)` 挂在 Driver/Agent 可测缝）
- [x] 6. `rg rmcp src/agent src/domain` 为零
- [x] 7. `llman sdd validate c515-add-mcp-config-reload --strict --no-interactive`
- [x] 8. `just lint` + 相关测试
