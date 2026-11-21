# Design：Actor 执行模型合约

状态：八问深挖已闭环（2026-11-20），本文为决策汇总骨架；标【细化】的条目在 tasks / apply 期展开。

## 1. 目标与非目标

目标：会话槽升格 SessionActor；三通道队列与逐项撤销；会话格式重定版；插件单一脊柱（发现委派保留 + 工具统一 MCP）。一步到位：无兼容层、无双路径、旧数据拒载、双库 lockstep。

非目标：迁移旧会话数据；引入 actix 等框架；兼容别名与双解析路径；端侧（TUI / Print）交互重设计；thought-loop-guard 中间件本体（后续独立 change）。

## 2. 总体形态

- SessionActor（每会话槽一个）：select 主循环路由三通道；turn 任务在其管辖内运行。
- 命令信箱：小容量有界，满 → 发送方等待（纯内部背压，用户无感）。
- XyDriver 门面 + watch 状态投影：同步 getter 读快照（签名级稳定）、异步语句信箱回执、abort try_send 永不等待。
- ReAct 内环、七阶段中间件、XyEvent 词表、rutis 事件面：保持。

【细化】主循环与 turn 任务的 LocalSet / spawn 关系草图（对齐 grok-build run_loop 先例，research/01）。

## 3. 决策记录（D1–D8）

| # | 决策 | 要点 |
|---|---|---|
| D1 | 信箱与背压：现状搬运 | 内部有界信箱 + 既有 steer/follow_up 队列收编，行为逐字节不变；无容量上限、无 QueueFull |
| D2 | XyDriver 表面 | 门面 + watch 投影；签名允许改；前置 c3412 收窄（剪贴板三件删、attach 六件删、session_store 由 kernel 直供、XySessionOps 独立 trait） |
| D3 | 撤销边界 | 仅未出队可撤（两队列一致）；出队即成事实，唯一出路 abort；id 入队分配 |
| D4 | wire 形状 | QueueUpdate items-only（计数删）、preview 80、uuid id；revoke_queue_item Inline/Writer（未知 id → NotFound）；PROTOCOL_VERSION 升版 |
| D5 | 会话格式演进 | 严格式：条目新增与破坏性变更都升 SESSION_VERSION；等值门保留、LEGACY 删、无 Unknown 变体；可选字段增删不升版（serde 天然穿透）；首批升格 contextInjected；低频观测走 custom |
| D6 | 采样编排 | 独立 SamplerActor（每会话槽）；按请求取消 + 运行中配置为预留（首场景：思考循环防护）；旁路调用统一登记；XyModel 端口零改动；热重载维持 r1042 turn 边界原则 |
| D7 | 插件信任门 | 安装 + 首用批准（TUI 批准、状态落盘）；config 显式停用随时可用且优先级最高；config 来源 MCP 不变 |
| D0 | 全局哲学 | 一步到位、无兼容层；双库 lockstep 只有一种新模式；主线隔离于独立 worktree |

## 4. 分节设计要点

### 4.1 三通道映射（D1/D3）
协议方法表 exec 元数据（Queued 默认 / Exclusive / Inline，protocol registry.rs:31）升格为 SessionActor 信箱路由规则。对话通道排队 + 逐项撤销；bang 通道忙碌拒绝；控制通道立即处理。

### 4.2 队列与撤销（D3/D4）
PendingMessageQueue 演进为 VecDeque<QueuedItem{ id, message }>；QueueUpdate 载荷 items-only（id / kind / preview 80）；revoke_queue_item 未知 id 报 kind=NotFound；abort（清 steer 留 follow_up）与 clear_queue（全清）语义不变。

### 4.3 会话格式（D5）
条目清单：现有 10 变体保留 + 首批新增 contextInjected（注入器 id / point / 内容指纹或预览）；queue_revoked 等低频观测走 custom（todo_checkpoint / workflow 先例）；header 字段审计（version / profile 指纹 / cwd）留【细化】；repair_partial_tail 崩溃尾修复保留。

### 4.4 采样编排（D6）
SamplerActor 协议 Submit / Cancel / UpdateConfig / IsActive / ActiveCount；请求子任务 child token + 现有 AutoRetry 梯（r3021）搬入；旁路调用（generate_session_name，react/mod.rs:101 先例）登记；热重载清单【细化】：可热换（skills / context / models）与不可热换（provider 实例 / 密钥）逐项落 specs。

### 4.5 插件信任门（D7）
发现 → describe 清单（commands + mcp 声明）→ 首用批准（持久化）→ 接线；config disabled 优先级最高；批准存储位置与 UI 交互【细化】（对齐 app-tui-trust ChoicePrompt 先例）。

### 4.6 多端验收场景（随合约重写落 specs）
①双客户端队列同步与跨端撤销（控制面会话级共享）②中段订阅 resync 携带完整队列项（QueueUpdate 为 journal 事实）③嵌入端 driver 直调与 wire 端语义一致 ④print 无感 ⑤并发撤同一 id 竞态（一成一 NotFound）⑥PROTOCOL_VERSION 协商拦截旧端。

## 5. specs 绑定方案（planned 期预案；绑定分支后按此执行）

