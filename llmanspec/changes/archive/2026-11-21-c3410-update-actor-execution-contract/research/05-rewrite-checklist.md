# T1 产物：基线自检与六处 capability 改写清单核对

核对时间：2026-11-21（apply 期，分支 sdd/c3410-update-actor-execution-contract，base=main@522c922）。

## 一、基线自检

- HEAD：main 522c922（c3431 归档提交）；planning squash 基线 6c2b782。
- `llman-sdd validate --all`（非严格）：66/66 绿（spec 60/60、change 6/6）。
- `llman-sdd validate --all --strict`：spec 60/60 绿、change 6/6 红——本组六条 change tasks 未勾，属本 change 自身范围，不构成 blocker（lead 已核实）。
- BDD 绑定面：60 capability、1,204 处场景绑定（research/03 锚点，结构未变）。

## 二、c3431 主线漂移检查（lead 预警项）

`git diff 6c2b782..HEAD -- llmanspec/specs/` 为空：c3431（aff58d1 草案 + 522c922 归档）**未触碰任何 spec 文件**，
其对 design §5 处置表六处 capability 无影响，无重叠、无结构性冲突。

- `llman-sdd spec next-req-id` = **r3373**；r3370-r3372 已被 mainline WorkflowUpdated（protocol-app:414）与渲染降级（app-tui-host:427/436）占用。
- design §5 预留区间 r3373-r3377 与当前池一致：**无 id 漂移**，按预留直接分配（r3376 经下述合并后实际不消耗）。

## 三、design §5 处置表核对修正（以现状为准）

1. **r3304 引用漂移（修正）**：planning 基线 6c2b782 上 r3304 即 `auto-retry-event-semantics`（重试代数事件语义），
   与 §5.1 r3304 行描述的「忙碌并发拒绝 / 信箱按序消费 / 轮内改向」语义无关。该语义现行落点是
   **r3322 `one-submission-one-turn`**（agent-runtime.feature:98，含「追问逐条独立回合」「改向之前模型」两场景锚点，与 §5.1 要求的场景不动清单吻合）。
   处置修正：**改写对象 r3304 → r3322**（加信箱表述）；r3304 本体不动。proposal Why 中「忙碌并发被拒（r3304）」同此更正为 r3322 与 server-core r2894 的现行语义。
2. **§5.2 r2910 随触清**：现行 workspace-assembly 全文无 `external_plugins` 语句（全 specs 仅 infra-plugin r2990 文本内一处），随 r2990 改写自然清除，workspace-assembly 无额外动作。
3. **§5.4 r3376 并入 r1104**：design 允许「可与 r1104 改写合并」；采用合并——格式严格演进直接改写 r1104，r3376 编号不消耗。实际分配区间为 r3373-r3375、r3377 共四个新 id。
4. **多端验收场景①-⑥挂靠**（tasks T3 要求全落 agent-runtime.feature）：
   - ①双客户端同步与跨端撤销、③嵌入端与 wire 端一致、④print 无感、⑤并发撤销竞态 → 挂 **r3374**（撤销规则）。
   - ②中段订阅 resync 完整队列项（QueueUpdate 为 journal 事实）、⑥PROTOCOL_VERSION 协商拦旧端 → 挂 **r1060**（QueueUpdate 载荷规则，与 D4 一体决策）。
5. **§5.6 核对结果**：r1698 现行文本已含 QueueUpdate / WorkflowUpdated 闭集，本组无新增 XyEvent 变体、载荷形状不在其文本内 → 不动；
   r1716 queue_stats 只读与新增 revoke_queue_item（Inline/Writer）并存无冲突 → 不动；r1692/r1693/r1695/r1697/r1699/r1700/r1694 不动。
6. **§5.5 消费者核准**（r2992「核准消费者后再删」）：infra-plugin 全部 7 个场景绑定集中于 `tests/bdd/bindings_infra_plugin.rs`（7 个 #[scenario]），
   无其他消费方。r2991/r2992 删除连带 6 个旧场景删除（r2990 改写连带 3 个旧场景删除），绑定悬空进红名单，bindings 清理归 c3415。
7. **r3027 / r3317 核对**：commands 通道（commands/call 透传）与 run_command_capture 助手属 CLI 委派面，与工具桥退役无冲突 → 保留不动。
8. **范围外张力（记录，不动）**：server-core r2894「根提交回合成中稳定拒绝（session_busy）」描述的忙碌拒绝与信箱排队表述存在既有张力；
   server-core 不在本 change 六 capability 范围（proposal Capabilities 未列），留给本组后续 change（c3415 或专项）处置。

