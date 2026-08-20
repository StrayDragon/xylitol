# language: zh-CN
# managed by llman sdd partition-migrate
功能: server-ws

  @req:w1
  场景: frame-serialize
    假如 构造下行 ServerRequest session/event
    当 序列化为 JSON
    那么 JSON 含 type=server-request 与 method=session/event

  @req:w2
  场景: subscribe-frame
    假如 客户端欲以 last_seq 5 订阅会话 s0
    当 发送 unary subscribe
    那么 payload 含 session_id=s0 与 last_seq=5
    并且 mux 不接受 Subscribe 应用帧

  @req:w3
  场景: ws-upgrade-mounted
    假如 客户端连接 /api/events.mux
    当 server 接受升级
    那么 连接只收下行 ServerRequest
    并且 不把会话绑在 /api/v1/session/x/ws

  @req:w4
  场景: seq-monotonic
    假如 向会话 append 3 个事件
    当 读回事件
    那么 seq 值为 1,2,3（严格递增）

  @req:w5
  场景: resync-on-wrap
    假如 向会话 append 10001 个事件（journal 容量=10000）
    当 last_seq=0 的客户端 unary subscribe
    那么 server 发送 session/resync_required 因事件 0..1 已丢失

  @req:w6
  场景: resync-recover
    假如 客户端收到 session/resync_required
    当 客户端以 last_seq=0 再次 unary subscribe
    那么 server 从 journal 重放全部可用事件

  @req:w7
  场景: ws-events-pushed
    假如 客户端已 subscribe 且 prompt 运行
    当 agent 发出 TextDelta 事件
    那么 客户端在 mux 上收到 session/event 的 ServerRequest