总则：绑定前本节为预案，不触碰 specs/**；新 req id 经 next-req-id 顺序分配（预览起点 **r3373**——r3370-r3372 已被 mainline 的 WorkflowUpdated 保真与渲染降级规则消耗，实际以分配为准）；既有场景名一律不改；载荷耦合场景（r1060 / r1719）步骤文本随形状改写并同步旧绑定——属预期红名单（proposal Further Notes，c3415 T9 清零）。

### 5.1 agent-runtime.feature

| req | 处置 | 要点 |
|---|---|---|
| r3304 | 改写 | 提交一律入会话信箱按序消费；在途时对话提交 = 轮内改向；场景「追问逐条独立回合」「改向之前模型」不动 |
| r1059 | 改写 + 新场景 | 双队列 + OneAtATime 保留；新增撤销句；新场景「撤销未出队项」（入队两条 → revoke 第一条 → 列表仅剩第二条且其不入 turn） |
| r1028 | 改写 | 信箱表述重述；语义不变（取消当前 turn、清 steer、留 follow_up）；场景不动 |
| r1060 | 改写 | 载荷 items（id/kind/preview）；场景「队列更新」步骤改断言 items——预期红名单 |
| r2920 / r2921 | 重述主语 | core 持 turn 边界 → SessionActor 持 turn 边界；handoff 门维持稳定报错；场景不动 |
| 新增 | 采样记账（id 预留 r3373） | 并行模型调用经 SamplerActor 登记 / 取消 / 汇聚 = 发起序；场景：双请求并行时序（step 随 c3425） |
| 新增 | 撤销（id 预留 r3374） | revoke_queue_item 语义；场景「撤销未出队项」「出队后 NotFound」 |
| 新增 | 会话槽单写者物理化（id 预留 r3375） | 信箱顺序消费即单写者；场景：两提交一处理一排队 |
| 其余 | 不动 | r1027 / r1030-r1058 / r2922-r2931 / r3305 等全部保留 |

### 5.2 workspace-assembly.feature

| req | 处置 | 要点 |
|---|---|---|
| r2914 | 重述 | 执行权威 = SessionActor；ReAct 内环仍为不可替换核心插件；场景「无会话无工具仍跑完一轮」不动 |
| r2910 | 随触清 | external_plugins 语句随桥退役删除；rutis 限定条款不动 |

### 5.3 layer-architecture.feature

| req | 处置 | 要点 |
|---|---|---|
| r1514 | 重述主语 | 单写者 = 会话信箱顺序消费；场景「第二次写者拒绝带冲突」不动 |
| r1535 等 | 不动 | |

### 5.4 agent-session-store.feature

| req | 处置 | 要点 |
|---|---|---|
| r1097 | 改写 | 条目类型清单 + contextInjected（injector_id / point / 指纹或预览） |
| r1104 | 改写或删并新立 | 旧格式边界 → 格式严格演进：加条目与破坏性变更必升 SESSION_VERSION；等值门拒载旧版；可选字段增删不升版（serde 穿透）；LEGACY 删除 |
| 新增 | 格式严格演进（id 预留 r3376，可与 r1104 改写合并） | 场景：旧版本文件拒载（错误含版本号）；同版本文件完整解析 |
| 其余 | 不动 | r41 / r1090 / r1102 / r1103 / r1105-r1111 / r1900 / r1901 / r1094 |

### 5.5 infra-plugin.feature

| req | 处置 | 要点 |
|---|---|---|
| r2991 | 删除 | 工具桥退役 |
| r2992 | 删除 | 桥失败隔离随桥退役（核准消费者后再删） |
| r2990 | 改写 | 配置与信任 → 首用批准（ChoicePrompt、批准持久化）+ config disabled 优先级最高 |
| r3074 | 改写 | config section → plugins.disabled / plugins.enabled 新形状 |
| r3027 / r3317 | 核对后保留 | 委派与命令捕获语义不变 |
| 新增 | 清单 mcp 声明（id 预留 r3377） | 场景：未批准不注册工具；批准后注册；disabled 覆盖批准 |

### 5.6 protocol-app.feature

| req | 处置 | 要点 |
|---|---|---|
| r1691 | 改写 | Command 面 + revoke_queue_item 命令（Inline / Writer 语义在 registry 层） |
| r1719 | 改写 | 线协议映射队列与闭集 → items 形状——预期红名单 |
| r1698 | 核对 | 新变体登记（mainline 已登记 WorkflowUpdated，r3370——本组无新增 XyEvent 变体，仅 QueueUpdate 载荷重形） |
| r1716 | 核对 | 队列深度方法 queue_stats 保持只读；revoke_queue_item 为新增 Writer / Inline 方法，二者并存 |
| 其余 | 不动 | r1693 / r1695 / r1697 / r1699 / r1700 / r1692 / r1694 |

### 5.7 步骤与绑定预期

- 新场景 step 定义与 bindings 归属：撤销 / 队列 items / 多端①②⑤ → c3415；采样记账 → c3425；信任门与清单 → c3430；格式演进 → c3420。
- 红名单：r1060 / r1719 步骤改写 + 新场景无绑定——清单记 proposal Further Notes。

## 6. 风险与回归地板

- BDD：执行章节场景随本合约重写；其余场景（端 / 协议 / 插件行为）为回归地板，全程护绿。
- 双库 lockstep：嵌入面签名 / XyEvent 枚举 / describe 清单字段 / 新会话格式——每批次 thoughtdag 同步适配。
- 已知待验证：commit_compaction 对未知行的实际行为（c3420 实施首项）；剪贴板 / attach 六件删除前二次普查（c3412 实施首项）。
