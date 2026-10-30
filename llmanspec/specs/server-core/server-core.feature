# language: zh-CN
# purpose: Server 运行时 — 独立 Host 监听器经 JSON-RPC 2.0 文本信封暴露产品契约（POST /rpc + WS /rpc）；每 session 槽独立 journal 与写者。

# ── 文件导览（60 规则）────────────────────────────────────────
#   接入与生命周期    : host · HTTP/WS · healthz/就绪 · session 方法表
#   信封与命令面      : unary · dispatch · 队列/订阅/写占用 · export
#   线上协议          : JSON-RPC 信封 · 格式协商 · 方法名寻址 · seq · 演进
#   审批/恢复         : reverse-rpc approval/question · journal · 冷恢复投影
#   （精确检索以 @req/规则标题为准，本导览为非强制快速导航）
# ────────────────────────────────────────────────────────────────

功能: server-core

  @req:r1794
  规则: server 托管 agent
    app::server 须在启动时经 Kernel::boot（Host profile）装配 infra 运行时并注入 agent 端口；组合根 须持有可按 session 槽取用的 Driver（或等价缝），供 unary 处理器调用，不得以无槽的裸 Agent 作为唯一生产入口。serve 是 Host profile 的载体插件。

    场景: 服务端低于应用
      当 server 应用面启动
      那么 装配 infra 运行时并按 session 槽注入 Driver，供 unary 处理器调用

  @req:r1778
  规则: 产品路径
    产品客户端 必须经 JSON-RPC 2.0 文本访问 Host，入口为 POST /rpc 与 WS /rpc（同一方法表、同一 dispatch）：JSON-RPC 文本 = 唯一产品路径。Host 不得接受二进制 body 或 binary 帧作为 /rpc 业务通道。现行 REST 资源动词、POST /api/respond、GET /api/events.mux 与 JSON-RPC 之外的 WS 应用帧 不得作为产品真源。

    场景: 产品路径RPC
      当 服务端在空闲端口上启动
      并且 POST /rpc 调用 host.describe
      那么 应答为产品成功且 id 回显
      并且 Host 对该路径给出可观察往返

  @req:r1796
  规则: 产品 HTTP 与 WS 入口
    Host 须暴露 POST /rpc 与 WS /rpc 作为产品入口（同一方法表；仅 JSON-RPC 文本）。须另暴露 GET /healthz。不得暴露 GET /openapi.json 或 GET /docs。不得再以 POST /api/{method}、POST /api/respond 或 GET /api/events.mux 为产品真源。不得再以 /api/v1 REST 资源动词或 JSON-RPC 之外的 WS 应用帧承载产品命令。

    场景: 服务端其余WebSocket低于应用
      当 启动 app::server 运行时
      那么 暴露 POST /rpc、WS /rpc 与 GET /healthz
      并且 不暴露 /api/v1 产品 REST

  @req:r1797
  规则: 绑定占用
    Host 须绑定配置的地址与端口（默认 127.0.0.1:18790）。同一 {addr,port} 已被占用时 须失败（EADDRINUSE），不得 port+1，不得使用整机锁文件作为单实例闸。

    场景: 第二次实例拒绝
      假如 服务端已在该地址端口监听
      当 第二个服务端绑定同一地址端口
      那么 第二个实例因地址占用失败

    场景: 服务端锁低于应用
      当 第二个 app::server 进程绑定同一地址端口
      那么 因地址占用失败且不写整机锁文件

  @req:r1780
  规则: healthz 探针
    Host 监听器 须提供 GET /healthz 健康探针；服务端在地址端口上完成启动后访问该端点 须返回 200 OK。

    场景: 启动healthz
      当 服务端在空闲端口上启动
      那么 healthz 端点返回 200 OK

  @req:r1786
  规则: 启动就绪窗口三态语义
    Host 监听器 须提供就绪状态机（starting / ready / stopping / failed）：绑定端口后装配完成前为 starting，此时 /healthz 须返回 503 并携带 starting 语义与 retry-after；其余 unary 与 WS 升级 须统一 503 拒绝（同语义码）且 不得半执行。装配完成后 /healthz 须返回 200。装配失败 须进入 failed 不可重试语义；优雅停机 须进入 stopping 语义。

    场景: 启动中窗口503
      假如 监听器已绑定端口但装配未完成
      当 访问 /healthz 或任一 unary
      那么 healthz 返回 503 且携带 starting 语义与 retry-after
      并且 unary 得到同语义 503 且不半执行

    场景: 就绪翻转healthz
      假如 装配已完成
      当 访问 /healthz
      那么 返回 200 OK

    场景: 失败不可重试
      假如 装配失败
      当 访问 /healthz
      那么 返回 503 且携带 failed 不可重试语义

    场景大纲: 就绪阶段转换表
      假如 监听器处于 "<from>" 阶段
      当 装配事件 "<event>" 发生
      那么 就绪状态迁移为 "<to>"

      例子:
        | from     | event             | to     |
        | starting | assembly_complete | ready  |
        | starting | assembly_fail     | failed |

  @req:r1787
  规则: serve 注册文件发现契约
    serve 装配完成（ready）后 须原子写注册文件 `~/.xylitol/serve.json`（0600），内容含 url、pid、version；优雅退出 须删除该文件。serve 进程 须周期复读注册文件，字段与自身不全等（被顶替或被删）时 须自行优雅退出。/healthz 各状态应答 body 须携带本进程 pid 与 version。客户端 attach 探活失败时 须读取注册文件给出可操作分级诊断（未启动 / 僵死注册 / 旧版本 / 端口被无关进程占用 / serve 已退出），探活正常路径 不得依赖该文件。

    场景: 服务写入登记文件
      假如 serve 已装配完成并在空闲端口监听
      当 读取注册文件
      那么 文件存在且为 0600 且内容含 url、pid 与 version
      并且 healthz 应答 body 的 pid 和 version 与文件一致

    场景: attach诊断过期登记
      假如 注册文件存在但对应端口的 serve 已退出
      当 客户端 attach 探活该地址
      那么 得到可操作诊断说明 pid 对应的 serve 已退出并提示重新 serve

    场景: 登记接管驱逐旧守护进程
      假如 旧 serve 进程持有注册文件
      当 新 serve 写入字段不同的注册文件
      那么 旧进程在自检周期内检测到字段不全等并自行退出

  @req:r1798
  规则: 重连 journal
    Host 须维护每会话单调事件序号与事件 journal；客户端经产品订阅携带 last_seq 重连时 须从 last_seq+1 重放事件；journal 截断超过 last_seq 时 Host 须发出 session/resync_required。journal 须只承载事实事件（生命周期边界、消息边界、工具边界、状态变更），流式 tape（TextDelta、ThinkingDelta、MessageUpdate、ToolExecutionUpdate）不得写入 journal、不得消耗 seq。tape 帧携带的 `seq` 只表示其所属事实位置快照，不唯一标识帧；客户端 不得按 `seq` 去重或过滤 tape，须按到达顺序处理。中段重连（last_seq>0 且回合成中）须在订阅应答的可选 partial 字段携带当前回合最新累积消息快照，使客户端无需 tape 重放即可恢复当前回合文本与工具意图行；冷订阅（last_seq=0）不得携带 partial。

    场景: 重连回放
      假如 客户端断开 N 秒后以 last_seq 经 POST /rpc 订阅
      当 断开期间 server 产生事件
      那么 客户端收到 last_seq+1 起全部遗漏事件再收实时事件

    场景: tape 不占 journal 序号
      假如 会话产生流式 tape 与事实事件
      当 读取 journal 最大序号
      那么 tape 未消耗任何序号且仅事实事件可被重放

    场景: 中段重连带当前回合快照
      假如 会话回合成中且客户端以 last_seq>0 订阅
      当 订阅确立
      那么 应答含 partial 且其内容为当前回合最新累积消息
      并且 客户端 以 partial 快照恢复当前回合的工具意图行

  @req:r1799
  规则: 审批与问卷
    Host 须支持 ApprovalRequired 与 QuestionRequired：经 WS /rpc 下行通知告知，挂起回合，并在匹配 call 的第一笔审批/问卷 unary 时恢复。不得再以 POST /api/respond 为产品作答路径。

    场景: 审批往返
      假如 工具需审批
      当 推送 ApprovalRequired
      那么 客户端经 POST /rpc 调用 approve_tool 且回合恢复

  @req:r1800
  规则: 无整机锁文件
    Host 不得依赖锁文件载荷（port/pid/hostname）做单实例互斥；优雅停机 须释放监听套接字，不得以删除锁文件当作停机协议。

    场景: 锁文件JSON
      假如 server 已启动
      当 检查单实例互斥手段
      那么 不依赖锁文件 JSON 的 port、pid、hostname

  @req:r1801
  规则: 禁止端口重试
    配置端口繁忙（EADDRINUSE）时 Host 须失败返回，不得尝试 port+1，不得用实际端口回写锁文件。

    场景: 端口重试绑定
      假如 18790 被其它进程占用
      当 server 在 18790 启动
      那么 失败返回且不绑定 18791

  @req:r1802
  规则: unary 驱动回合
    Host 须以 unary prompt 入队/驱动对话（HTTP 200 + 产品应答）；abort 为 unary。不得再提供 POST /api/v1/session/{id}/run、DELETE /api/v1/session/{id} 或 GET /api/v1/session/{id}/events 作为产品路径。

    场景: 运行端点可用
      假如 向 POST /rpc 发送 prompt 请求
      当 server 处理 prompt
      那么 HTTP 200 且应答回显 id
      并且 POST /api/v1/session/x/run 不是产品路径

  @req:r1795
  规则: ExportIo 接线
    Server 运行时组合根 须构造导出 I/O 实现，并在组装运行时注入导出协作器（优先经 app 组合缝）。

    场景: 服务端组合导出IO
      当 app::server 运行时经 app::core::composition 组装 Agent
      那么 向 Agent 传入 Arc<dyn ExportIo>

  @req:r1777
  规则: Server 经 Driver 驱动
    Server 运行时 须通过 Driver（或等价文档化缝）驱动对话与会话命令，不得在生产路径上以裸 AgentRuntime 作为唯一后端旁路 Print 主线。

    场景: 无裸运行时
      假如 审查 server 组合根
      当 检查持有类型
      那么 按 session 槽持有 Driver；handlers 不直接以 Mutex AgentRuntime 作为唯一入口

  @req:r1788
  规则: RemoteDriver 经信封
    产品 RemoteDriver 须经产品入口调用已登记方法与事件订阅；未登记方法 不得发明 REST 端点来假装与 InProcess 对等。

    场景: 远端命令
      当 调用 RemoteDriver 已登记 unary（如 steer）
      那么 经 HostClient 到达 Host
      并且 未登记方法不发明 REST

  @req:r1914
  规则: prompt 会话身份保真
    产品 RemoteDriver 发送 prompt/run 时 须携带会话身份与 run 上下文字段（session_id、cwd、model_id、thinking_level 或等价信息），并使其经任何线协议形状转换后仍保真到达 Host 的会话解析；Host 须将 run 路由到该字段所指会话并在其上广播事件，不得因命令承载字段收缩把 run 落向无订阅者的 fallback 会话而事件不可达。

    场景: 提示词运行时已订阅会话
      假如 客户端已订阅会话 s-prompt 并以可用模型 fake-model 发送 prompt
      当 Host 处理该 prompt
      那么 run 路由到 s-prompt 且事件在 s-prompt 的订阅者上可达

  @req:r1915
  规则: 生效模型状态同步
    Host 在会话写者装配出已解析模型（含用户显式配置的默认模型）且订阅者绑定该会话时 须向订阅者下发生效模型同步事件（session/model 形状），使附加端模型徽标收敛而非停留在未选占位；显式 set_model/cycle_model 后的模型状态 须反映切换结果。不识别该事件形状的旧客户端 须可忽略该帧且其余行为不受影响。

    场景: 模型同步时绑定
      假如 会话写者已装配且持有已解析默认模型 fake-model
      当 订阅者绑定该会话
      那么 订阅者收到模型同步事件且模型为 fake-model

  @req:r1916
  规则: 会话命令身份透明保真
    产品 RemoteDriver 发送的任何 session-scoped 命令（prompt、set_model、cycle_model、set_thinking_level、steer、follow_up、bash、compact、abort、get_state、session_tree、get_messages 等）须与注入的会话身份与上下文字段（session_id、cwd 及所需业务字段）在任意线协议形状下保真到达 Host 的会话解析，Host 须将命令路由到所指会话；线协议形状转换 不得因固定 schema 变体收缩剥离注入字段，把命令落向无写者 / 非订阅的 fallback 会话（症状：writer_conflict、状态侧写、副作用落错槽）。上行对命令为透明信封：除握手 host.describe 外不得把命令压缩进会丢失注入字段的固定 schema 变体。

    场景: 命令保留会话身份
      假如 以产品客户端订阅会话 s-rpc 且写者装配了默认模型 fake-model
      当 该客户端在 s-rpc 上设置模型 fake-model
      那么 设置无 writer_conflict

  @req:r1917
  规则: get_state 反映生效写者模型
    get_state unary 在会话写者已装配时 须返回该写者当前模型与 thinking level（而非对无模型 reader 的空模型）；写者未装配时 MAY 返回空占位。

    场景: 读取状态反映写者模型
      假如 会话写者已装配且持有已解析默认模型 fake-model
      当 客户端请求该会话 get_state
      那么 应答的 model 为 fake-model

  @req:r2973
  规则: 会话切换与订阅的模式事件路由
    Host 在 `switch_session` / `new_session` 成功后 须把 `planModeChanged` 发布到切换后的新会话槽，不得落到切换前的槽或 fallback 会话；订阅者绑定会话时 须收到一次由该会话最新 `plan_mode` 推导的 `planModeChanged`（不写入 journal、不消耗 seq），使附加端模式条收敛。其他会话的订阅者 不得收到本事件。

    场景: 切换会话后的模式事件路由
      假如 Host 成功 switch_session 或 new_session
      当 发布 planModeChanged
      那么 事件进入切换后的新会话槽且订阅者收到一次由最新 plan_mode 推导的事件

  @req:r1776
  规则: Server 经 dispatch 执行命令
    Server 对可映射的会话命令 须经 protocol::Command + dispatch（或文档化的同一执行路径）执行，不得为同一语义维护第二套与 dispatch 漂移的手写分支。

    场景: 共享路径
      当 审查 prompt/steer 等 unary handlers
      那么 调用 dispatch 或共享 helper；与 InProcessDriver 语义一致

  @req:r1791
  规则: 会话树经方法表
    Host 须为 session_tree、travel_session_tree 与 append_entry_label 提供已登记的产品 unary；Host 不得再为 SessionTreeKind 提供 REST 读树/travel 端点。

    场景: 其余读取树
      假如 server 上存在 session 与消息树
      当 请求 /api/v1 会话树 REST
      那么 不是产品路径（不存在或非产品）

    场景: 远端旅行
      假如 RemoteDriver 指向该 server
      当 调用已登记方法表的 session_tree/travel
      那么 经产品 unary 到达 Host 且不经 REST 冒充

  @req:r1783
  规则: 会话生命周期方法接线
    Host 须为 list_sessions、load_session_entries、new_session、get_session_name、set_session_name、set_session_name_for 与 delete_session 提供已登记 unary，并经同一 Driver/dispatch 语义执行；Remote 客户端调用这些方法时 不得得到预留 unsupported。

    场景: 远端会话方法
      假如 RemoteDriver 指向该 server
      当 调用已登记的 session 能力 unary
      那么 经产品 unary 到达 Host 且不经 REST 冒充

  @req:r1779
  规则: 上下文估计方法接线
    Host 须为 estimate_context 提供已登记 unary：经与 in-process driver 同源的估算入口计算 ContextTokenEstimate，须计入 host 侧固定请求开销（system prompt + tool schemas）；不得返回 stub 或客户端自估降级。返回的 ContextTokenEstimate 须含 prompt cache 三态（与 in-process 同源），不得丢弃为不适用。

    场景: 估算上下文一元已登记只读
      当 查询方法表的 estimate_context
      那么 已登记且为只读 unary 不占写者

    场景: 估算上下文Host来源不自身估算
      假如 会话条目含 stop 回合的 usage 锚点
      当 以同源估计器估计上下文
      那么 估计来源为 Api
      假如 会话条目仅含 abort 回合的 usage 锚点
      当 以同源估计器估计上下文
      那么 估计来源降级而非 Api

    场景: 估算上下文携带缓存三态
      假如 会话条目含缓存命中的 stop 回合 usage 锚点
      当 以同源估计器估计上下文
      那么 估计缓存状态为命中且 token 数一致

  @req:r1782
  规则: wire 会话导入内容暂存
    Host 须为 import_jsonl 提供内容暂存导入：载荷携带 content 而无 input_path 时，须将内容暂存为临时输入路径、经同一 dispatch 导入并返回新 session_id，处理结束 须清理暂存文件；携带 input_path 的直传行为 须保持不变。

    场景: 暂存线协议导入
      假如 server 就绪
      当 推送 content 载荷导入会话
      那么 返回新 session_id 且暂存文件不残留

  @req:r1789
  规则: Host 资源方法接线
    Host 须提供 reload 与 loaded_resources unary；reload 须重装共享的 skills、MCP 与 prompt 资源，loaded_resources 须返回当前快照。该快照 须反映进程内真实 MCP 连接态（configured / connecting / connected / 诊断），不得来自一份从未发起连接的空壳。这些 Host 级操作 不得被伪装成 session 专属 REST。

    场景: 远端Host资源方法
      假如 RemoteDriver 指向该 server
      当 调用 Host 资源 unary
      那么 经产品 unary 到达 Host 且不经 REST 冒充

  @req:r1790
  规则: 固定区资源下行
    Host 须为已物化写者的会话经产品订阅下行 `session/resources` 推送 MCP/skills 固定区 快照：写者侧在 Host 进程内 poll MCP bootstrap，快照变化时才向该会话的订阅连接广播一帧，payload 含 session_id 与资源快照（形状与 loaded_resources unary 结果一致）。该帧 须为无作答义务的下行通知，不得消耗 journal seq，不得写入事件 journal；断线重放与冷恢复投影 不得复播固定区帧。不识别该方法的旧客户端 须可忽略该帧且其余行为不受影响；本方法 不得要求 bump 协议版本。

    场景: 固定区资源帧为通知
      假如 构造下行 session/event、session/subscribed、session/resync_required 与 session/resources notification
      当 编码为下行通知帧
      那么 各帧均为无作答义务的下行通知且携带 method 语义

    场景: 固定区资源方法时产品表
      假如 RemoteDriver 指向该 server
      当 调用 Host 资源 unary
      那么 经产品 unary 到达 Host 且不经 REST 冒充

  @req:r1775
  规则: abort 对进程级 reload 的合作取消
    进程级 reload 进行中收到 abort unary 时，Host 须合作取消该次 reload（停止后续重装步骤），须以 cancelled 指示应答；reload 未进行时该 unary 须落回既有会话 abort 处理。reload 仅 idle 可发起，不得与回合 abort 产生并发歧义。

    场景: 重载协作取消
      假如 进程级 reload 正在进行（取消令牌已注册）
      当 收到进程级 abort unary
      那么 应答携带 cancelled 指示且取消令牌被置位

    场景: 中止空闲回落返回至会话
      假如 无进行中的进程级 reload
      当 收到进程级 abort unary
      那么 未命中 reload 取消而落回会话 abort 处理

  @req:r1784
  规则: unary 调试文档
    Host 不得暴露 GET /openapi.json 或 GET /docs（含 Scalar 或其它调试 UI）。产品方法表与载荷形状以协议真源为准，不得再以 JSON OpenAPI 为客户端或人读真源。不得再为每个 unary 生成 /api/<method> 产品 path。

    场景: OpenAPI调试文档
      当 服务端在空闲端口上启动
      并且 GET /openapi.json
      那么 该路径不提供调试文档
      当 GET /docs
      那么 该路径不提供调试文档

  @req:r1785
  规则: 队列深度只读方法
    Host 须提供只读 unary queue_stats（或语义等价），返回当前 session 的 steer 与 follow-up 深度；该 unary 不得占用会话写锁。Remote 客户端 不得将队列深度静默当成恒 0。

    场景: 队列统计只读一元
      当 服务端在空闲端口上启动
      并且 POST /rpc 查询只读方法 queue_stats
      那么 返回 steer 与 follow-up 队列深度

  @req:r1792
  规则: 订阅跨回合存活
    客户端对某 session 的事件订阅 须持续到该连接断开或再次订阅替换。单次 prompt 的 AgentEnd 不得结束该订阅，不得停止后续 session/event（含 QueueUpdate 与下一轮）。

    场景: 订阅存活agent结束
      假如 客户端已订阅会话 s-sub
      当 会话回合以 AgentEnd 结束后又追加新事件
      那么 订阅仍存活且新事件继续送达

  @req:r1793
  规则: session 写占用
    同一已物化 session 跨进程 须至多一个写占用。占用 须为内核排他对象：POSIX 对该会话目录内 `session.lock` 取 `flock`（锁文件 不得在释放时 unlink，不得锁会原子改写的提交指针）；Windows 须用由该锁路径派生的命名内核对象，不得对会话数据文件做强制文件锁。不得用协议令牌或进程内 UUID 当跨进程所有权。持有进程退出后内核 须释放。他进程争用 须以业务错误拒绝（HTTP 仍 200），不得静默成功后再等待回合结束。
    该 session 最后一条事件订阅连接干净关闭（含客户端主动退出与切走会话）须立即释放写占用。异常断开 须在可配置宽限后释放（缺省约 60 秒，0 表示立即）。宽限内同一进程重连 须续占。不得在宽限到期时抢夺他进程仍持有的内核锁。
    只读方法 不得单独要求持锁。载荷与应答 不得携带 writerToken 或 X-Writer-Token。

    场景: 两进程写入冲突
      假如 进程 A 已对会话取得写占用
      当 进程 B 再对该会话发起写占用
      那么 B 得到业务错误且不得写入

    场景: 持有者退出释放
      假如 进程 A 持锁后退出
      当 进程 B 再对该会话发起写占用
      那么 B 取得占用

    场景: 最后干净断开释放
      假如 会话仅一条订阅连接且持有写占用
      当 该连接干净关闭
      那么 写占用已释放且后续同机进程可取得

    场景: 滞留重连同一进程
      假如 唯一订阅异常断开且宽限未到
      当 同一进程再次订阅该会话
      那么 写占用仍在且订阅成功

  @req:r1803
  规则: 下行信封
    下行 必须为 JSON-RPC 2.0 notification（method 与 params，无 id、无客户端作答义务；params 内携带既有 seq 与 payload 语义）。产品下行 method 必须含 session/event、session/subscribed、session/resync_required、session/resources、session/model 与 bash/output；审批与问卷告知为 approval/requested 与 question/requested。客户端 不得对事件 notification 作答。

    场景: 帧序列化
      假如 构造下行 session/event、session/subscribed、session/resync_required 与 session/resources notification
      当 编码为下行通知帧
      那么 各帧均为无作答义务的下行通知且携带 method 语义

  @req:r1804
  规则: WS 只承载 JSON-RPC 文本
    WS /rpc 必须只接受 text 帧承载的 JSON-RPC 上行（含客户端 request 与订阅）。binary 帧 必须被拒绝：服务端以关闭帧 1003 断开且不执行业务。超过 4 MiB 的 text 帧 必须以关闭帧 1009 收口。载体 ping、pong 与 close 允许。

    场景: 订阅帧
      假如 客户端欲以 last_seq 5 订阅会话 s0
      当 服务端在空闲端口上启动
      并且 POST /rpc 调用 subscribe 带 session_id=s0 与 last_seq=5
      那么 应答为产品成功且 result 含 session 与 seq

    场景: WebSocket文本一元受理
      当 服务端在空闲端口上启动
      并且 客户端经 WS /rpc 发送 host.describe JSON-RPC 请求
      那么 WS 回 JSON-RPC 文本 response

    场景: WebSocket二进制帧拒绝
      当 服务端在空闲端口上启动
      并且 客户端经 WS /rpc 发送 binary frame
      那么 服务端以关闭帧 1003 断开且不执行业务

  @req:r1805
  规则: WS 升级
    Host 须在 WS /rpc 挂载 WebSocket 升级。会话选择 不得依赖 /api/v1/session/{id}/ws 或 GET /api/events.mux。缺 Origin 的原生客户端 须可升级。

    场景: WebSocket升级已挂载
      假如 客户端连接 /rpc
      当 server 接受升级
      那么 升级成功
      并且 不把会话绑在 /api/v1/session/x/ws

  @req:r1806
  规则: 单调 seq
    每个会话 须维护单调递增的 u64 序号，每 append 事件递增；seq 须每会话单调递增。会话槽释放后同会话重建 须从释放时的序号水位续接，不得回到 1，也不得在空 journal 下让携带更大 last_seq 的客户端误以为无遗漏。

    场景: 序号单调
      假如 向会话 append 3 个事件
      当 读回事件
      那么 seq 值为 1,2,3（严格递增）

  @req:r2932
  规则: 会话槽释放
    会话槽 须在最后一条事件订阅连接消失且宽限到期后从 Host 的会话表移除，不得随进程常驻；释放前 须确认无进行中的回合且无订阅者。释放 须同时清空该槽的驱动，不得因槽与驱动互相持有而无法回收。释放 不得删除会话数据，也不得改变跨进程重启后序号从头开始的既有行为。

    场景: 释放后重建续接序号
      假如 会话槽已产生若干事件后被释放
      当 同会话以更大 last_seq 重新订阅
      那么 重建槽的下一事件序号大于客户端 last_seq

  @req:r1807
  规则: journal 环形缓冲
    Host 须维护每会话最近 N 个事实事件的环形 journal（默认 1024）；journal 环绕超过客户端 last_seq 时，Host 须向该客户端发出 session/resync_required。容量按条数计，不按字节计。

    场景: 再同步时包裹
      假如 向会话 append 1025 个事实事件（journal 容量=1024）
      当 last_seq=0 的客户端经 POST /rpc 订阅
      那么 server 发送 session/resync_required 因最早事件已丢失

  @req:r1808
  规则: resync 流程
    收到 session/resync_required 后，客户端 须以更新的 last_seq（如 0 全量重放）再次订阅以恢复事件流。

    场景: 再同步恢复
      假如 客户端收到 session/resync_required
      当 客户端以 last_seq=0 再次经 POST /rpc 订阅
      那么 server 从 journal 重放全部可用事件

  @req:r1809
  规则: WS 事件推送
    订阅之后，Host 须为 agent 发出的每个新事件推送 session/event 的 notification，附带会话单调 seq；载体为 WS text 帧中的 JSON-RPC notification，不得用 SSE 或 binary 帧当下行真源。

    场景: WebSocket事件已推送
      假如 客户端已 subscribe 且 prompt 运行
      当 agent 发出 TextDelta 事件
      那么 客户端收到 session/event notification

  @req:r1810
  规则: 冷恢复投影
    Host 须提供会话消息快照 unary（get_messages 或等价）返回该会话 transcript 条目，作为客户端恢复（冷订或切会话）的投影源；journal 实况重放（sr4）不得作为冷恢复的 transcript 投影来源；快照内容 不得被恢复窗内的实况磁带回放污染。断线续传语义仍按 sr4/w5/w6。快照 unary 须支持分页：调用方可给 limit 与 before_entry_id，语义为「从最新一条向前取 limit 条」与「取 before_entry_id 之前的更早 limit 条」；两者缺省时 须仍返回全量以保持既有调用形状。完整消息 须先落库再下行 MessageEnd，使快照在客户端收到完成事件时已可读到该条，不得出现「已收到 MessageEnd 但快照缺该条」的窗口。

    场景: 冷恢复快照投影
      假如 会话 s0 已有 3 条历史条目且客户端无有效 last_seq
      当 冷订客户端调用消息快照
      那么 快照一次返回全部 3 条条目

    场景: 分页从最新向前
      假如 会话已有 500 条条目
      当 客户端以 limit=200 调用消息快照
      那么 返回最新 200 条且首条早于次新页边界

    场景: 完成事件前已落库
      假如 会话产生一条助手消息
      当 客户端收到该消息的 MessageEnd
      那么 消息快照已包含该条

    场景: 回放窗口会不污染快照
      假如 冷订客户端处于恢复窗内且 journal 含实况磁带事件
      当 调用消息快照
      那么 快照内容与磁带回放无关
      并且 断线续传语义仍按 sr4 从 last_seq+1 重放

  @req:r1771
  规则: reverse-rpc-approval
    ApprovalRequired 时，Host 须向该 session 的订阅连接广播 approval/requested notification；首个匹配该 call 的审批 unary 胜出。

    场景: 批准往返
      假如 服务端和已连接的 mux 客户端
      当 agent 执行需要审批的工具
      那么 客户端收到 approval/requested notification
      当 客户端 POST /rpc 调用 approve_tool 且 approved=true
      那么 工具执行继续
      并且 turn 正常结束

    场景: 工具拒绝
      假如 服务端和已连接的 mux 客户端
      当 agent 执行需要审批的工具
      并且 客户端 POST /rpc 调用 approve_tool 且 approved=false
      那么 工具被拒绝
      并且 turn 继续但不包含工具结果

    场景: 审批广播
      假如 agent 发出 ApprovalRequired，call_id 为 xyz
      当 server 检查 session 的 mux 连接
      那么 3 个已连接客户端均收到 approval/requested

  @req:r1772
  规则: reverse-rpc-question
    QuestionRequired 时，Host 须向该 session 的订阅连接广播 question/requested notification；首个匹配该 call 的问卷 unary 胜出。

    场景: 首次胜出
      假如 2 个客户端 POST /rpc 调用 approve_tool，call_id xyz（首个 true，50ms 后 false）
      当 server 处理首个应答
      那么 agent 以 approved=true 恢复；第二个应答被忽略

  @req:r1773
  规则: call-id-lifecycle
    每个审批/问卷 须有稳定 call id（出现在 notification 的 params 中）；超时（默认 60s）时收到 ApprovalTimeout；应答时收到结果；消费后 须移除条目。unary 的 JSON-RPC request id 是幂等键，不得与 call id 混用。

    场景: 超时错误
      假如 60s 内无客户端 POST /rpc 调用 approve_tool
      当 server 将 call_id 标记为 expired
      那么 agent 收到 ApprovalTimeout 错误

  @req:r1774
  规则: no-conflict-arbitration
    首版 不得实现多客户端冲突仲裁；给定 call id 的首个审批/问卷 unary 胜出；同一 call id 的后续 unary 须被静默忽略。

    场景: 第二次作答忽略
      假如 第二个客户端对已消费 call 再 POST /rpc 调用 approve_tool
      当 server 收到该应答
      那么 应答被静默忽略（无状态变化、无错误）

  @req:r1781
  规则: unary 幂等准入
    Host 须以 JSON-RPC request id（字符串形式）为幂等键：同一 session 槽内，同 id 的重复 unary 首次准入获胜，Host 须回放首次执行结果且 不得二次执行；首次仍在处理中的同键重复 须等待首次完成后获得同一结果，不得并行执行。同 id 但 method 或 params 不同的提交 须返回稳定冲突错误（产品码 idempotency_conflict，HTTP 仍 200）。幂等账本 须有界且进程内，不得跨重启持久化。

    场景: 幂等回放首次结果
      假如 客户端以 id R 对某 session 经 POST /rpc 提交方法并得到结果
      当 客户端以相同 id R 重试同一方法
      那么 第二次得到与首次相同的结果且命令仅执行一次

    场景: 幂等冲突不同
      假如 id R 已被某 method 与 params 的提交占用
      当 以相同 id R 提交不同 method 或 params
      那么 返回产品错误且 data.code 为 idempotency_conflict 且不执行

    场景: 幂等在途等待
      假如 id R 的首次命令仍在处理中
      当 相同 id R 的重复请求到达
      那么 等待首次完成并回放同一结果且不并行执行

  @req:r1902
  规则: 产品线协议 JSON-RPC 信封
    产品线协议 必须以 JSON-RPC 2.0 文本承载：WS /rpc 的 text 帧与 POST /rpc 的 JSON body 为唯一产品路径。Host 不得以二进制 body 或 binary 帧承载 /rpc 业务。领域语义（方法表、journal、订阅、审批）必须在该唯一载体上保持。

    场景: JSON-RPC 文本往返
      当 服务端在空闲端口上启动
      并且 客户端经 WS /rpc 发送 host.describe JSON-RPC 请求且 id 为 1
      那么 同一条 WS 收回 JSON-RPC 文本应答且 id 回显为 1

    场景: JSON文本RPC受理
      当 服务端在空闲端口上启动
      并且 POST /rpc 发送 JSON-RPC 2.0 文本
      那么 HTTP 200 且应答为 JSON-RPC 文本 response

  @req:r1905
  规则: 下行通知与 seq 语义平移
    下行 必须为 JSON-RPC notification（method 为 session/event，params 含 seq 与 event）；每会话单调 seq、journal 环形重放、resync、订阅跨回合存活、冷恢复快照投影的既有语义 必须原样保持（与载体格式无关）。transient 模型同步 必须以 session/model（无 seq、不写 journal）承载。固定区 session/resources 与审批/问卷告知在产品路径上 须保持：不消耗 journal seq、不写入 journal、不识别的端 须可忽略且其余行为不受影响。

    场景: 事件携带序号
      假如 产品客户端已订阅会话且 prompt 运行
      当 agent 经订阅发出事实事件
      那么 客户端收到携带单调 seq 的事实事件帧
      并且 流式 tape 帧 不消耗 seq

  @req:r1906
  规则: 动态载荷原文纪律
    形状由运行时决定的载荷（工具参数、工具结果、模型消息内容、自定义条目数据等任意 JSON）必须以 JSON 原文值过线（不做结构化 schema）；协议 不得为其定义结构化形状，也 不得拓宽闭集词表；接收端 必须能原样取回该 JSON 值。工具起始参数往返保真、ToolEnd 失败标记必填等既有载荷语义在产品路径上 必须等价保持。

    场景: 工具启动参数原文
      当 对携带工具参数的 tool_execution_start 帧做往返
      那么 参数 JSON 值往返后语义相等且客户端可再解析

  @req:r1907
  规则: 字段级演进
    产品载荷 必须支持字段级演进：新增字段或事件变体 不得要求 bump protocol 整数；接收端对未知字段 必须忽略，对未知事件变体 必须可降级（落入 Unknown 载体）且 不得 panic。已发布的 type 名与字段名 不得改义或复用。

    场景: 未知变体降级
      假如 构造携带未来事件变体的产品帧
      当 旧词表接收端解码
      那么 变体落入 unknown 载体且不 panic 且其余事件不受影响

  @req:r1921
  规则: session_tree 应答透明与深度安全
    `session_tree` 方法 必须返回 JSON 原文载荷（`{"tree":[...]}` 语义），不得将该方法结果塑形为按树深递归的强类型结构：递归强类型在深树上会耗尽默认 worker 栈而 abort。

    场景: 会话树原文载体
      假如 同一会话有 3 条历史条目
      当 客户端取回该会话树
      那么 result 为 JSON 原文载体（非递归强类型）

  @req:r1922
  规则: session_tree 深度无关解析
    `session_tree` 原文载荷的解析 必须不受 serde_json 默认递归上限（128 层）限制：深树 必须完整还原为领域值，不得在超深时静默降级为空；入站 JSON 的嵌套深度 仍由 2048 层显式守卫约束（超出判 -32700）。

    场景: 会话树深恢复
      假如 同一会话有一条深链树（>默认递归上限）
      当 客户端取回该会话树
      那么 result 完整还原深链（非空降级）

  @req:r1923
  规则: 首轮工具门有界决议
    `arm_tool_freeze` unary 须有界决议：配置了但连不上的 MCP 不得让首轮门无限等待——在门时限（`MCP_FIRST_TURN_GATE_TIMEOUT`）内 settle；超时 须 detach 未连上的 bootstrap 并冻结已 armed 工具子集后返回权威快照（`tools_table_frozen`、`mcp_bootstrap_complete` 反映决议结果）。该 unary 不得在连接无界挂起时返回未冻结快照使客户端无限等待（否则 TUI 首轮死锁、round 永不启动）。

    场景: 武装冻结有界低于挂起MCP
      假如 宿主配置了连不上的 MCP 且会话写者已物化
      当 发起首轮工具门冻结
      那么 门时限内返回冻结快照且引导收口

  @req:r1925
  规则: arm_tool_freeze 租约冲突降级为无租约有界决议
    「arm_tool_freeze」unary 不得占用会话写锁；他进程已持有该会话写占用时 须仍返回权威冻结快照，不得使首轮工具门永久停滞。冻结的是写者内部门、不写会话状态。

    场景: 武装冻结租约空闲低于写入冲突
      假如 会话写租约已被其他写者持有
      当 发起首轮工具门冻结
      那么 不取写占用仍返回冻结快照

  @req:r1926
  规则: attach 生效模型同步不进 journal
    会话绑定期同步写者当前生效模型的「session/model」下行 须为 transient 推送（不落 journal、不消耗 seq）——它附属于订阅者收敛而非会话历史；重复绑定产生的重复「session/model」下行 须语义幂等（消费端徽标收敛不闪变）。会话历史中的模型变更记录由领域层「ModelChange」条目承载，不得由 attach 期同步兼任。

    场景: 模型同步时重绑为瞬时
      假如 会话写者已装配且持有已解析默认模型 fake-model
      当 同一会话被订阅两次
      那么 journal 序号未因绑定递增且两次均收到模型同步事件

  @req:r1928
  规则: 非 JSON-RPC 正文判非法信封
    POST /rpc 的正文 必须为合法 JSON-RPC 2.0 文本（单请求、通知或非空批量）。二进制正文、非 JSON 文本、解析失败的 JSON 与空批量 必须判非法信封（HTTP 400，error.code 为 -32700 或 -32600），不得当业务 unary 执行。

    场景: 非 JSON-RPC 正文为非法信封
      当 服务端在空闲端口上启动
      并且 POST /rpc 发送二进制、纯文本与非法 JSON 等非 JSON-RPC 正文
      那么 各 body 判非法信封 HTTP 400 且不执行业务

  @req:r2894
  规则: 根提交回合成中稳定拒绝
    同一会话回合进行中时，任何连接（含先到端）再发根提交（prompt）须获稳定业务错误 session_busy（HTTP 仍 200 的信封内错误），不得并行第二根对话写历史，不得偷跑第二轮，不得静默转插话。单端产品 TUI 忙碌时 Enter 已映射为插话，不经此守卫回归；插话（steer）与续跑（follow_up）为会话级共享道，回合成中任何连接仍可入队。

    场景: 第二次根忙碌拒绝
      假如 会话回合进行中
      当 任一连接对该会话再发根提交
      那么 得到稳定 session_busy 错误且不产生第二回合并发

  @req:r2895
  规则: 观景态订阅
    会话回合成中时，主机向冷订阅（last_seq=0）的新到端提供观景态：不重放历史 tape，下行流式 tape 冻结，仅放行 AgentEnd 生命周期帧与队列、审批等非 tape 帧。订阅确立应答 须携带绑定时刻回合成况标记 inflight。中段重连（last_seq>0）与空闲绑定不进入观景态；中段重连的当前回合内容 须经订阅应答的 partial 字段补齐，不得依赖 tape 重放。观景端显式重挂（同会话再次订阅）后 须按彼时回合成况重新决定订阅形态，主机不得自动升格。

    场景: 在途冷订阅观察
      假如 会话回合成中且已有一路直播订阅
      当 第二路冷订阅同一会话
      那么 订阅应答标记 inflight 且第二路不收到历史 tape 重放

    场景: 观察传递agent结束仅
      假如 第二路处于观景态
      当 会话产生流式 tape 与 AgentEnd
      那么 第二路仅收到 AgentEnd 而直播端全收

  @req:r2896
  规则: journal 重放定向
    订阅确立时的 journal 重放 须仅投递本次发起订阅的连接（新绑定或同连接重订），在订者不得因他端绑定而收到重放突发；同连接重订 须收到 last_seq 起的事实事件回放。重放 不得包含任何流式 tape。订阅确认帧与 resync 语义维持既有广播。

    场景: 回放定向至新订阅者
      假如 一路订阅在订且 journal 含历史事件
      当 第二路冷订阅
      那么 重放仅达第二路而在订者不收到重放帧

  @req:r3001
  规则: 回环凭据门禁
    产品 Host（`xylitol serve`）须默认启用回环凭据门禁：首次启动在 `~/.xylitol/serve.token`（0600）生成随机凭据并跨重启复用；`~/.xylitol/serve.json`（0600）须携带该凭据。`POST /rpc`、`WS /rpc` 升级与 `GET /healthz` 均须校验 `Authorization: Basic` 凭据；未携带或不匹配须返回 401 且附 `WWW-Authenticate`，不得执行任何业务。WS 升级可接受 `auth_token` 查询参数作为浏览器兜底。客户端 attach 须仅从与目标 URL 匹配的注册文件读取凭据；凭据不得写入日志、会话或工具子进程环境。

    场景: 未带凭据被拒
      假如 Host 已启用回环凭据门禁
      当 不带凭据访问 /healthz
      那么 返回 401 且携带 WWW-Authenticate

    场景: 携带发现凭据可访问
      假如 Host 已启用回环凭据门禁且注册文件含凭据
      当 以注册文件中的凭据访问 /healthz
      那么 返回 200

    场景: 注册文件携带凭据
      假如 Host 已启用回环凭据门禁
      当 读取鉴权注册文件
      那么 文件含 token 且与启动凭据一致