## 四、逐 req 改写清单（含现行文本摘录）

### agent-runtime.feature

| req | 现行（摘录） | 处置 |
|---|---|---|
| r3322 | 「每次提交（含 follow_up FIFO 弹出）须产生独立 turn…队列空时才进入空闲。」 | 改写：段首加「会话提交须一律进入会话信箱按序消费」（信箱主语）；场景「追问逐条独立回合」不动 |
| r1059 | 「须提供独立的 steer 与 follow_up 队列（enqueue/drain/clear）；缺省 QueueMode / SteeringMode 须为 OneAtATime…不得 drain steer 或 follow_up。」 | 改写：保留全部既有句 + 信箱收编表述 + 新增撤销句（队列项携带入队分配 id、仅未出队可撤销）；场景「改向之前模型」「应当停止跳过追问」不动；新场景「撤销未出队项」 |
| r1028 | 「abort 须取消当前 turn、清空 steer 并保留 follow_up FIFO…不得仅靠无人订阅的 EventBus 旁路传队列状态。」 | 改写：abort 立即投递会话信箱不等待回执；语义不变；场景「中止清除改向」「中止取消bang」不动 |
| r1060 | 「入队/drain/clear 后 须发射 XyEvent::QueueUpdate（steer_count / follow_up_count）。」 | 改写：载荷 items-only（id/kind/preview 80），计数删除由列表派生；QueueUpdate 为 journal 事实（resync 补发）；场景「队列更新」步骤改断言 items（红名单）；新场景「中段订阅补发队列项」「旧协议版本端被拦」 |
| r2920 | 「…core 须永远持有 turn 边界、取消、权限最终裁决与消息落盘格式，不得允许插件整段替换 ReAct。」 | 重述主语：core → SessionActor；场景「中间件有序且卸载后消失」「空链走基线」不动 |
| r2921 | 「…handoff 本 change 未支持，core 须以稳定 kind LoopMiddlewareHandoffUnsupported 的错误结束本 turn…下一次 generate 只能由 core 调度。」 | 重述主语：core → SessionActor（两处）；场景三件不动 |
| 新 r3373 | —（新增） | 采样记账：并行模型调用经每会话采样 actor 登记/注销，登记序=发起序，旁路调用统一登记；场景「双请求并行汇聚按发起序」（step 归 c3425） |
| 新 r3374 | —（新增） | revoke_queue_item 语义：仅未出队可撤（两队列一致）、出队即成事实、NotFound、id 入队分配、并发撤销串行化一成一 NotFound；场景「撤销未出队项」「出队后 NotFound」「双客户端队列同步与跨端撤销」「嵌入端与线协议端撤销一致」「print 端无感」「并发撤销恰好一次成功」（step 归 c3415） |
| 新 r3375 | —（新增） | 会话槽单写者：信箱到达序逐条处理、同一时刻至多一个 turn 在写、第二路提交排队；场景「两提交一处理一排队」（step 归 c3415） |
| 其余 | r1027/r1030-r1058/r2922-r2931/r3304/r3305 等 | 不动（r3304 经第三节修正明确不动） |

### workspace-assembly.feature

| req | 现行（摘录） | 处置 |
|---|---|---|
| r2914 | 「ReAct 循环必须留在执行权威，并作为不可经 loader rows 替换的核心插件…」 | 重述：执行权威=每会话一个 SessionActor、有界信箱按序消费；场景「无会话无工具仍跑完一轮」不动 |
| r2910 | rutis 限定条款（无 external_plugins 语句） | 不动（第三节第 2 条） |

### layer-architecture.feature

| req | 现行（摘录） | 处置 |
|---|---|---|
| r1514 | 「同会话第二路写入必须被稳定处理，不得靠协议写者令牌误伤同进程第二客户端。」 | 重述：加信箱顺序消费物理化单写者表述；场景「第二次写者拒绝带冲突」不动 |
| r1535 等 | 其余全部 | 不动 |

### agent-session-store.feature

