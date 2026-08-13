# language: zh-CN
# migrated from tests/features/server.feature
功能: server-runtime
  背景:
    假如 锁路径已清理

  场景: start-healthz
    当 服务端在空闲端口上启动
    那么 healthz 端点返回 200 OK
    并且 锁文件包含 port, pid, hostname

  场景: second-instance-rejected
    假如 服务端已在运行（锁文件存在）
    当 第二个服务端启动（相同锁路径）
    那么 第二个实例收到 ServerLockedError

  @req:sr1
  场景: server-under-app
    当 server 应用面启动
    那么 装配 infra 运行时并注入 agent 端口，供路由处理器调用

  @req:sr2
  场景: server-rest-ws-under-app
    当 启动 app::server 运行时
    那么 暴露使用 protocol::Command 与 protocol::Event 的 REST 与 WebSocket 端点

  @req:sr3
  场景: server-lock-under-app
    当 第二个 app::server 进程对同一锁路径启动
    那么 收到 ServerLockedError

  @req:sr4
  场景: reconnect-replay
    假如 客户端断开 N 秒后以 last_seq 重连
    当 断开期间 server 产生事件
    那么 客户端收到 last_seq+1 起全部遗漏事件再收实时事件

  @req:sr5
  场景: approval-roundtrip
    假如 工具需审批
    当 推送 ApprovalRequired
    那么 客户端应答 ApproveTool 且回合恢复

  @req:sr7
  场景: lock-file-json
    假如 server 获取锁
    当 读取锁文件
    那么 含有效 JSON 的 port、pid、hostname 字段

  @req:sr8
  场景: port-retry-binds
    假如 8080 被其它进程占用
    当 server 在 8080 启动
    那么 重试并绑定 8081 后更新锁文件

  @req:sr9
  场景: run-endpoint-works
    假如 向 /api/v1/session/x/run POST prompt
    当 server 处理 prompt
    那么 端点返回 200 与 session_id

  @req:sr10
  场景: server-composition-export-io
    当 app::server 运行时经 app::core::composition 组装 Agent
    那么 向 Agent 传入 Arc<dyn ExportIo>

  @req:sr-driver1
  场景: no-bare-runtime
    假如 审查 server AppState
    当 检查持有类型
    那么 持有 Driver；handlers 不直接以 Mutex AgentRuntime 作为唯一入口

  @req:sr-remote1
  场景: remote-commands
    当 调用 RemoteDriver steer 与 queue_stats
    那么 成功并与 server 状态一致

  @req:sr-dispatch1
  场景: shared-path
    当 审查 switch_model/steer 等 handlers
    那么 调用 dispatch 或共享 helper；与 InProcessDriver 语义一致

  @req:sr-st1
  场景: rest-get-tree
    假如 server 上存在 session 与消息树
    当 GET 文档化的 session tree 端点且 kind=MessageHistory
    那么 200 且 body 含树节点

  @req:sr-st1
  场景: remote-travel
    假如 RemoteDriver 指向该 server
    当 travel_session_tree(MessageHistory, user_entry_id)
    那么 返回 SessionTreeTravel 且与 InProcess 语义一致（travel→父 leaf + editor_text）
