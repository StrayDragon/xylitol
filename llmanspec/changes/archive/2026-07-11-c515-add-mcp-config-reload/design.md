# design — c515 MCP 配置驱动与重载

## 行为

```text
mcp_servers 缺失或 []
    → 不构造 McpClientManager
    → ToolSet 无 mcp:* 工具
    → zero-cost

mcp_servers 非空
    → 按 transport 连接/启动
    → 发现工具 → McpToolAdapter : XyTool
    → 名称 mcp:{server_id}:{name}
```

## 重载

- 缝建议：`Driver` 或 composition 暴露的 `reload_mcp(&McpConfig)`（具体签名实施时定）。
- 语义：拆掉旧 client/工具 → 按新配置重建 → 替换 ToolSet 中 `mcp:` 前缀项；内置七工具保留。
- 分阶段：MVP 可先「仅启动时装配」+ 测试钩子验证重载函数；热更新路径须在本变更内可测，不得把「重启进程」当唯一手段写进合约。

## 边界

- `rmcp` 仅 `infra/mcp`
- 安全 allowlist / UI 不在本变更
