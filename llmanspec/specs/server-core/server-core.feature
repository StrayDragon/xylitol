# language: zh-CN
# capability: server-core
# purpose: Server 运行时 — 独立 Host 监听器经 JSON-RPC 2.0 暴露产品契约（POST /rpc + WS /rpc）；每 session 槽独立 journal 与写者。
# scope: src/

功能: server-core

  @req:r1794 @human
  场景: server 托管 agent
    - app::server MUST 在启动时装配 infra 运行时并注入 agent 端口；组合根 MUST 持有可按 session 槽取用的 Driver（或等价缝），供 unary 处理器调用，MUST NOT 以无槽的裸 Agent 作为唯一生产入口。

  @req:r1778 @human
  场景: 产品路径 JSON-RPC
    - 产品 TUI 与其它产品客户端 MUST 经 JSON-RPC 2.0 访问 Host：产品入口为 POST /rpc 与 WS /rpc（同一方法表）；WS /rpc MUST 只承载 JSON-RPC 帧。Host 监听器 MUST 实现该路径。现行 REST 资源动词、POST /api/respond、GET /api/events.mux 与非 JSON-RPC 的 WS 应用帧 MUST NOT 再作为产品真源。

  @req:r1796 @human
  场景: 产品 HTTP 与 WS 入口
    - Host MUST 暴露 POST /rpc 与 WS /rpc 作为产品 JSON-RPC 入口（同一方法表）。MUST 另暴露 GET /healthz 与调试 GET /openapi.json、GET /docs。MUST NOT 再以 POST /api/{method}、POST /api/respond 或 GET /api/events.mux 为产品真源。MUST NOT 再以 /api/v1 REST 资源动词或非 JSON-RPC 的 WS 应用帧承载产品命令。

  @req:r1797 @human
  场景: 绑定占用
    - Host MUST 绑定配置的地址与端口（默认 127.0.0.1:18790）。同一 {addr,port} 已被占用时 MUST 失败（EADDRINUSE），MUST NOT port+1，MUST NOT 使用整机锁文件作为单实例闸。

  @req:r1780 @human
  场景: healthz 探针
    - Host 监听器 MUST 提供 GET /healthz 健康探针；服务端在地址端口上完成启动后访问该端点 MUST 返回 200 OK。

  @req:r1786 @human
  场景: 启动就绪窗口三态语义
    - Host 监听器 MUST 提供就绪状态机（starting / ready / stopping / failed）：绑定端口后装配完成前为 starting，此时 /healthz MUST 返回 503 并携带 starting 语义与 retry-after；其余 unary 与 WS 升级 MUST 统一 503 拒绝（同语义码）且 MUST NOT 半执行。装配完成后 /healthz MUST 返回 200。装配失败 MUST 进入 failed 不可重试语义；优雅停机 MUST 进入 stopping 语义。

  @req:r1787 @human
  场景: serve 注册文件发现契约
    - serve 装配完成（ready）后 MUST 原子写注册文件 `~/.xylitol/serve.json`（0600），内容含 url、pid、version；优雅退出 MUST 删除该文件。serve 进程 MUST 周期复读注册文件，字段与自身不全等（被顶替或被删）时 MUST 自行优雅退出。/healthz 各状态应答 body MUST 携带本进程 pid 与 version。客户端 attach 探活失败时 MUST 读取注册文件给出可操作分级诊断（未启动 / 僵死注册 / 旧版本 / 端口被无关进程占用 / serve 已退出），探活正常路径 MUST NOT 依赖该文件。

  @req:r1798 @human
  场景: 重连 journal
    - Host MUST 维护每会话单调事件序号与事件 journal；客户端经产品订阅携带 last_seq 重连时 MUST 从 last_seq+1 重放事件；journal 截断超过 last_seq 时 Host MUST 发出 session/resync_required。

  @req:r1799 @human
  场景: 审批与问卷
    - Host MUST 支持 ApprovalRequired 与 QuestionRequired：经 WS /rpc 下行 JSON-RPC notification 告知，挂起回合，并在匹配 call 的第一笔审批/问卷 unary 时恢复。MUST NOT 再以 POST /api/respond 为产品作答路径。

  @req:r1800 @human
  场景: 无整机锁文件
    - Host MUST NOT 依赖锁文件载荷（port/pid/hostname）做单实例互斥；优雅停机 MUST 释放监听套接字，MUST NOT 以删除锁文件当作停机协议。

  @req:r1801 @human
  场景: 禁止端口重试
    - 配置端口繁忙（EADDRINUSE）时 Host MUST 失败返回，MUST NOT 尝试 port+1，MUST NOT 用实际端口回写锁文件。

  @req:r1802 @human
  场景: unary 驱动回合
    - Host MUST 以 unary prompt 入队/驱动对话（HTTP 200 + JSON-RPC result）；abort 为 unary。MUST NOT 再提供 POST /api/v1/session/{id}/run、DELETE /api/v1/session/{id} 或 GET /api/v1/session/{id}/events 作为产品路径。

  @req:r1795 @human
  场景: ExportIo 接线
    - Server 运行时组合根 MUST 构造导出 I/O 实现，并在组装运行时注入导出协作器（优先经 app 组合缝）。

  @req:r1777 @human
  场景: Server 经 Driver 驱动
    - Server 运行时 MUST 通过 Driver（或等价文档化缝）驱动对话与会话命令，MUST NOT 在生产路径上以裸 AgentRuntime 作为唯一后端旁路 Print 主线。

  @req:r1788 @human
  场景: RemoteDriver 经信封
    - 产品 RemoteDriver MUST 经产品 JSON-RPC 入口调用已登记方法与事件订阅；未登记方法 MUST NOT 发明 REST 端点来假装与 InProcess 对等。

  @req:r1776 @human
  场景: Server 经 dispatch 执行命令
    - Server 对可映射的会话命令 MUST 经 protocol::Command + dispatch（或文档化的同一执行路径）执行，MUST NOT 为同一语义维护第二套与 dispatch 漂移的手写分支。

  @req:r1791 @human
  场景: 会话树经方法表
    - Host MUST 为 session_tree、travel_session_tree 与 append_entry_label 提供已登记的产品 unary；Host MUST NOT 再为 SessionTreeKind 提供 REST 读树/travel 端点。

  @req:r1783 @human
  场景: 会话生命周期方法接线
    - Host MUST 为 list_sessions、load_session_entries、new_session、get_session_name、set_session_name、set_session_name_for 与 delete_session 提供已登记 unary，并经同一 Driver/dispatch 语义执行；Remote 客户端调用这些方法时 MUST NOT 得到预留 unsupported。

  @req:r1779 @human
  场景: 上下文估计方法接线
    - Host MUST 为 estimate_context 提供已登记 unary：经与 in-process driver 同源的估算入口计算 ContextTokenEstimate，MUST 计入 host 侧固定请求开销（system prompt + tool schemas）与 host tokenizer 映射；MUST NOT 返回 stub 或客户端自估降级。

  @req:r1782 @human
  场景: wire 会话导入内容暂存
    - Host MUST 为 import_jsonl 提供内容暂存导入：载荷携带 content 而无 input_path 时，MUST 将内容暂存为临时输入路径、经同一 dispatch 导入并返回新 session_id，处理结束 MUST 清理暂存文件；携带 input_path 的直传行为 MUST 保持不变。

  @req:r1789 @human
  场景: Host 资源方法接线
    - Host MUST 提供 reload 与 loaded_resources unary；reload MUST 重装共享的 skills、MCP 与 prompt 资源，loaded_resources MUST 返回当前快照。该快照 MUST 反映进程内真实 MCP 连接态（configured / connecting / connected / 诊断），MUST NOT 来自一份从未发起连接的空壳。这些 Host 级操作 MUST NOT 被伪装成 session 专属 REST。

  @req:r1790 @human
  场景: 固定区资源下行
    - Host MUST 为已物化写者的会话经产品订阅下行 `session/resources` 推送 MCP/skills 固定区 快照：写者侧在 Host 进程内 poll MCP bootstrap，快照变化时才向该会话的订阅连接广播一帧，payload 含 session_id 与资源快照（形状与 loaded_resources unary 结果一致）。该帧 MUST 为 JSON-RPC notification，MUST NOT 消耗 journal seq，MUST NOT 写入事件 journal；断线重放与冷恢复投影 MUST NOT 复播固定区帧。不识别该方法的旧客户端 MUST 可忽略该帧且其余行为不受影响；本方法 MUST NOT 要求 bump 协议版本。

  @req:r1775 @human
  场景: abort 对进程级 reload 的合作取消
    - 进程级 reload 进行中收到 abort unary 时，Host MUST 合作取消该次 reload（停止后续重装步骤），MUST 以 cancelled 指示应答；reload 未进行时该 unary MUST 落回既有会话 abort 处理。reload 仅 idle 可发起，MUST NOT 与回合 abort 产生并发歧义。

  @req:r1784 @human
  场景: unary 调试文档
    - Host MUST 在监听器暴露 GET /openapi.json，返回 OpenAPI 3.1 文档描述产品 JSON-RPC 入口 POST /rpc（信封级）并附 /healthz。条目 MUST 从方法表生成，MUST NOT 手写第二套 schema 词表；payload schema 保持信封级粒度，具体形状以产品方法表为准。Host MUST 在 GET /docs 提供 Scalar 调试 UI（指向 /openapi.json），且它 MUST 是唯一的调试 UI，MUST NOT 引入第二套调试 UI。WS /rpc MUST NOT 作为 OpenAPI operation path 呈现，MUST 以文档说明指向同一 JSON-RPC 方法表。该端点仅调试文档，MUST NOT 作为客户端生成真源。MUST NOT 再为每个 unary 生成 /api/<method> 产品 path，MUST NOT 描述 /api/respond。

  @req:r1785 @human
  场景: 队列深度只读方法
    - Host MUST 提供只读 unary queue_stats（或语义等价），返回当前 session 的 steer 与 follow-up 深度；该 unary MUST NOT 占用写者租约。Remote 客户端 MUST NOT 将队列深度静默当成恒 0。

  @req:r1792 @human
  场景: 订阅跨回合存活
    - 客户端对某 session 的事件订阅 MUST 持续到该连接断开或再次订阅替换。单次 prompt 的 AgentEnd MUST NOT 结束该订阅，MUST NOT 停止后续 session/event（含 QueueUpdate 与下一轮）。

  @req:r1793 @human
  场景: session 写者租约
    - 同一 session MUST 至多一个写者租约。首次非只读方法颁发令牌：HTTP 经响应 header `X-Writer-Token` 回传；WS unary 应答 MUST 把同一令牌放在 JSON-RPC 对象顶层 `writerToken`（信封 extra member）。后续非只读方法：HTTP MUST 在请求 header 回显该令牌；WS 升级 MUST 可带同一 header，同一条连接内后续 unary 用连接本地租约。缺失或错误 MUST 以业务错误拒绝（HTTP 仍 200，产品码在 JSON-RPC error.data）。只读方法与事件订阅 MUST NOT 占用写者；HTTP 只读响应 MUST NOT 带该 header。`params` 与 `result` MUST NOT 携带 writerToken。HTTP 连接不是写者身份（每次 POST /rpc 都是新 TCP）。

  @req:r1803 @human
  场景: 下行信封
    - WS /rpc 文本帧 MUST 为 JSON-RPC 2.0。下行事件 MUST 为 notification（有 method 与 params，无 id）。产品下行 method MUST 含 session/event、session/subscribed、session/resync_required 与 session/resources；审批/问卷告知为 approval/requested 与 question/requested（亦为 notification）。MUST NOT 再以 ServerFrame tagged 外层或四象限 type tag 为产品真源，MUST NOT 要求客户端对事件 notification 作答。

  @req:r1804 @human
  场景: WS 只承载 JSON-RPC
    - WS /rpc MUST 接受合法 JSON-RPC 帧（含客户端 unary 与订阅）。MUST NOT 接受非 JSON-RPC 应用帧。载体 ping/pong/close 除外。MUST NOT 再以「禁一切业务上行」或 POST /api/respond 为产品通道纪律。

  @req:r1805 @human
  场景: WS 升级
    - Host MUST 在 WS /rpc 挂载 WebSocket 升级。会话选择 MUST NOT 依赖 /api/v1/session/{id}/ws 或 GET /api/events.mux。缺 Origin 的原生客户端 MUST 可升级。

  @req:r1806 @human
  场景: 单调 seq
    - 每个会话 MUST 维护单调递增的 u64 序号，每 append 事件递增；seq MUST 每会话单调递增。

  @req:r1807 @human
  场景: journal 环形缓冲
    - Host MUST 维护每会话最近 N 个事件的环形 journal（默认 10000）；journal 环绕超过客户端 last_seq 时，Host MUST 向该客户端发出 session/resync_required。

  @req:r1808 @human
  场景: resync 流程
    - 收到 session/resync_required 后，客户端 MUST 以更新的 last_seq（如 0 全量重放）再次订阅以恢复事件流。

  @req:r1809 @human
  场景: WS 事件推送
    - 订阅之后，Host MUST 为 agent 发出的每个新事件推送 session/event 的 JSON-RPC notification，附带会话单调 seq；MUST 以 JSON 文本帧经 WS /rpc 发送。

  @req:r1810 @human
  场景: 冷恢复投影
    - Host MUST 提供会话消息快照 unary（get_messages 或等价）一次返回该会话完整 transcript 条目，作为客户端恢复（冷订或切会话）的投影源；journal 实况重放（sr4）MUST NOT 作为冷恢复的 transcript 投影来源；快照内容 MUST NOT 被恢复窗内的实况磁带回放污染。断线续传语义仍按 sr4/w5/w6。

  @req:r1771 @human
  场景: reverse-rpc-approval
    - ApprovalRequired 时，Host MUST 向该 session 的订阅连接广播 approval/requested notification；首个匹配该 call 的审批 unary 胜出。

  @req:r1772 @human
  场景: reverse-rpc-question
    - QuestionRequired 时，Host MUST 向该 session 的订阅连接广播 question/requested notification；首个匹配该 call 的问卷 unary 胜出。

  @req:r1773 @human
  场景: call-id-lifecycle
    - 每个审批/问卷 MUST 有稳定 call id（出现在 notification 的 params 中）；超时（默认 60s）时收到 ApprovalTimeout；应答时收到结果；消费后 MUST 移除条目。JSON-RPC 信封 id 是该 unary 的幂等键，MUST NOT 与 call id 混用。

  @req:r1774 @human
  场景: no-conflict-arbitration
    - v1 MUST NOT 实现多客户端冲突仲裁；给定 call id 的首个审批/问卷 unary 胜出；同一 call id 的后续 unary MUST 被静默忽略。

  @req:r1781 @human
  场景: unary 幂等准入
    - Host MUST 以 JSON-RPC `id` 为幂等键：同一 session 槽内，同 `id` 的重复 unary 首次准入获胜，Host MUST 回放首次执行结果且 MUST NOT 二次执行；首次仍在处理中的同键重复 MUST 等待首次完成后获得同一结果，MUST NOT 并行执行。同 `id` 但 method 或 params 不同的提交 MUST 返回稳定冲突错误（产品码 idempotency_conflict，HTTP 仍 200）。幂等账本 MUST 有界且进程内，MUST NOT 跨重启持久化。

  @executable @req:r1780
  场景: start-healthz
    当 服务端在空闲端口上启动
    那么 healthz 端点返回 200 OK
  @req:r1784 @executable
  场景: openapi-debug-doc
    当 服务端在空闲端口上启动
    并且 GET /openapi.json
    那么 返回 OpenAPI 3.1 文档且描述 POST /rpc 信封
    并且 文档不含 WS 下行 path
    当 GET /docs
    那么 Scalar 调试页可达且指向 spec
  @executable @req:r1797
  场景: second-instance-rejected
    假如 服务端已在该地址端口监听
    当 第二个服务端绑定同一地址端口
    那么 第二个实例因地址占用失败
  @req:r1794 @executable
  场景: server-under-app
    当 server 应用面启动
    那么 装配 infra 运行时并按 session 槽注入 Driver，供 unary 处理器调用
  @req:r1778 @executable
  场景: product-path-jsonrpc
    当 服务端在空闲端口上启动
    并且 POST /rpc 调用 host.describe
    那么 应答为 JSON-RPC 成功且 id 回显
    并且 Host 对该路径给出可观察往返
  @req:r1796 @executable
  场景: server-rest-ws-under-app
    当 启动 app::server 运行时
    那么 暴露 POST /rpc、WS /rpc 与 GET /healthz
    并且 不暴露 /api/v1 产品 REST
  @req:r1797 @executable
  场景: server-lock-under-app
    当 第二个 app::server 进程绑定同一地址端口
    那么 因地址占用失败且不写整机锁文件
  @req:r1798 @executable
  场景: reconnect-replay
    假如 客户端断开 N 秒后以 last_seq 经 POST /rpc 订阅
    当 断开期间 server 产生事件
    那么 客户端收到 last_seq+1 起全部遗漏事件再收实时事件
  @req:r1799 @executable
  场景: approval-roundtrip
    假如 工具需审批
    当 推送 ApprovalRequired
    那么 客户端经 POST /rpc 调用 approve_tool 且回合恢复
  @req:r1800 @executable
  场景: lock-file-json
    假如 server 已启动
    当 检查单实例互斥手段
    那么 不依赖锁文件 JSON 的 port、pid、hostname
  @req:r1801 @executable
  场景: port-retry-binds
    假如 18790 被其它进程占用
    当 server 在 18790 启动
    那么 失败返回且不绑定 18791
  @req:r1802 @executable
  场景: run-endpoint-works
    假如 向 POST /rpc 发送 prompt 的 JSON-RPC 请求
    当 server 处理 prompt
    那么 HTTP 200 且应答回显 id
    并且 POST /api/v1/session/x/run 不是产品路径
  @req:r1795 @executable
  场景: server-composition-export-io
    当 app::server 运行时经 app::core::composition 组装 Agent
    那么 向 Agent 传入 Arc<dyn ExportIo>
  @req:r1777 @executable
  场景: no-bare-runtime
    假如 审查 server 组合根
    当 检查持有类型
    那么 按 session 槽持有 Driver；handlers 不直接以 Mutex AgentRuntime 作为唯一入口
  @req:r1788 @executable
  场景: remote-commands
    当 调用 RemoteDriver 已登记 unary（如 steer）
    那么 经 HostClient 到达 Host
    并且 未登记方法不发明 REST
  @req:r1776 @executable
  场景: shared-path
    当 审查 prompt/steer 等 unary handlers
    那么 调用 dispatch 或共享 helper；与 InProcessDriver 语义一致
  @req:r1791 @executable
  场景: rest-get-tree
    假如 server 上存在 session 与消息树
    当 请求 /api/v1 会话树 REST
    那么 不是产品路径（不存在或非产品）
  @req:r1791 @executable
  场景: remote-travel
    假如 RemoteDriver 指向该 server
    当 调用已登记方法表的 session_tree/travel
    那么 经 JSON-RPC unary 到达 Host 且不经 REST 冒充
  @req:r1783 @executable
  场景: remote-session-methods
    假如 RemoteDriver 指向该 server
    当 调用已登记的 session 能力 unary
    那么 经 JSON-RPC unary 到达 Host 且不经 REST 冒充
  @req:r1782 @executable
  场景: staged-wire-import
    假如 server 就绪
    当 推送 content 载荷导入会话
    那么 返回新 session_id 且暂存文件不残留
  @req:r1789 @executable
  场景: remote-host-resource-methods
    假如 RemoteDriver 指向该 server
    当 调用 Host 资源 unary
    那么 经 JSON-RPC unary 到达 Host 且不经 REST 冒充
  @req:r1793 @executable
  场景: writer-lease
    假如 客户端 A 已对 session 经 POST /rpc 发出非只读方法
    当 客户端 B 无 X-Writer-Token 再发非只读方法
    那么 业务错误 data.code 为 writer_conflict
    并且 A 的响应带 X-Writer-Token 且 body 不含 writerToken
  @req:r1793 @executable
  场景: writer-token-via-ws-unary
    当 服务端在空闲端口上启动
    并且 客户端经 WS /rpc 发送非只读 JSON-RPC 请求
    那么 WS 应答顶层有 writerToken 且 result 不含

  @req:r1803 @executable
  场景: frame-serialize
    假如 构造下行 session/event、session/subscribed、session/resync_required 与 session/resources notification
    当 序列化为 JSON
    那么 各帧均为 JSON-RPC 2.0 notification 且无 id
  @req:r1804 @executable
  场景: subscribe-frame
    假如 客户端欲以 last_seq 5 订阅会话 s0
    当 服务端在空闲端口上启动
    并且 POST /rpc 调用 subscribe 带 session_id=s0 与 last_seq=5
    那么 应答为 JSON-RPC 成功且 result 含 session 与 seq
    并且 WS 丢弃非 JSON-RPC 文本上行
  @req:r1804 @executable
  场景: ws-jsonrpc-unary-same-module
    当 服务端在空闲端口上启动
    并且 客户端经 WS /rpc 发送 host.describe JSON-RPC 请求
    那么 同一条 WS 收回显 id 的 JSON-RPC result
  @req:r1805 @executable
  场景: ws-upgrade-mounted
    假如 客户端连接 /rpc
    当 server 接受升级
    那么 升级成功
    并且 不把会话绑在 /api/v1/session/x/ws
  @req:r1806 @executable
  场景: seq-monotonic
    假如 向会话 append 3 个事件
    当 读回事件
    那么 seq 值为 1,2,3（严格递增）
  @req:r1807 @executable
  场景: resync-on-wrap
    假如 向会话 append 10001 个事件（journal 容量=10000）
    当 last_seq=0 的客户端经 POST /rpc 订阅
    那么 server 发送 session/resync_required 因事件 0..1 已丢失
  @req:r1808 @executable
  场景: resync-recover
    假如 客户端收到 session/resync_required
    当 客户端以 last_seq=0 再次经 POST /rpc 订阅
    那么 server 从 journal 重放全部可用事件
  @req:r1809 @executable
  场景: ws-events-pushed
    假如 客户端已 subscribe 且 prompt 运行
    当 agent 发出 TextDelta 事件
    那么 客户端收到 session/event notification
  @req:r1810 @executable
  场景: cold-restore-snapshot-projection
    假如 会话 s0 已有 3 条历史条目且客户端无有效 last_seq
    当 冷订客户端调用消息快照
    那么 快照一次返回全部 3 条条目
  @req:r1810 @executable
  场景: replay-window-does-not-pollute-snapshot
    假如 冷订客户端处于恢复窗内且 journal 含实况磁带事件
    当 调用消息快照
    那么 快照内容与磁带回放无关
    并且 断线续传语义仍按 sr4 从 last_seq+1 重放

  @executable @req:r1771
  场景: approve-roundtrip
    假如 服务端和已连接的 mux 客户端
    当 agent 执行需要审批的工具
    那么 客户端收到 approval/requested notification
    当 客户端 POST /rpc 调用 approve_tool 且 approved=true
    那么 工具执行继续
    并且 turn 正常结束
  @executable @req:r1771
  场景: tool-denied
    假如 服务端和已连接的 mux 客户端
    当 agent 执行需要审批的工具
    并且 客户端 POST /rpc 调用 approve_tool 且 approved=false
    那么 工具被拒绝
    并且 turn 继续但不包含工具结果
  @req:r1771 @executable
  场景: approval-broadcast
    假如 agent 发出 ApprovalRequired，rpcId 为 xyz
    当 server 检查 session 的 mux 连接
    那么 3 个已连接客户端均收到 approval/requested
  @req:r1772 @executable
  场景: first-wins
    假如 2 个客户端 POST /rpc 调用 approve_tool，call_id xyz（首个 true，50ms 后 false）
    当 server 处理首个应答
    那么 agent 以 approved=true 恢复；第二个应答被忽略
  @req:r1773 @executable
  场景: timeout-error
    假如 60s 内无客户端 POST /rpc 调用 approve_tool
    当 server 将 rpcId 标记为 expired
    那么 agent 收到 ApprovalTimeout 错误
  @req:r1774 @executable
  场景: second-answer-ignored
    假如 第二个客户端对已消费 call 再 POST /rpc 调用 approve_tool
    当 server 收到该应答
    那么 应答被静默忽略（无状态变化、无错误）
  @req:r1785 @executable
  场景: queue-stats-readonly-unary
    当 服务端在空闲端口上启动
    并且 POST /rpc 查询只读方法 queue_stats
    那么 返回 steer 与 follow-up 队列深度
    并且 响应不携带写者租约 token
  @req:r1775 @executable
  场景: reload-cooperative-cancel
    假如 进程级 reload 正在进行（取消令牌已注册）
    当 收到进程级 abort unary
    那么 应答携带 cancelled 指示且取消令牌被置位
  @req:r1775 @executable
  场景: abort-idle-falls-back-to-session
    假如 无进行中的进程级 reload
    当 收到进程级 abort unary
    那么 未命中 reload 取消而落回会话 abort 处理
  @req:r1792 @executable
  场景: subscription-survives-agent-end
    假如 客户端已订阅会话 s-sub
    当 会话回合以 AgentEnd 结束后又追加新事件
    那么 订阅仍存活且新事件继续送达

  @executable @req:r1781
  场景: idempotent-replay-first-result
    假如 客户端以 JSON-RPC id R 对某 session 经 POST /rpc 提交方法并得到结果
    当 客户端以相同 id R 重试同一方法
    那么 第二次得到与首次相同的结果且命令仅执行一次

  @executable @req:r1781
  场景: idempotency-conflict-differs
    假如 JSON-RPC id R 已被某 method 与 params 的提交占用
    当 以相同 id R 提交不同 method 或 params
    那么 返回 JSON-RPC 错误且 data.code 为 idempotency_conflict 且不执行

  @executable @req:r1781
  场景: idempotency-inflight-wait
    假如 JSON-RPC id R 的首次命令仍在处理中
    当 相同 id R 的重复请求到达
    那么 等待首次完成并回放同一结果且不并行执行

  @executable @req:r1786
  场景: starting-window-503
    假如 监听器已绑定端口但装配未完成
    当 访问 /healthz 或任一 unary
    那么 healthz 返回 503 且携带 starting 语义与 retry-after
    并且 unary 得到同语义 503 且不半执行

  @executable @req:r1786
  场景: ready-flips-healthz
    假如 装配已完成
    当 访问 /healthz
    那么 返回 200 OK

  @executable @req:r1786
  场景: failed-nonretryable
    假如 装配失败
    当 访问 /healthz
    那么 返回 503 且携带 failed 不可重试语义

  @executable @req:r1787
  场景: serve-writes-registration-file
    假如 serve 已装配完成并在空闲端口监听
    当 读取注册文件
    那么 文件存在且为 0600 且内容含 url、pid 与 version
    并且 healthz 应答 body 的 pid 和 version 与文件一致

  @executable @req:r1787
  场景: attach-diagnoses-stale-registration
    假如 注册文件存在但对应端口的 serve 已退出
    当 客户端 attach 探活该地址
    那么 得到可操作诊断说明 pid 对应的 serve 已退出并提示重新 serve

  @executable @req:r1787
  场景: registration-takeover-evicts-old-daemon
    假如 旧 serve 进程持有注册文件
    当 新 serve 写入字段不同的注册文件
    那么 旧进程在自检周期内检测到字段不全等并自行退出
