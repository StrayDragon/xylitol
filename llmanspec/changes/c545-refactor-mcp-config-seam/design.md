# design — c545 MCP 配置缝

## 目标

```text
bootstrap → BootstrappedRuntime { mcp_servers: Vec<McpServerSpec> 或等价 }
  → McpSession::reload(&mut Driver, &[McpServerSpec])
  → 内部再转 infra::McpServerConfig
```

嵌入方只见缝类型；infra 配置仍是实现细节。

## 选项

1. **新缝类型** `McpServerSpec`（字段与配置对齐，serde 可选）放在 `app::core` 或 `domain` 旁路——优先 `app::core` / embed re-export。
2. **Opaque**：`type McpServers = …` 仅 crate 内可见构造——嵌入方只能透传 bootstrap 结果，不能自建（过严，不选）。

定稿倾向选项 1。
