# language: zh-CN
# migrated from tests/features/approval.feature
功能: server-reverse-rpc
  场景: approve-roundtrip
    假如 服务端和已连接的 mux 客户端
    当 agent 执行需要审批的工具
    那么 客户端收到带有 rpcId 的 approval/requested
    当 客户端 POST /api/respond 且 approved=true
    那么 工具执行继续
    并且 turn 正常结束

  场景: tool-denied
    假如 服务端和已连接的 mux 客户端
    当 agent 执行需要审批的工具
    并且 客户端 POST /api/respond 且 approved=false
    那么 工具被拒绝
    并且 turn 继续但不包含工具结果

  @req:rr1
  场景: approval-broadcast
    假如 agent 发出 ApprovalRequired，rpcId 为 xyz
    当 server 检查 session 的 mux 连接
    那么 3 个已连接客户端均收到 approval/requested

  @req:rr2
  场景: first-wins
    假如 2 个客户端 POST /api/respond，rpcId xyz（首个 true，50ms 后 false）
    当 server 处理首个应答
    那么 agent 以 approved=true 恢复；第二个应答被忽略

  @req:rr3
  场景: timeout-error
    假如 60s 内无客户端 POST /api/respond
    当 server 将 rpcId 标记为 expired
    那么 agent 收到 ApprovalTimeout 错误

  @req:rr4
  场景: second-answer-ignored
    假如 第二个客户端对已消费 rpcId POST /api/respond
    当 server 收到该应答
    那么 应答被静默忽略（无状态变化、无错误）
