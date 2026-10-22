# language: zh-CN
# purpose: Client↔host 产品真源为 JSON-RPC 2.0 文本信封（入口 POST /rpc 与 WS /rpc）；Command/Event 为方法载荷与下行通知内容。

功能: protocol-app

  @req:r1693
  规则: Command 枚举
    protocol 须定义 Command 枚举，至少覆盖 Run、Cancel、SwitchModel、SetThinking、ListCommands、ApproveTool、AnswerQuestion、SwitchSession、GetMessages、ExportHtml、ExportJsonl、ImportJsonl；command-backed 方法 须反序列化为此枚举。方法 须按方法名寻址，不得引入数字 method_id。

    场景: 命令往返保真
      当 对产品命令样例做线协议序列化与反序列化往返
      那么 命令往返保真且逐变体可反序列化
  @req:r1695
  规则: Event 枚举
    protocol/ 须定义 Event 枚举，至少覆盖 TurnStart、Delta、ToolCall、ApprovalRequired、QuestionRequired、TurnEnd、Usage；每个 core→client 消息 须反序列化为此枚举。Event 须覆盖全部 XyEvent 变体（见 r1698）；未知 type 须降级为 Unknown 变体。

    场景: 事件枚举覆盖agent流
      当 对 Agent 流族事件做线协议序列化与反序列化往返
      那么 流族事件保真且 thinking_delta 可投影为 XyEvent

    场景: 未知事件变体降级
      当 对未知 type 的线协议事件做反序列化
      那么 落入 Unknown 变体且不 panic
  @req:r1696
  规则: 信封与错误
    产品 unary 应答 必须为 JSON-RPC 2.0 形态：成功则带 result，失败则带 error（error.message 为 details）。产品业务码 必须为稳定字符串，置于 error.data.code；载体数字码 仅作 JSON-RPC 错误码，不得成为产品错误模型。合法 JSON-RPC 的 HTTP 状态 表示载体成功（含业务错误）；非法信封 必须失败。上行 notification 不执行、不应答；产品方法 必须为 request。旧 REST {code,msg,data}、四象限 `{ok,value,error}` 顶层形态 不得再作为产品路径。

    场景: 一元稳定错误信封
      当 服务端在空闲端口上启动
      并且 POST /rpc 调用未登记方法 no_such_method
      那么 应答为产品错误且为未登记方法
  @req:r1697
  规则: Subscribe 命令
    产品 必须支持以 session_id 与 last_seq 订阅事件流；订阅确认 必须以 JSON-RPC result 表达 session_id、seq 与 inflight。该订阅 必须走产品入口（POST /rpc 或 WS /rpc 的 JSON-RPC 文本），不得另开 REST 或二进制帧。

    场景: 订阅经RPC
      当 服务端在空闲端口上启动
      并且 POST /rpc 调用 subscribe 带 session_id=s0 与 last_seq=5
      那么 应答为产品成功且 result 含 session 与 seq
  @req:r1698
  规则: Event 变体完整
    protocol::Event 须覆盖全部 XyEvent 变体：AgentStart、AgentEnd、TurnStart、TurnEnd、MessageStart、MessageEnd、MessageUpdate、ToolExecutionStart、ToolExecutionUpdate、ToolExecutionEnd、CompactionStart、CompactionEnd、ContextTokenSettlement、ModelSelect、ThinkingLevelChanged、QueueUpdate、TodoUpdated、AutoRetryStart、AutoRetryEnd、Error、SessionInfoChanged、TextDelta、ThinkingDelta；ThinkingDelta 须往返 XyEvent::ThinkingDelta 且不得降级为空 MessageUpdate；XyEvent ↔ Event 的映射 必须唯一且位于 xylitol-wire。`AgentEnd` 的 `messages` 为面本地保留字段，线上不携带；跨进程客户端 须以 `get_messages` 重建 transcript。

    场景: agent流式事件往返
      当 对 Agent 流族事件做线协议序列化与反序列化往返
      那么 流族事件保真且 thinking_delta 可投影为 XyEvent
  @req:r1699
  规则: 会话命令分发
    RPC dispatch 须实现 SwitchSession：经 SessionStore 校验目标会话存在并切换上下文；GetMessages 返回已加载 SessionEntry 记录；ExportJsonl 与 ImportJsonl 委托会话导出/导入实现；不得交付 stub 或仅字符串实现。

    场景: 切换会话校验目标
      当 服务端在空闲端口上启动
      并且 POST /rpc 调用 switch_session 指向不存在会话
      那么 应答为产品错误且错误提及会话不存在或无效

    场景: 读取消息返回条目
      假如 向 POST /rpc 发送 prompt 请求
      当 server 处理 prompt
      并且 POST /rpc 调用 get_messages
      那么 应答含已加载会话条目
  @req:r1700
  规则: dispatch 归属
    会话操作的执行语义 必须进入同一产品分发。订阅 必须为产品订阅（session_id 与 last_seq）。审批与问卷 必须登记为产品 unary；host 须先以下行通知告知，客户端再以 unary 作答。不得另开 respond HTTP 路径，不得以 JSON-RPC 之外的帧作答。

    场景: 批准工具为产品一元
      当 服务端在空闲端口上启动
      并且 POST /rpc 调用 approve_tool
      那么 应答不是未登记方法错误
  @req:r1691
  规则: steer 与 followup 命令
    protocol::Command 须包含 Steer、FollowUp 与 ClearQueue（或语义等价变体）；client 经线协议入队或清队列 须反序列化为这些变体。

    场景: 命令队列变体解析
      当 解析队列命令 steer、follow_up、clear_queue 的线协议 JSON
      那么 分别得到 Steer、FollowUp 与 ClearQueue 变体
  @req:r1692
  规则: dispatch 队列命令
    app::core::dispatch 须将 Steer、FollowUp、ClearQueue 路由到 Driver 对应方法；这些变体 不得留在仅 WS 层处理。

    场景: 队列命令路由经一元
      当 服务端在空闲端口上启动
      并且 POST /rpc 调用 steer 携带文本
      并且 POST /rpc 查询会话 "s-prompt" 的只读 queue_stats
      那么 steer 深度至少为 1
  @req:r1694
  规则: 线协议不镜像厂商事件
    protocol::Event 须映射领域 XyEvent 闭集；不得为 OpenAI/Anthropic 等厂商专属事件增加平行变体。无法表达的细节 须留在领域 Message 载荷或被省略，而非拓宽线协议宽表。

    场景: 无厂商专用事件变体
      当 对 Agent 流族事件做线协议序列化与反序列化往返
      那么 流族事件保真且 thinking_delta 可投影为 XyEvent

    场景: 闭合设置拒绝表面本地命令名称
      当 解析面本地能力冒充的命令（clipboard_write / osc52_put / set_keybinding）
      那么 命令闭集拒绝且不产生任何变体
  @req:r1719
  规则: 线协议映射队列与闭集
    protocol Event 须能表达跨面所需的 XyEvent 闭集子集（至少含 QueueUpdate）；未映射变体 须可降级忽略，不得 panic。

    场景: 队列更新线协议往返
      当 对 QueueUpdate 事件做线协议序列化与反序列化往返
      那么 队列计数保真且未知 type 落入 Unknown 而非 panic
  @req:r1711
  规则: agent-part-tagged-wire
    AgentPart 序列化到 JSONL/session message.content 时 须使用带 type 判别的自描述形态（对齐 pi ThinkingContent/TextContent/ToolCall）：thinking 须为 {type:thinking, thinking, thinkingSignature?, redacted?}；text 须为 {type:text, text}；toolCall 须为 {type:toolCall, id, name, arguments}。不得使用 serde untagged；不得将 Text 写成裸字符串；不得将 Thinking 写成无 type 且字段名为 text 的对象。

    场景: agent部件带标签内容线协议
      当 序列化含 thinking 与 text 的 assistant content 为会话部件
      那么 每个部件带 type 判别且无裸字符串 content
  @req:r1712
  规则: preview-text-excludes-thinking
    用于 editor 预填与树摘要的 message 纯文本提取（message_text 或等价）须只聚合 type=text 的正文；不得把 type=thinking 的正文拼进预填/摘要。

    场景: 预览文本排除思考
      当 对含 thinking 与 text 的消息提取纯文本摘要
      那么 摘要仅含 text 正文
  @req:r1707
  规则: xy-event-error-kind
    XyEvent::Error 须携带结构化 XyEventError（kind + message）；kind 在源自 XyError 时 须对齐 XyError::kind（Aborted/Provider/Session/Config/Tool/…），裸字符串 须归类为 Message（message==aborted 除外，归 Aborted）。wire Event::Error 须可选携带 kind；有 kind 时往返 须保留；缺省反序列化 须不 panic（可回落 Message 或由 message 推断 Aborted）。

    场景: 错误种类线协议往返
      当 序列化携带 kind 的 error 事件
      那么 kind 往返保真
  @req:r1708
  规则: error-kind-at-source
    应用缝与 XyEvent::Error 对会话持久化、导出/导入、trust 持久化失败 须给出稳定 kind（至少能区分缺失、IO、校验、不支持），不得仅凭错误文案子串猜测这些域的分类。用户可见 message 须不把同一语义前缀叠两次。真正无结构的提示 MAY 使用 Message kind。

    场景: 稳定错误种类在来源
      当 以不存在的文件调用导入会话
      那么 错误携带稳定 kind 且非文案猜测
  @req:r1720
  规则: 工具起始参数往返
    wire Event 的 tool_execution_start 须保留领域工具起始事件中的工具参数；经 JSON 序列化与反序列化往返后，客户端 须能从同一字段生成与本地路径一致的人类可读工具摘要，不得无故退化为 `Read ...` 等路径占位。

    场景: 工具启动参数存活线协议
      当 对携带工具参数的 tool_execution_start 做线协议往返
      那么 工具参数在往返后保持原字段
  @req:r1718
  规则: todo-updated-wire
    领域事件 TodoUpdated（完整 TodoList 快照载荷）须能经 wire Event 表达，使远程 attach 端的 live checklist 刷新 不得依赖解析工具结果文本；未映射该变体的端 须可降级忽略，不得 panic。该事件 不得作为冷回放 tape 重画（resume 侧 checklist 由 SSOT 快照重建）。

    场景: 待办已更新线协议往返
      当 对 TodoUpdated 快照事件做线协议往返
      那么 清单快照载荷保真
  @req:r1714
  规则: 会话能力方法表
    产品方法表 须登记并由 Host 暴露会话树读取与 travel、entry label、会话列表、会话条目读取、新建会话、会话名称读写与删除能力；这些能力 须使用产品 unary，不得退回未登记的 REST 产品动词。

    场景: 会话能力方法已登记
      当 查询方法表的会话能力方法
      那么 会话树、travel、label、列表、条目读取、新建、名称读写与删除均已登记
  @req:r1715
  规则: Host 资源方法
    产品方法表 须登记 Host 级 `reload` 与只读 `loaded_resources` 能力；`reload` 须作用于 Host 共享的 skills、MCP 与 prompt 资源，`loaded_resources` 须返回当前资源快照且 须含真实 MCP 连接态。Remote 客户端 不得将已登记方法静默降级为空快照、从未连接的假完成（仅 configured、connected 恒 0 且无诊断）或 no-op。

    场景: Host资源方法已登记
      当 查询方法表的 Host 资源方法
      那么 reload 与 loaded_resources 均已登记
  @req:r1716
  规则: 队列深度方法
    产品方法表 须登记只读 `queue_stats`（steer/follow-up 深度）；须为产品 unary 且不占会话写锁。Remote 客户端 不得将未实现当成恒空深度。

    场景: 队列统计方法表只读
      当 服务端在空闲端口上启动
      并且 POST /rpc 查询只读方法 queue_stats
      那么 返回 steer 与 follow-up 队列深度
  @req:r1710
  规则: ToolEnd 失败标记下行
    wire 的 tool_execution_end 事件 须携带 is_error 终态失败标记，该字段线上必填（缺失载荷 须解析失败）；attach 客户端据此呈现失败态。

    场景: 工具结束为错误开关
      当 序列化 is_error 失败标记的 tool_execution_end 并构造缺省旧载荷
      那么 完整载荷保真失败态且缺失标记的载荷被拒绝
  @req:r1721
  规则: CompactionEnd 载荷下行
    wire 的 CompactionEnd 事件 须与 XyEvent::CompactionEnd 同构携带 result、aborted、reason、will_retry、error_message、summary、tokens_before、tokens_after、notice 全部载荷；经 JSON 序列化与反序列化往返 须保真，反序列化侧 不得把丢载荷重建为伪成功完成态；旧的无载荷形态 须可解码为缺省载荷（None/false/空）且 不得 panic。attach 客户端据此呈现真实 Compacted from N（或含 tokens_after 的 N → M）tokens 与可展开 summary，notice 携带一次性诊断时以滚动提示呈现。

    场景: 压缩结束载荷往返
      当 序列化全载荷 CompactionEnd 并构造旧无载荷形态
      那么 全载荷保真且旧形态解码为缺省载荷
  @req:r1717
  规则: 上下文估计方法
    产品方法表 须登记只读 `estimate_context`：host 侧以与本地 driver 同源入口计算 ContextTokenEstimate（含固定请求开销折算）；须为产品 unary 且不占写者。Remote 客户端 不得再以 GetMessages 拉条目在本地自估充当该能力。

    场景: 估算上下文方法已登记只读
      当 查询方法表的 estimate_context
      那么 已登记且为只读 unary 不占写者
  @req:r1701
  规则: 单一产品真源
    client 与 host 之间的产品消息 必须且仅须经 JSON-RPC 2.0 文本信封投递。Command 与 Event 闭集 必须作为 params、result 与下行 notification 载荷，不得作为产品协议外层。测试用进程内客户端与产品 attach 客户端 必须使用同一方法表。不得为远程再开平行的 REST 产品动词、二进制信封或第二套词表。

    场景: 一元成功形状
      当 服务端在空闲端口上启动
      并且 POST /rpc 调用 host.describe
      那么 应答为产品成功且 id 回显
  @req:r1709
  规则: 产品通道
    产品信封 必须为 JSON-RPC 2.0 文本（request、notification、response、batch）。请求应答 必须原样回显 request 的 id。网络产品入口 必须为 POST /rpc 与 WS /rpc（同一方法表）。WS /rpc 的下行 notification 与上行 request 均为 text 帧内的 JSON-RPC 文本。产品 TUI 不得用 SSE 当下行真源。不得再以四象限 type tag、POST /api/respond 或 GET /api/events.mux 为产品真源。

    场景: JSON-RPC 文本信封受理
      当 服务端在空闲端口上启动
      并且 POST /rpc 发送 JSON-RPC 2.0 文本
      那么 HTTP 200 且应答为 JSON-RPC 文本 response

    场景: 非法信封
      当 服务端在空闲端口上启动
      并且 POST /rpc 发送非法信封
      那么 HTTP 失败且无成功业务应答
  @req:r1713
  规则: 一份方法表
    已承诺的会话操作 必须出现在一份方法表（方法名 + 载荷 + 返回）。关闭 TUI 不得停 Host。审批与问卷 必须登记为 unary。下行生命周期 必须以产品通知携带既有 Event，不得为每个增量另开方法名，不得要求客户端对事件 notification 作答。未登记方法 必须以产品未登记方法错误失败。

    场景: 方法表未知为32601
      当 服务端在空闲端口上启动
      并且 POST /rpc 调用未登记方法 no_such_method
      那么 应答为产品错误且为未登记方法
  @req:r1702
  规则: 面本地不进协议
    剪贴板（含 OSC 52）、TTY、本机编辑器、键位与绘制 不得成为 Command 或 Event 变体。这些能力 须留在 client 面本地。

    场景: 命令闭合设置拒绝本地表面
      当 解析面本地能力冒充的命令（clipboard_write / osc52_put / set_keybinding）
      那么 命令闭集拒绝且不产生任何变体
  @req:r1703
  规则: bang 与工具分流
    人发起的工作区 bash 须走独立于模型工具的事件路径，不得并进 ToolExecutionStart / ToolExecutionUpdate / ToolExecutionEnd。结束态 须能用 BashResult（或语义等价）表达。直播增量 必须以 bash/output 下行通知承载，不得改走工具流或 REST 冒充。

    场景: bang运行之外agent工具流
      当 以主机泵在 idle 提交 bang 命令
      那么 execute_bash 收到命令体且未调用 run

    场景: bash终态状态于单一块
      当 以主机泵提交 bang 命令并注入分段输出与成功结果
      那么 命令进入单一 Bash 块且输出在同一块内且状态轨为成功
  @req:r1704
  规则: 握手与版本
    每条线协议消息 必须归属某个 session。握手 必须经已登记方法 host.describe 的 result 携带 protocol 整数与 formats（含 jsonrpc）；客户端 对 protocol 或 formats 不匹配 必须断开且不得降级。客户端 须能以 session_id 与 last_seq 订阅并续传；journal 截断超过 last_seq 时 须发出 resync。不得以特例首帧或环境变量并行维护第二套版本协商。

    场景: 握手协议经描述
      当 服务端在空闲端口上启动
      并且 POST /rpc 调用 host.describe
      那么 result 携带协议版本整数
  @req:r1705
  规则: 闭集缺口禁旁路
    协议闭集 须能表达：host 侧重装（MCP / prompt / 技能）、项目信任持久化、session 写者与只读、导出回传内容、人 bash 直播增量。上述语义在尚未进入 Command/Event 枚举前，不得用新的 REST 产品动词或第二套远程专用词表冒充；不得要求本 requirement 单独新增运行时枚举变体。

    场景: Host语义时单一方法表
      假如 RemoteDriver 指向该 server
      当 调用已登记的 session 能力 unary
      那么 经产品 unary 到达 Host 且不经 REST 冒充

    场景: 无第二次词表供表面本地项
      当 解析面本地能力冒充的命令（clipboard_write / osc52_put / set_keybinding）
      那么 命令闭集拒绝且不产生任何变体
  @req:r1706
  规则: 审批问卷首应答
    host 向 client 发起的审批与问卷 须能经线协议往返（下行 notification + 客户端 unary）；同一 call 的第一 unary 应答 须生效，后续 须忽略。

    场景: 反向RPC首次作答生效
      假如 工具需审批
      当 推送 ApprovalRequired
      那么 客户端经 POST /rpc 调用 approve_tool 且回合恢复
  @req:r1927
  规则: 线上 JSON 深解析有界
    对线上输入的 JSON 深解析 必须设显式深度上限（2048 层：容纳实机深树两个量级、远低于栈风险区），超限 必须按降级语义收口（服务端判非法信封并返回 -32700，客户端 将结果降级为 null 而不断链），不得以无界递归深度解析线上输入（4 MiB 体量上限内的嵌套字节数组仍可打穿 worker 栈）。上限 必须为单一常量（两处解码入口共用）。

    场景: JSON深度上限信封解码
      当 对超过深度上限的线上 JSON 信封做解码
      那么 深解析有界早退且超限按降级语义拒绝

    场景: JSON深度上限放行树规模
      当 对实机深树量级的合法线上信封做解码
      那么 深度上限不误伤常规深树

  @req:r2917
  规则: JSON-RPC 信封形状
    产品线协议 必须遵循 JSON-RPC 2.0：request 必须带 jsonrpc 为 "2.0"、id（string 或 integer）与 method；id 必须原样回显到对应 response；notification 无 id 且 不得产生应答；batch 必须逐条处理并按序返回 response，batch 中的 notification 不产生应答；空数组 batch 必须判非法。batch 条目数 不得超过 100；超限 必须判非法请求（-32600，data.code 为 batch_too_large）。params 为 object 或 array；产品方法 必须只接受 object。

    场景: JSON-RPC请求应答往返
      当 服务端在空闲端口上启动
      并且 POST /rpc 发送 host.describe 的 JSON-RPC 请求且 id 为 1
      那么 应答为 JSON-RPC 成功 response 且 id 回显为 1

    场景: JSON-RPC通知无应答
      当 服务端在空闲端口上启动
      并且 POST /rpc 发送无 id 的 host.describe notification
      那么 HTTP 200 且 body 为空

    场景: 批量请求逐条应答
      当 服务端在空闲端口上启动
      并且 POST /rpc 发送含两个 request 与一个 notification 的 batch
      那么 返回两个 response 且 id 对应、notification 无应答

    场景: 空批量判非法
      当 服务端在空闲端口上启动
      并且 POST /rpc 发送空数组 batch
      那么 HTTP 400 且 error.code 为 -32600

    场景: 解析错误码
      当 服务端在空闲端口上启动
      并且 POST /rpc 发送非法 JSON
      那么 HTTP 400 且 error.code 为 -32700

    场景: 非法请求码
      当 服务端在空闲端口上启动
      并且 POST /rpc 发送缺少 method 的 JSON-RPC 对象
      那么 HTTP 400 且 error.code 为 -32600
  @req:r2918
  规则: 错误码与业务码
    JSON-RPC 错误码 必须区分标准类别与业务错误：标准类别错误 必须映射到标准码——invalid_input 为 -32602、unregistered_method 为 -32601、internal_error 为 -32603（解析错误 -32700 与非法请求 -32600 同为标准码）；其余产品业务错误 必须使用 -32000 至 -32099 区间的 error.code，并在 error.data.code 中给出稳定字符串（如 session_busy、writer_conflict、not_found）。HTTP 状态 必须为：合法 JSON-RPC（含业务错误）为 200；解析或非法信封为 400；body 超过 4 MiB 为 413。

    场景: 参数错误码
      当 服务端在空闲端口上启动
      并且 POST /rpc 调用 set_model 且缺少 model_id
      那么 error.code 为 -32602 且 data.code 为 invalid_input

    场景: 业务错误带稳定码
      当 服务端在空闲端口上启动
      并且 POST /rpc 调用 switch_session 指向不存在会话
      那么 error.code 为 -32000 且 data.code 为 not_found

    场景: 请求体超限判 413
      当 服务端在空闲端口上启动
      并且 POST /rpc 发送超过 4 MiB 的正文
      那么 HTTP 413 且 error.data.code 为 payload_too_large
  @req:r2919
  规则: 方法名寻址与预留命名空间
    方法 必须按方法名寻址，未登记方法 必须返回 -32601 且 error.data.code 为 unregistered_method。命名空间 plugin/* 与 plugin/call 仅为预留前缀，当前 未登记，调用 必须返回 -32601；本次不为插件扩展 Event 闭集。

    场景: plugin前缀预留未登记
      当 服务端在空闲端口上启动
      并且 POST /rpc 调用 plugin/call
      那么 error.code 为 -32601 且 data.code 为 unregistered_method
