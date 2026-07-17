# language: zh-CN
# managed by llman sdd partition-migrate
功能: infra-mcp

  @req:mcp1
  场景: zero-cost
    假如 配置无 mcp_servers
    当 bootstrap/composition 装配 agent
    那么 无 McpClientManager 实例且工具列表无 mcp: 前缀

  @req:mcp2
  场景: wired
    假如 配置含一个可用 mcp server
    当 装配完成
    那么 ToolSet 含对应 mcp: 工具且可经 XyTool 执行

  @req:mcp3
  场景: reload
    假如 已装配一套 MCP 工具
    当 配置变更为另一组 mcp_servers 并触发重载
    那么 ToolSet 反映新集合且旧服务器工具不再可用且内置工具仍在

  @req:mcp4
  场景: invalid-stdio
    假如 stdio 条目缺 command
    当 装配或校验
    那么 诊断或错误指出缺字段且该条目未注册 mcp 工具

  @req:mcp4
  场景: connect-fail-continues
    假如 一个 server 连接失败且另一配置有效
    当 bootstrap 装配
    那么 进程仍成功；失败可观察；有效服务器工具仍可注册

  @req:mcp5
  场景: snapshot-empty
    假如 配置无 mcp_servers
    当 查询已连接列表
    那么 空列表且无 client

  @req:mcp5
  场景: snapshot-connected
    假如 一个 server 已成功连接并注册工具
    当 查询已连接列表
    那么 含该 server id 与非零工具数或等价摘要
