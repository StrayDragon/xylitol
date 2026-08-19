# language: zh-CN
# migrated from tests/features/server.feature
功能: server-runtime

  场景: start-healthz
    当 服务端在空闲端口上启动
    那么 healthz 端点返回 200 OK

  场景: second-instance-rejected
    假如 服务端已在该地址端口监听
    当 第二个服务端绑定同一地址端口
    那么 第二个实例因地址占用失败

  @req:sr1
  场景: server-under-app
    当 server 应用面启动
    那么 装配 infra 运行时并按 session 槽注入 Driver，供 unary 处理器调用

  @req:sr-env1
  场景: product-path-four-quadrant
    当 产品 TUI 访问 Host
    那么 经四象限 POST unary 与 WebSocket 下行
    并且 Host 对该路径给出可观察往返

  @req:sr2
  场景: server-rest-ws-under-app
    当 启动 app::server 运行时
    那么 暴露 POST /api/{method}、POST /api/respond 与只下行的 events.mux
    并且 不暴露 /api/v1 产品 REST

  @req:sr3
  场景: server-lock-under-app
    当 第二个 app::server 进程绑定同一地址端口
    那么 因地址占用失败且不写整机锁文件

  @req:sr4
  场景: reconnect-replay
    假如 客户端断开 N 秒后以 last_seq unary subscribe
    当 断开期间 server 产生事件
    那么 客户端收到 last_seq+1 起全部遗漏事件再收实时事件

  @req:sr5
  场景: approval-roundtrip
    假如 工具需审批
    当 推送 ApprovalRequired
    那么 客户端经 POST /api/respond 应答且回合恢复

  @req:sr7
  场景: lock-file-json
    假如 server 已启动
    当 检查单实例互斥手段
    那么 不依赖锁文件 JSON 的 port、pid、hostname

  @req:sr8
  场景: port-retry-binds
    假如 18790 被其它进程占用
    当 server 在 18790 启动
    那么 失败返回且不绑定 18791

  @req:sr9
  场景: run-endpoint-works
    假如 向 POST /api/prompt 发送 ClientRequest
    当 server 处理 prompt
    那么 HTTP 200 且 ServerResponse 回显 rpcId
    并且 POST /api/v1/session/x/run 不是产品路径

  @req:sr10
  场景: server-composition-export-io
    当 app::server 运行时经 app::core::composition 组装 Agent
    那么 向 Agent 传入 Arc<dyn ExportIo>

  @req:sr-driver1
  场景: no-bare-runtime
    假如 审查 server 组合根
    当 检查持有类型
    那么 按 session 槽持有 Driver；handlers 不直接以 Mutex AgentRuntime 作为唯一入口

  @req:sr-remote1
  场景: remote-commands
    当 调用 RemoteDriver 已登记 unary（如 steer）
    那么 经 HostClient 到达 Host
    并且 未登记方法不发明 REST

  @req:sr-dispatch1
  场景: shared-path
    当 审查 prompt/steer 等 unary handlers
    那么 调用 dispatch 或共享 helper；与 InProcessDriver 语义一致

  @req:sr-st1
  场景: rest-get-tree
    假如 server 上存在 session 与消息树
    当 请求 /api/v1 会话树 REST
    那么 不是产品路径（不存在或非产品）

  @req:sr-st1
  场景: remote-travel
    假如 RemoteDriver 指向该 server
    当 调用未入方法表的 session_tree/travel
    那么 不经 REST 冒充；可为未实现或默认值

  @req:sr-w1
  场景: writer-lease
    假如 客户端 A 已对 session 发出非只读 unary
    当 客户端 B 无 writerToken 再发非只读 unary
    那么 业务错误说明已有写者