| req | 现行（摘录） | 处置 |
|---|---|---|
| r1097 | 「会话存储 须支持条目类型…：message、compaction、branchSummary、modelChange、thinkingLevelChange、custom、customMessage、label、sessionInfo；bang-bash…」 | 改写：清单新增 contextInjected（注入器 id、注入点、内容指纹或预览）；场景四件不动 |
| r1104 | 「仅存在旧 v6/v5 或未知单文件…System 不得自动迁移、不得静默删除…list 须跳过该条目并记录有限诊断…」 | 改写：格式严格演进（升 SESSION_VERSION、等值门拒载且错误含版本号、LEGACY 删除、可选字段增删不升版、同版本文件必然可完整解析）；保留 list 跳过语义；场景「列表跳过旧版仅会话」不动；新场景「旧版本文件拒载报版本」「同版本文件完整解析」（step 归 c3420） |
| 其余 | r41/r1090/r1102/r1103/r1105-r1111/r1900/r1901/r1094 | 不动 |

### infra-plugin.feature

| req | 现行（摘录） | 处置 |
|---|---|---|
| r2990 | 「仅 external_plugins 中 trusted: true…须装配；未 trusted、保留名、重复名 须跳过并产生可观察 warn…」 | 改写：plugins/bin 发现 + describe 清单 + 首用批准（批准持久化、对齐信任选择交互）+ config 显式停用优先级最高；旧场景三件删除（trusted 装配语义随 external_plugins 配置节退役） |
| r2991 | 「外部进程 须经 newline-delimited JSON-RPC 2.0 stdio 握手：initialize…tools/call 结果 须以文本内容回传…」 | 删除：私有工具桥退役；场景三件删除 |
| r2992 | 「单个外部插件 spawn 或握手失败 须 warn…插件卸载 须移除其注册的工具并终止外部进程…」 | 删除：桥失败隔离随桥退役；场景一件删除（消费者已核：bindings_infra_plugin.rs） |
| r3074 | 「外进程插件 须 可在 initialize 中声明 config_key 与 config_schema…」 | 改写：加 plugins.enabled / plugins.disabled 集合形状、external_plugins 节与 trusted 字段不保留；config_key 机制句保留；无场景（裸规则） |
| r3027 | commands 声明与 commands/call 透传 | 不动 |
| r3317 | run_command_capture 助手 | 不动 |
| 新 r3377 | —（新增） | 清单 mcp 声明：声明者注册为 MCP server、工具统一 mcp__ 管线；未批准不注册、批准后注册、停用覆盖批准；场景三件（step 归 c3430） |
| purpose | 「…stdio 握手、工具桥与失败隔离…」 | 更新：发现与清单（含 mcp 声明）、首用批准、CLI 委派；工具统一 MCP 管线 |

### protocol-app.feature

| req | 现行（摘录） | 处置 |
|---|---|---|
| r1691 | 「protocol::Command 须包含 Steer、FollowUp 与 ClearQueue（或语义等价变体）…」 | 改写：Command 面加 RevokeQueueItem；revoke_queue_item 为 Inline/Writer、未知 id → kind=NotFound；场景「命令队列变体解析」不动 |
| r1719 | 「protocol Event 须能表达跨面所需的 XyEvent 闭集子集（至少含 QueueUpdate）；未映射变体 须可降级忽略…」 | 改写：QueueUpdate 线载荷 items-only（id/kind/preview 80），不得镜像计数标量，未知 kind 降级 Unknown；场景「队列更新线协议往返」步骤改断言队列项（红名单） |
| r1698 | Event 变体完整（已含 QueueUpdate/WorkflowUpdated） | 不动（核对通过） |
| r1716 | queue_stats 只读 | 不动（核对通过：与新增 Writer 方法并存） |
| 其余 | r1693/r1695/r1697/r1699/r1700/r1692/r1694 | 不动 |

## 五、红名单预告（T6 定稿后全文记入 proposal Further Notes）

1. 步骤改写绑定断裂：r1060「队列更新」（bindings_agent_runtime.rs:41）、r1719「队列更新线协议往返」（bindings_protocol_app.rs:182）。
2. 场景删除绑定悬空：infra-plugin 7 场景（bindings_infra_plugin.rs 全量，随 r2990 改写 / r2991 / r2992 删除）。
3. 新场景无 step 定义（待实现，分属 c3415/c3420/c3425/c3430）：agent-runtime 10 个（撤销 6 + 单写者 1 + 采样 1 + QueueUpdate 2）、agent-session-store 2 个、infra-plugin 3 个。
