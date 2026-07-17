# language: zh-CN
# managed by llman sdd partition-migrate
功能: protocol-app

  @req:ip1
  场景: jsonl-commands-in-enum
    假如 检查 Command 枚举
    当 检查 ExportJsonl 与 ImportJsonl 变体
    那么 两变体存在且可从 client 消息反序列化

  @req:ip2
  场景: approval-event
    假如 agent 需审批工具
    当 发出 ApprovalRequired 事件
    那么 携带 call_id 与 tool summary

  @req:ip3
  场景: envelope-serializes
    假如 构造含 code Ok 与 data 的 Envelope
    当 序列化为 JSON
    那么 JSON 含 code、msg、data、request_id 字段

  @req:ip6
  场景: subscribe-command-serializes
    假如 Command::Subscribe {session_id: s0, last_seq: 5} 序列化为 JSON
    当 反序列化回来
    那么 匹配原始 session_id 与 last_seq

  @req:ip7
  场景: thinking-delta-variant
    假如 检查 Event 枚举
    当 检查 TextDelta 与 ThinkingDelta 变体
    那么 两变体存在且携带 text 字段

  @req:ip7
  场景: thinking-roundtrip
    假如 XyEvent::ThinkingDelta 经 wire 往返转换
    当 反序列化
    那么 结果等于原始 ThinkingDelta 而非空 MessageUpdate

  @req:ip8
  场景: switch-validates-target
    假如 client 对不存在 session id 发送 SwitchSession
    当 RPC dispatch 处理
    那么 返回错误且活动会话未改变

  @req:ip8
  场景: get-messages-returns-entries
    假如 活动会话有持久化条目
    当 分发 GetMessages 命令
    那么 返回已加载 SessionEntry 记录（非 stub 错误）

  @req:ip9
  场景: ws-variants-stay-in-server
    当 检查 app::core::dispatch 源码
    那么 不含 Subscribe、ApproveTool、AnswerQuestion 处理；这些留在 server/ws.rs

  @req:ip9
  场景: tui-dispatch-path
    假如 tui 分发 Command::SetModel
    当 追踪执行路径
    那么 到达 app::core::dispatch 并由 Driver 支撑，非并行 tui 本地实现

  @req:ip-q1
  场景: serde-steer
    当 序列化再反序列化 Command::Steer
    那么 字段与原文一致

  @req:ip-q2
  场景: dispatch-steer
    当 dispatch Steer
    那么 调用 Driver::steer 且不报未处理变体错误

  @req:ip10
  场景: no-vendor-wire
    当 对比 protocol::Event 与各 provider SSE 事件名
    那么 无线协议级厂商专名变体

  @req:pa-wire1
  场景: queue-on-wire
    当 XyEvent QueueUpdate 经 to_wire_event
    那么 得到对应 Event 变体或文档化等价载荷
