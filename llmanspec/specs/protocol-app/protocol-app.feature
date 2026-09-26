# language: zh-CN
# capability: protocol-app
# purpose: Client↔host 产品真源为 JSON-RPC 2.0（入口 POST /rpc 与 WS /rpc）；Command/Event 为方法载荷与下行 notification 内容。
# scope: src/, tests/

功能: protocol-app

  @req:r1693 @human
  场景: Command 枚举
    - protocol MUST 定义 Command 枚举，至少覆盖 Run、Cancel、SwitchModel、SetThinking、ListCommands、ApproveTool、AnswerQuestion、SwitchSession、GetMessages、ExportHtml、ExportJsonl、ImportJsonl；每个 client→core 消息 MUST 反序列化为此枚举。

  @req:r1695 @human
  场景: Event 枚举
    - protocol/ MUST 定义 Event 枚举，至少覆盖 TurnStart、Delta、ToolCall、ApprovalRequired、QuestionRequired、TurnEnd、Usage；每个 core→client 消息 MUST 反序列化为此枚举。

  @req:r1696 @human
  场景: 信封与错误
    - 产品 unary 应答 MUST 使用 JSON-RPC 2.0 结果形态：成功则带 result，失败则带 error（message 为 details）。产品业务码 MUST 为 error.data 内稳定字符串；JSON-RPC 数字码 MUST 仅作载体，MUST NOT 成为产品错误模型。合法信封的 HTTP 状态 MUST 表示载体成功；非法信封 MUST 失败。旧 REST {code,msg,data} 与四象限 `{ok,value,error}` 顶层形态 MUST NOT 再作为产品 TUI 路径。

  @req:r1697 @human
  场景: Subscribe 命令
    - 产品 MUST 支持以 session 与 last_seq 订阅事件流；订阅确认 MUST 能表达 session 与当前 seq。该订阅 MUST 走产品 JSON-RPC 入口，MUST NOT 另开 REST 或非 JSON-RPC 的 WS 应用帧。

  @req:r1698 @human
  场景: Event 变体完整
    - protocol::Event MUST 覆盖全部 AgentEvent 变体：TurnStart、TurnEnd、MessageStart、MessageEnd、MessageUpdate、ToolExecutionUpdate、CompactionEnd、TextDelta、ThinkingDelta；ThinkingDelta MUST 往返 XyEvent::ThinkingDelta 且不得降级为空 MessageUpdate。

  @req:r1699 @human
  场景: 会话命令分发
    - RPC dispatch MUST 实现 SwitchSession：经 SessionStore 校验目标会话存在并切换上下文；GetMessages 返回已加载 SessionEntry 记录；ExportJsonl 与 ImportJsonl 委托会话导出/导入实现；MUST NOT 交付 stub 或仅字符串实现。

  @req:r1700 @human
  场景: dispatch 归属
    - 会话操作的执行语义 MUST 进入同一产品分发。订阅 MUST 为产品 JSON-RPC 订阅（session 与 last_seq）。审批与问卷 MUST 登记为产品 unary；host MUST 先以下行 JSON-RPC notification 告知，客户端再以 unary 作答。MUST NOT 另开 respond HTTP 路径，MUST NOT 用非 JSON-RPC 的 WS 应用帧作答。

  @req:r1691 @human
  场景: steer 与 followup 命令
    - protocol::Command MUST 包含 Steer、FollowUp 与 ClearQueue（或语义等价变体）；client 经线协议入队或清队列 MUST 反序列化为这些变体。

  @req:r1692 @human
  场景: dispatch 队列命令
    - app::core::dispatch MUST 将 Steer、FollowUp、ClearQueue 路由到 Driver 对应方法；这些变体 MUST NOT 留在仅 WS 层处理。

  @req:r1694 @human
  场景: 线协议不镜像厂商事件
    - protocol::Event MUST 映射领域 XyEvent 闭集；MUST NOT 为 OpenAI/Anthropic 等厂商专属事件增加平行变体。无法表达的细节 MUST 留在领域 Message 载荷或被省略，而非拓宽线协议宽表。

  @req:r1719 @human
  场景: 线协议映射队列与闭集
    - protocol Event MUST 能表达跨面所需的 XyEvent 闭集子集（至少含 QueueUpdate）；未映射变体 MUST 可降级忽略，MUST NOT panic。

  @req:r1711 @human
  场景: agent-part-tagged-wire
    - AgentPart 序列化到 JSONL/session message.content 时 MUST 使用带 type 判别的自描述形态（对齐 pi ThinkingContent/TextContent/ToolCall）：thinking MUST 为 {type:thinking, thinking, thinkingSignature?, redacted?}；text MUST 为 {type:text, text}；toolCall MUST 为 {type:toolCall, id, name, arguments}。MUST NOT 使用 serde untagged；MUST NOT 将 Text 写成裸字符串；MUST NOT 将 Thinking 写成无 type 且字段名为 text 的对象。

  @req:r1712 @human
  场景: preview-text-excludes-thinking
    - 用于 editor 预填与树摘要的 message 纯文本提取（message_text 或等价）MUST 只聚合 type=text 的正文；MUST NOT 把 type=thinking 的正文拼进预填/摘要。

  @req:r1707 @human
  场景: xy-event-error-kind
    - XyEvent::Error MUST 携带结构化 XyEventError（kind + message）；kind 在源自 XyError 时 MUST 对齐 XyError::kind（Aborted/Provider/Session/Config/Tool/…），裸字符串 MUST 归类为 Message（message==aborted 除外，归 Aborted）。wire Event::Error MUST 可选携带 kind；有 kind 时往返 MUST 保留；缺省反序列化 MUST 不 panic（可回落 Message 或由 message 推断 Aborted）。

  @req:r1708 @human
  场景: error-kind-at-source
    - 应用缝与 XyEvent::Error 对会话持久化、导出/导入、trust 持久化失败 MUST 给出稳定 kind（至少能区分缺失、IO、校验、不支持），MUST NOT 仅凭错误文案子串猜测这些域的分类。用户可见 message MUST 不把同一语义前缀叠两次。真正无结构的提示 MAY 使用 Message kind。

  @req:r1720 @human
  场景: 工具起始参数往返
    - wire Event 的 ToolStart MUST 保留领域工具起始事件中的工具参数；经 JSON 序列化与反序列化往返后，客户端 MUST 能从同一字段生成与本地路径一致的人类可读工具摘要，MUST NOT 无故退化为 `Read ...` 等路径占位。

  @req:r1718 @human
  场景: todo-updated-wire
    - 领域事件 TodoUpdated（完整 TodoList 快照载荷）MUST 能经 wire Event 表达，使远程 attach 端的 live checklist 刷新 MUST NOT 依赖解析工具结果文本；未映射该变体的端 MUST 可降级忽略，MUST NOT panic。该事件 MUST NOT 作为冷回放 tape 重画（resume 侧 checklist 由 SSOT 快照重建）。

  @req:r1714 @human
  场景: 会话能力方法表
    - 产品方法表 MUST 登记并由 Host 暴露会话树读取与 travel、entry label、会话列表、会话条目读取、新建会话、会话名称读写与删除能力；这些能力 MUST 使用产品 unary，不得退回未登记的 REST 产品动词。

  @req:r1715 @human
  场景: Host 资源方法
    - 产品方法表 MUST 登记 Host 级 `reload` 与只读 `loaded_resources` 能力；`reload` MUST 作用于 Host 共享的 skills、MCP 与 prompt 资源，`loaded_resources` MUST 返回当前资源快照且 MUST 含真实 MCP 连接态。Remote 客户端 MUST NOT 将已登记方法静默降级为空快照、从未连接的假完成（仅 configured、connected 恒 0 且无诊断）或 no-op。

  @req:r1716 @human
  场景: 队列深度方法
    - 产品方法表 MUST 登记只读 `queue_stats`（steer/follow-up 深度）；MUST 为产品 unary 且不占写者。Remote 客户端 MUST NOT 将未实现当成恒空深度。

  @req:r1710 @human
  场景: ToolEnd 失败标记下行
    - wire 的 ToolEnd 事件 MUST 携带 is_error 终态失败标记，该字段线上必填（缺失载荷 MUST 解析失败）；attach 客户端据此呈现失败态。

  @req:r1721 @human
  场景: CompactionEnd 载荷下行
    - wire 的 CompactionEnd 事件 MUST 与 XyEvent::CompactionEnd 同构携带 result、aborted、reason、will_retry、error_message、summary、tokens_before、tokens_after、notice 全部载荷；经 JSON 序列化与反序列化往返 MUST 保真，反序列化侧 MUST NOT 把丢载荷重建为伪成功完成态；旧的无载荷形态 MUST 可解码为缺省载荷（None/false/空）且 MUST NOT panic。attach 客户端据此呈现真实 Compacted from N（或含 tokens_after 的 N → M）tokens 与可展开 summary，notice 携带一次性诊断时以滚动提示呈现。

  @req:r1717 @human
  场景: 上下文估计方法
    - 产品方法表 MUST 登记只读 `estimate_context`：host 侧以与本地 driver 同源入口计算 ContextTokenEstimate（含固定请求开销折算与 host tokenizer 映射）；MUST 为产品 unary 且不占写者。Remote 客户端 MUST NOT 再以 GetMessages 拉条目在本地自估充当该能力。

  @req:r1701 @human
  场景: 单一产品真源
    - client 与 host 之间的产品消息 MUST 且仅 MUST 经 JSON-RPC 2.0 形状信封投递。Command 与 Event 闭集 MUST 作为方法载荷 / 下行帧内容，MUST NOT 再作为产品协议外层。测试用进程内客户端与产品 attach 客户端 MUST 使用同一方法表。MUST NOT 为远程再开平行的 REST 产品动词或第二套词表。

  @req:r1709 @human
  场景: JSON-RPC 通道
    - 产品信封 MUST 为 JSON-RPC 2.0。带 id 的请求应答 MUST 回显同一 id。网络产品入口 MUST 为 POST /rpc 与 WS /rpc（同一方法表）。WS /rpc MUST 只承载 JSON-RPC 帧（含客户端 unary 与下行 notification），MUST NOT 收非 JSON-RPC 应用帧。产品 TUI MUST NOT 用 SSE 当下行真源。MUST NOT 再以四象限 type tag、POST /api/respond 或 GET /api/events.mux 为产品真源。

  @req:r1713 @human
  场景: 一份方法表
    - 已承诺的会话操作 MUST 出现在一份方法表（方法名 + 载荷 + 返回）。关闭 TUI MUST NOT 停 Host。审批与问卷 MUST 登记为 unary。下行生命周期 MUST 以 JSON-RPC notification 携带既有 Event，MUST NOT 为每个增量另开方法名，MUST NOT 要求客户端对事件帧作答。未登记方法 MUST 以 JSON-RPC -32601 失败。

  @req:r1702 @human
  场景: 面本地不进协议
    - 剪贴板（含 OSC 52）、TTY、本机编辑器、键位与绘制 MUST NOT 成为 Command 或 Event 变体。这些能力 MUST 留在 client 面本地。

  @req:r1703 @human
  场景: bang 与工具分流
    - 人发起的工作区 bash MUST 走独立于模型工具的事件路径，MUST NOT 并进 ToolStart / ToolExecutionUpdate / ToolEnd。结束态 MUST 能用 BashResult（或语义等价）表达。直播增量若尚未成为 Event 变体，MUST NOT 改走工具流或 REST 冒充。

  @req:r1704 @human
  场景: 订阅与握手版本
    - 每条线协议消息 MUST 归属某个 session。客户端 MUST 能以 session 与 last_seq 订阅并续传；journal 截断超过 last_seq 时 MUST 发出 resync。握手 MUST 经已登记方法（host.describe 或语义等价）的 result 携带协议版本；对不上 MUST 断开且 MUST NOT 降级。MUST NOT 再以特例首帧并行维护第二套版本协商。

  @req:r1705 @human
  场景: 闭集缺口禁旁路
    - 协议闭集 MUST 能表达：host 侧重装（MCP / prompt / 技能）、项目信任持久化、session 写者与只读、导出回传内容、人 bash 直播增量。上述语义在尚未进入 Command/Event 枚举前，MUST NOT 用新的 REST 产品动词或第二套远程专用词表冒充；MUST NOT 要求本 requirement 单独新增运行时枚举变体。

  @req:r1706 @human
  场景: 审批问卷首应答
    - host 向 client 发起的审批与问卷 MUST 能经线协议往返（下行 notification + 客户端 unary）；同一 call 的第一 unary 应答 MUST 生效，后续 MUST 忽略。

  @req:r1691 @executable
  场景: command-queue-variants-parse
    当 解析队列命令 steer、follow_up、clear_queue 的线协议 JSON
    那么 分别得到 Steer、FollowUp 与 ClearQueue 变体
  @req:r1702 @executable
  场景: command-closed-set-rejects-local-surface
    当 解析面本地能力冒充的命令（clipboard_write / osc52_put / set_keybinding）
    那么 命令闭集拒绝且不产生任何变体
  @req:r1698 @executable
  场景: agent-streaming-events-roundtrip
    当 对 Agent 流族事件做线协议序列化与反序列化往返
    那么 流族事件保真且 thinking_delta 可投影为 XyEvent
  @req:r1720 @executable
  场景: tool-start-args-survive-wire
    当 对携带工具参数的 tool_start 做线协议往返
    那么 工具参数在往返后保持原字段
  @req:r1710 @executable
  场景: tool-end-is-error-flag
    当 序列化 is_error 失败标记的 tool_end 并构造缺省旧载荷
    那么 完整载荷保真失败态且缺失标记的载荷被拒绝
  @req:r1709 @executable
  场景: jsonrpc-envelope-shape
    当 解析 JSON-RPC 信封样例（request / result / notification）
    那么 JSON-RPC 形态与 id 回显成立
  @req:r1709 @executable
  场景: ws-jsonrpc-unary-peer
    当 服务端在空闲端口上启动
    并且 客户端经 WS /rpc 发送 host.describe JSON-RPC 请求
    那么 同一条 WS 收回显 id 的 JSON-RPC result
  @req:r1696 @executable
  场景: unary-stable-error-envelope
    当 服务端在空闲端口上启动
    并且 POST /rpc 调用未登记方法 no_such_method
    那么 应答为 JSON-RPC 错误且产品码在 data.code、信封数字码为 -32601
  @req:r1713 @executable
  场景: method-table-unknown-is-32601
    当 服务端在空闲端口上启动
    并且 POST /rpc 调用未登记方法 no_such_method
    那么 应答为 JSON-RPC 错误且产品码在 data.code、信封数字码为 -32601
  @req:r1701 @executable
  场景: jsonrpc-unary-success-shape
    当 服务端在空闲端口上启动
    并且 POST /rpc 调用 host.describe
    那么 应答为 JSON-RPC 成功且 id 回显
  @req:r1704 @executable
  场景: handshake-protocol-via-describe
    当 服务端在空闲端口上启动
    并且 POST /rpc 调用 host.describe
    那么 result 携带协议版本整数
  @req:r1697 @executable
  场景: subscribe-via-jsonrpc
    当 服务端在空闲端口上启动
    并且 POST /rpc 调用 subscribe 带 session_id=s0 与 last_seq=5
    那么 应答为 JSON-RPC 成功且 result 含 session 与 seq
  @req:r1700 @executable
  场景: approve-tool-is-product-unary
    当 服务端在空闲端口上启动
    并且 POST /rpc 调用 approve_tool
    那么 应答不是 JSON-RPC -32601
  @req:r1709 @executable
  场景: jsonrpc-illegal-envelope
    当 服务端在空闲端口上启动
    并且 POST /rpc 发送非法信封
    那么 HTTP 失败且无 JSON-RPC result 成功
  @req:r1716 @executable
  场景: queue-stats-method-table-readonly
    当 服务端在空闲端口上启动
    并且 POST /rpc 查询只读方法 queue_stats
    那么 返回 steer 与 follow-up 队列深度
    并且 响应不携带写者租约 token
  @req:r1706 @executable
  场景: reverse-rpc-first-answer-effective
    假如 工具需审批
    当 推送 ApprovalRequired
    那么 客户端经 POST /rpc 调用 approve_tool 且回合恢复
