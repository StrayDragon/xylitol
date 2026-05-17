---
depends_on: [c10-add-config, c20-add-tools]
---

# c65-add-skills-mcp

## Why

Skill 声明式定义 + MCP 客户端使 agent 能动态加载外部工具和技能包，保持核心二进制精简（§9）。

## What Changes

1. 在 `src/infra/skills/` 实现 Skill 系统
   - YAML 声明式定义（name, description, system_prompt_addon, allowed_tools）
   - 运行时加载与注册
2. 实现 MCP 客户端
   - stdio 和 SSE 两种传输
   - 启动时连接 MCP server，获取工具列表
   - 代理调用 `mcp_tool(name, args)` 转发到 MCP server

### Skill 配置

```yaml
skills:
  - name: python-testing
    description: "Write and run pytest tests"
    system_prompt_addon: |
      When writing tests, always use pytest...
    allowed_tools: [shell, file_write, lsp_query]
```

### MCP 配置

```yaml
mcp_servers:
  - id: filesystem
    type: stdio
    command: ["npx", "@modelcontextprotocol/server-filesystem", "/home/user/projects"]
  - id: sqlite
    type: stdio
    command: ["uvx", "mcp-server-sqlite"]
  - id: remote-tool
    type: sse
    url: "https://mcp.example.com/sse"
```

## Capabilities

- `skills-mcp`: Skill 声明式定义 + MCP 客户端（rmcp via adk-tool）

## Impact

- 新增 `rmcp` 依赖（通过 adk-tool 集成）
- feature flag `infra-skills` 启用此模块
- MCP server 返回的工具自动受 Hook 和安全策略监控
