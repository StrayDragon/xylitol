# language: zh-CN
# managed by llman sdd partition-migrate
功能: infra-mcp

  @req:mcp1
  场景: zero-cost
    假如 配置无 mcp_servers
    当 bootstrap/composition 装配 agent
    那么 无 McpClientManager 实例且工具列表无 mcp__ 前缀

  @req:mcp2
  场景: wired
    假如 配置含一个可用 mcp server
    当 装配完成
    那么 ToolSet 含对应 mcp__ 工具且可经 XyTool 执行

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

  @req:mcp7
  场景: startup-does-not-await-all-mcp
    假如 配置含至少一个慢启动或可连接的 mcp server
    当 产品启动进入 TUI
    那么 应用面已渲染且未要求全部 MCP 连接完成

  @req:mcp7
  场景: parallel-connect-progress-snapshot
    假如 配置含多个 mcp server
    当 后台连接进行中
    那么 Driver 只读缝可观察到进行中或已连接进度快照且单失败不拖死其余

  @req:mcp7
  场景: agent-prompt-allowed-while-connecting
    假如 MCP 仍在 connecting
    当 用户提交普通 agent prompt
    那么 允许提交进队列或等价路径；MUST NOT 因 connecting 短拒；generate 门闸语义见 mcp8

  @req:mcp7
  场景: settle-updates-registry-unique-names
    假如 MCP 结算或 reload 成功并更新 registry
    当 查询内部 armed / ToolSet 名（未谈 provider 定稿）
    那么 每个工具名唯一且内置工具仍在

  @req:mcp8
  场景: frozen-settle-does-not-expand-provider-tools
    假如 会话工具表已 FROZEN
    当 又有 MCP settle 发现新工具
    那么 provider 可见 tools 表 MUST NOT 静默变长；用户可经 idle /reload 重定稿

  @req:mcp7
  场景: snapshot-exposes-tools-armed
    假如 MCP 部分或全部已 settle
    当 读取 Driver/composition MCP 快照
    那么 可观察每 server 或汇总的 tools armed 态
