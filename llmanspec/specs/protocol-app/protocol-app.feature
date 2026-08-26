# language: zh-CN
# capability: protocol-app
# purpose: Client↔host 产品真源为四象限信封；Command/Event 为方法/帧载荷；禁 JSON-RPC 2.0 与全双工 WS 外层。
# scope: 主 crate, workspace 测试

功能: protocol-app

  @manual @req:ip1 @human
  场景: Command 枚举
    - protocol MUST 定义 Command 枚举，至少覆盖 Run、Cancel、SwitchModel、SetThinking、ListCommands、ApproveTool、AnswerQuestion、SwitchSession、GetMessages、ExportHtml、ExportJsonl、ImportJsonl；每个 client→core 消息 MUST 反序列化为此枚举。

  @req:ip2 @human @manual
  场景: Event 枚举
    - protocol/ MUST 定义 Event 枚举，至少覆盖 TurnStart、Delta、ToolCall、ApprovalRequired、QuestionRequired、TurnEnd、Usage；每个 core→client 消息 MUST 反序列化为此枚举。

  @req:ip3 @human
  场景: 信封与错误
    - 产品 unary 应答 MUST 使用结果形态：成功则 ok 为真并带 value，失败则 ok 为假并带稳定 code 与 details。合法信封的 HTTP 状态 MUST 表示载体成功；非法信封 MUST 失败。MUST NOT 以 JSON-RPC 2.0 数字码为产品错误模型。旧 REST {code,msg,data} 形态 MUST NOT 再作为产品 TUI 路径。

  @req:ip6 @human @manual
  场景: Subscribe 命令
    - protocol/ MUST 定义 Command::Subscribe {session_id, last_seq} 用于 WS 订阅，Event::Subscribed {session_id, seq} 作为确认。

  @req:ip7 @human
  场景: Event 变体完整
    - protocol::Event MUST 覆盖全部 AgentEvent 变体：TurnStart、TurnEnd、MessageStart、MessageEnd、MessageUpdate、ToolExecutionUpdate、CompactionEnd、TextDelta、ThinkingDelta；ThinkingDelta MUST 往返 XyEvent::ThinkingDelta 且不得降级为空 MessageUpdate。

  @req:ip8 @human @manual
  场景: 会话命令分发
    - RPC dispatch MUST 实现 SwitchSession：经 SessionStore 校验目标会话存在并切换上下文；GetMessages 返回已加载 SessionEntry 记录；ExportJsonl 与 ImportJsonl 委托会话导出/导入实现；MUST NOT 交付 stub 或仅字符串实现。

  @req:ip9 @human
  场景: dispatch 归属
    - 会话操作的执行语义 MUST 进入同一产品分发。订阅 MUST 为 unary。审批与问卷 MUST 作为对 host 可应答下行的 client-response（回显同一相关 id），MUST NOT 当作 unary，MUST NOT 留在仅全双工 WebSocket 应用层。

  @req:ip-q1 @human
  场景: steer 与 followup 命令
    - protocol::Command MUST 包含 Steer、FollowUp 与 ClearQueue（或语义等价变体）；client 经线协议入队或清队列 MUST 反序列化为这些变体。

  @req:ip-q2 @human
  场景: dispatch 队列命令
    - app::core::dispatch MUST 将 Steer、FollowUp、ClearQueue 路由到 Driver 对应方法；这些变体 MUST NOT 留在仅 WS 层处理。

  @req:ip10 @human
  场景: 线协议不镜像厂商事件
    - protocol::Event MUST 映射领域 XyEvent 闭集；MUST NOT 为 OpenAI/Anthropic 等厂商专属事件增加平行变体。无法表达的细节 MUST 留在领域 Message 载荷或被省略，而非拓宽线协议宽表。

  @req:pa-wire1 @human
  场景: 线协议映射队列与闭集
    - protocol Event MUST 能表达跨面所需的 XyEvent 闭集子集（至少含 QueueUpdate）；未映射变体 MUST 可降级忽略，MUST NOT panic。

  @req:pa-m1 @human
  场景: agent-part-tagged-wire
    - AgentPart 序列化到 JSONL/session message.content 时 MUST 使用带 type 判别的自描述形态（对齐 pi ThinkingContent/TextContent/ToolCall）：thinking MUST 为 {type:thinking, thinking, thinkingSignature?, redacted?}；text MUST 为 {type:text, text}；toolCall MUST 为 {type:toolCall, id, name, arguments}。MUST NOT 使用 serde untagged；MUST NOT 将 Text 写成裸字符串；MUST NOT 将 Thinking 写成无 type 且字段名为 text 的对象。

  @req:pa-m2 @human
  场景: preview-text-excludes-thinking
    - 用于 editor 预填与树摘要的 message 纯文本提取（message_text 或等价）MUST 只聚合 type=text 的正文；MUST NOT 把 type=thinking 的正文拼进预填/摘要。

  @req:pa-e1 @human
  场景: xy-event-error-kind
    - XyEvent::Error MUST 携带结构化 XyEventError（kind + message）；kind 在源自 XyError 时 MUST 对齐 XyError::kind（Aborted/Provider/Session/Config/Tool/…），裸字符串 MUST 归类为 Message（message==aborted 除外，归 Aborted）。wire Event::Error MUST 可选携带 kind；有 kind 时往返 MUST 保留；缺省反序列化 MUST 不 panic（可回落 Message 或由 message 推断 Aborted）。

  @req:pa-e2 @human
  场景: error-kind-at-source
    - 应用缝与 XyEvent::Error 对会话持久化、导出/导入、trust 持久化失败 MUST 给出稳定 kind（至少能区分缺失、IO、校验、不支持），MUST NOT 仅凭错误文案子串猜测这些域的分类。用户可见 message MUST 不把同一语义前缀叠两次。真正无结构的提示 MAY 使用 Message kind。

  @req:pa-wire2 @human
  场景: 工具起始参数往返
    - wire Event 的 ToolStart MUST 保留领域工具起始事件中的工具参数；经 JSON 序列化与反序列化往返后，客户端 MUST 能从同一字段生成与本地路径一致的人类可读工具摘要，MUST NOT 无故退化为 `Read ...` 等路径占位。

  @req:pa-map2 @human
  场景: 会话能力方法表
    - 产品方法表 MUST 登记并由 Host 暴露会话树读取与 travel、entry label、会话列表、会话条目读取、新建会话、会话名称读写与删除能力；这些能力 MUST 使用四象限 unary，不得退回未登记的 REST 产品动词。

  @req:pa-map3 @human
  场景: Host 资源方法
    - 产品方法表 MUST 登记 Host 级 `reload` 与只读 `loaded_resources` 能力；`reload` MUST 作用于 Host 共享的 skills、MCP 与 prompt 资源，`loaded_resources` MUST 返回当前资源快照且 MUST 含真实 MCP 连接态。Remote 客户端 MUST NOT 将已登记方法静默降级为空快照、从未连接的假完成（仅 configured、connected 恒 0 且无诊断）或 no-op。

  @req:pa-map4 @human
  场景: 队列深度方法
    - 产品方法表 MUST 登记只读 `queue_stats`（steer/follow-up 深度）；MUST 为四象限 unary 且不占写者。Remote 客户端 MUST NOT 将未实现当成恒空深度。

  @req:pa-err1 @human
  场景: ToolEnd 失败标记下行
    - wire 的 ToolEnd 事件 MUST 携带 is_error 终态失败标记，该字段线上必填（缺失载荷 MUST 解析失败）；attach 客户端据此呈现失败态。

  @req:pa-cs1 @human
  场景: 单一产品真源
    - client 与 host 之间的产品消息 MUST 且仅 MUST 经四象限信封投递。Command 与 Event 闭集 MUST 作为方法载荷 / 下行帧内容，MUST NOT 再作为产品协议外层。测试用进程内客户端与产品 attach 客户端 MUST 使用同一方法表。MUST NOT 为远程再开平行的 REST 产品动词或第二套词表。

  @req:pa-env1 @human
  场景: 四象限与通道
    - 产品信封 MUST 区分为 client-request、server-response、server-request、client-response。相关 id 由发起方铸造，应答 MUST 回显。网络 unary 与 respond MUST 经 HTTP POST；网络下行 MUST 为 WebSocket 文本帧且该套接字 MUST NOT 收业务上行。产品 TUI MUST NOT 用 SSE 当下行真源。MUST NOT 以 JSON-RPC 2.0 为产品协议。

  @req:pa-map1 @human
  场景: 一份方法表
    - 已承诺的会话操作 MUST 出现在一份方法表（unary 名 + 载荷 + 返回）。关闭 TUI MUST NOT 停 Host。审批与问卷 MUST NOT 登记为 unary。下行生命周期 MUST 能作为 server-request 载荷携带既有 Event，MUST NOT 为每个增量另开方法名。未登记方法 MUST 失败。

  @req:pa-cs2 @human
  场景: 面本地不进协议
    - 剪贴板（含 OSC 52）、TTY、本机编辑器、键位与绘制 MUST NOT 成为 Command 或 Event 变体。这些能力 MUST 留在 client 面本地。

  @req:pa-cs3 @human
  场景: bang 与工具分流
    - 人发起的工作区 bash MUST 走独立于模型工具的事件路径，MUST NOT 并进 ToolStart / ToolExecutionUpdate / ToolEnd。结束态 MUST 能用 BashResult（或语义等价）表达。直播增量若尚未成为 Event 变体，MUST NOT 改走工具流或 REST 冒充。

  @req:pa-cs4 @human
  场景: 订阅与握手版本
    - 每条线协议消息 MUST 归属某个 session。客户端 MUST 能以 session 与 last_seq 订阅并续传；journal 截断超过 last_seq 时 MUST 发出 resync。握手 MUST 携带协议版本；对不上 MUST 断开且 MUST NOT 降级。现行 ServerHello 版本字段在改名前 MUST 视为同一握手语义，MUST NOT 同时维护两套版本协商。

  @req:pa-cs5 @human @manual
  场景: 闭集缺口禁旁路
    - 协议闭集 MUST 能表达：host 侧重装（MCP / prompt / 技能）、项目信任持久化、session 写者与只读、导出回传内容、人 bash 直播增量。上述语义在尚未进入 Command/Event 枚举前，MUST NOT 用新的 REST 产品动词或第二套远程专用词表冒充；MUST NOT 要求本 requirement 单独新增运行时枚举变体。

  @req:pa-cs6 @human
  场景: 反向 RPC 首应答
    - host 向 client 发起的审批与问卷 MUST 能经线协议往返；同一 call 的第一应答 MUST 生效，后续 MUST 忽略。

  @req:ip-q1 @executable
  场景: command-queue-variants-parse
    当 解析队列命令 steer、follow_up、clear_queue 的线协议 JSON
    那么 分别得到 Steer、FollowUp 与 ClearQueue 变体
  @req:pa-cs2 @executable
  场景: command-closed-set-rejects-local-surface
    当 解析面本地能力冒充的命令（clipboard_write / osc52_put / set_keybinding）
    那么 命令闭集拒绝且不产生任何变体
  @req:ip7 @executable
  场景: agent-streaming-events-roundtrip
    当 对 Agent 流族事件做线协议序列化与反序列化往返
    那么 流族事件保真且 thinking_delta 可投影为 XyEvent
  @req:pa-wire2 @executable
  场景: tool-start-args-survive-wire
    当 对携带工具参数的 tool_start 做线协议往返
    那么 工具参数在往返后保持原字段
  @req:pa-err1 @executable
  场景: tool-end-is-error-flag
    当 序列化 is_error 失败标记的 tool_end 并构造缺省旧载荷
    那么 完整载荷保真失败态且缺失标记的载荷被拒绝
  @req:pa-env1 @executable
  场景: four-quadrant-envelope-shape
    当 解析四象限信封样例（client-request/server-response/server-request/client-response）
    那么 四象限形态与 rpcId 回显成立
  @req:ip3 @executable
  场景: unary-stable-error-envelope
    当 服务端在空闲端口上启动
    并且 调用未登记 unary 方法 no_such_method
    那么 应答为稳定错误形态且无 JSON-RPC 数字码
  @req:pa-map4 @executable
  场景: queue-stats-method-table-readonly
    当 服务端在空闲端口上启动
    并且 查询只读 unary queue_stats
    那么 返回 steer 与 follow-up 队列深度
    并且 响应不携带写者租约 token
  @req:pa-cs6 @executable
  场景: reverse-rpc-first-answer-effective
    假如 工具需审批
    当 推送 ApprovalRequired
    那么 客户端经 POST /api/respond 应答且回合恢复
