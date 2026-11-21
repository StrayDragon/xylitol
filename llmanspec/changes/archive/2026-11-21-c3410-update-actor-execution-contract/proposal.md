---
depends_on: []
needs_specs_change: true
title: Actor 执行模型行为合约：会话信箱、三通道队列撤销、会话格式重定版与插件单一脊柱
branch: sdd/c3410-update-actor-execution-contract
base_branch: main
base_sha: 522c922f9d84c9009bc00e6f7639eb7ffc2cf142
---

## Why

现行执行权威是同步 ReAct 状态机加分散的队列实现：根提交的忙碌并发在运行时不变量层被拒（r3304），产品路径经 Queued 方法入队改向；steer / follow_up 分槽规则（r1059）、abort 清场语义（r1028）各自独立，occupancy 会话槽状态机（占用 / 宽限 / 驱逐）约 3.9k 行；并行 model call、跨 resume 的 workflow 执行器、守护 agent 等拓扑被架构文件点名「中间件表达不了」（src/AGENTS.md:266）。产品处于 0.0.0-pre 未公开阶段，选定一步到位重写路线：无兼容层、无双路径、旧数据不迁移，一次性改全部调用点。参照 xai-org/grok-build（纯 Rust、tokio 原语自建 actor、零新框架）的生产形态，以及本仓 1,204 个可执行场景的 BDD 安全网。

## What Changes

- 会话槽升格 SessionActor：每会话一个 actor 与有界命令信箱；单写者由队列顺序消费物理化；occupancy 手写状态机上收删除。
- 提交语义：对话提交保持既有排队路径（prompt 方法表 exec 默认 Queued，忙碌时入队改向，行为与今天一致）；新增逐项撤销（队列项携带 message_id，revoke 语义，QueueUpdate 携带队列项列表）作为唯一新增用户语义；bang 一 / 双叹号保持 Exclusive 忙碌拒绝；slash 控制命令保持 Inline 立即处理——协议方法表既有 exec: Exclusive|Inline|Queued 元数据（xylitol-protocol registry.rs:31 默认 Queued）升格为 SessionActor 信箱路由规则。
- 会话格式重定版：append-only 维持、条目不可变、上下文注入条目一等公民；header 版本等值门保留；LEGACY_SESSION_VERSION 迁移路径删除；未知行 compaction 保留策略在 design 定稿。
- 插件单一脊柱：rutis 进程内插件为唯一能力插件形态；plugins/bin 发现与 CLI 委派保留并升级（清单新增可选 mcp 声明，自动注册为 MCP server，工具统一 mcp__ 管线）；私有 stdio JSON-RPC 工具桥退役。
- 合约改写范围：r2914（执行权威表述随会话 actor 重述）、r2920 / r2921（中间件有界参与保持，handoff 门随 actor 拓扑重述）、r3304（运行时不变量表述随信箱重述）、r1059 / r1028（队列与 abort 语义随信箱统一重写）、agent-session-store 版本演进条款、infra-plugin 桥退役条款。
- thoughtdag 适配契约：双库 lockstep，一次改调用点，不设兼容层；嵌入面（boot / driver / EventStream）概念稳定。

## Capabilities

- agent-runtime（执行模型重写）
- workspace-assembly（装配与执行权威条款）
- agent-session-store（格式重定版与演进策略）
- protocol-app（撤销方法与 QueueUpdate 扩展）
- infra-plugin（私有桥退役、清单 mcp 声明）
- layer-architecture（client / host 表述随 actor 重述，如涉及）

## Impact

- BDD：执行模型章节场景随本合约重写；端 / 协议 / 插件其余场景为回归地板，全程保持绿。
- thoughtdag：随各实施批次 lockstep 适配（嵌入端签名、XyEvent 枚举、清单字段），其 surface-* specs 同步改。
- 主线隔离：本组 change 全部在独立 worktree（xylitol.actor-rewrite-refactor）进行，不占用主线 apply 通道。
- 合并策略：各 change finalize 缺省 squash 单笔入主线；主线合并时机由维护者通知控制，未通知不得合并。

## Further Notes

- 调研底稿见本 change research/ 目录：grok-build actor 形态与 turn 消息流（01）、ReAct 拓扑谱系与行业横评（02）、三轴基线判定（03）、thoughtdag 耦合面（04）、T1 基线自检与改写清单核对（05，含 design §5 处置表的三处修正：r3304 引用漂移更正为 r3322、r2910 随触清在 workspace-assembly 无对象、r3376 并入 r1104 不单独分配）。
- 主线漂移核对（2026-11-21）：规划 squash 6c2b782 之后主线落地的 c3431（aff58d1 + 522c922）未触碰 `llmanspec/specs/` 任何文件，design §5 处置表六处 capability 无重叠无冲突；`llman-sdd spec next-req-id` = r3373 与预留一致，新 req id 实际分配为 r3373 / r3374 / r3375 / r3377（r3376 并入 r1104 改写，未消耗）。
- 【红名单（T6 定稿）】specs 重写后 `--strict` / `--check` 的预期红清单，非失败；清零责任分属本组后续 change，c3415 T9 统一收口：
  1. 步骤文本改写 → 既有场景绑定断裂（场景名未动，步骤断言随载荷重形）：
     - agent-runtime r1060 场景「队列更新」：断言由计数改为 items 载荷（绑定 `tests/bdd/bindings_agent_runtime.rs`）。
     - protocol-app r1719 场景「队列更新线协议往返」：断言由队列计数保真改为队列项列表保真（绑定 `tests/bdd/bindings_protocol_app.rs`）。
  2. 既有场景删除 → 绑定悬空（私有工具桥退役，绑定清理归 c3415）：infra-plugin 七场景——「受信外部插件装配工具」「未受信外部插件不装配」「保留名外部插件不装配」「外部插件握手与调用」「协议版本不匹配拒绝」「进程消亡后工具调用不可用」「失败插件不阻断装配」（`tests/bdd/bindings_infra_plugin.rs` 全量）。
  3. 新场景无 step 定义（待实现）：agent-runtime 十个（「撤销未出队项」「出队后 NotFound」「双客户端队列同步与跨端撤销」「嵌入端与线协议端撤销一致」「print 端无感」「并发撤销恰好一次成功」「两提交一处理一排队」「中段订阅补发队列项」「旧协议版本端被拦」→ c3415；「双请求并行汇聚按发起序」→ c3425）；agent-session-store 两个（「旧版本文件拒载报版本」「同版本文件完整解析」→ c3420）；infra-plugin 三个（「未批准插件不注册工具」「批准后注册为 MCP 工具」「停用覆盖批准」→ c3430）。
  4. 红窗窗口：c3410 归档后至 c3415 落地前主线 BDD 存在短暂红窗，维护者已明确接受（2026-11-20），无需请示。
- 【跨 change 合并顺序备忘（T7）】本组六条 change 全部在独立 worktree（xylitol.actor-rewrite-refactor）进行，finalize 缺省 squash 单笔入主线，合并时机由维护者通知控制。落地顺序（依 design D0-D7 与 tasks 依赖）：
  1. c3410（本 change）合约重写，specs 即实现体；
  2. c3412 XyDriver 表面收窄（剪贴板三件删、attach 六件删、session_store 由 kernel 直供、XySessionOps 独立 trait）；
  3. c3415 SessionActor + 三通道信箱 + 逐项撤销实现，含本红名单清零（T9）与 infra-plugin 旧绑定清理；
  4. c3420 会话格式重定版实施（contextInjected 首批升格、严格演进、commit_compaction 未知行行为首项验证）；
  5. c3425 采样编排 SamplerActor（思考循环防护首个消费者）；
  6. c3430 插件单一脊柱（首用批准、清单 mcp 声明、桥退役实现、thoughtdag describe/install.sh 随之退役）。
  thoughtdag 双库 lockstep 适配随各实施批次同批进行；多端③④⑥场景的 step 归属为 apply 期补充决定，与①②⑤一并归 c3415。

## Open Questions

- 【已决 2026-11-20】信箱容量与背压：现状搬运——SessionActor 内部设小容量有界命令信箱（满→发送方等待，用户无感），既有 steer / follow_up 队列结构原样收编、行为逐字节不变；不设对话队列容量上限、不做 QueueFull（该语义在现行产品路径不存在）。
- 【已决 2026-11-20】XyDriver 表面：actor 内部实现采用门面 + watch 状态投影（同步 getter 读快照、异步语句走信箱回执、abort try_send 永不等待）；trait 签名允许改动、表面收窄一步到位，先经前置 change c3412 完成表面清理再 actor 化；Send 约束由 Sender + watch Receiver 天然满足。
- 【已决 2026-11-20】撤销边界：仅未出队项可撤销（steer 与 follow_up 两队列一致），出队即成事实——steer 注入 turn 上下文、follow_up 成为新 turn 提交，唯一出路 abort；id 入队时分配；revoke_queue_item 为 Inline 方法；QueueUpdate 改带队列项列表；abort / clear_queue / QueueMode 语义不变。
- 【已决 2026-11-20】QueueUpdate 与撤销 wire 形状：items-only 载荷（steer_count / follow_up_count 删除，计数由列表派生）、preview 截断 80 字符、uuid v4 id、revoke_queue_item 为 Inline/Writer 方法（未知 id → kind=NotFound）、未知 kind Unknown 降级、PROTOCOL_VERSION 升版经 host.describe 协商拦旧端。并要求多端验收场景随合约重写落 specs：①双客户端队列同步与跨端撤销（控制面会话级共享，与 clear_queue 同权）②中段订阅经 resync 拿到完整队列项（QueueUpdate 为 journal 事实事件）③嵌入端（driver 直调）与 wire 端语义一致 ④print 端无感不受影响 ⑤并发撤同一 id 竞态（单写者串行化保证一成一 NotFound）⑥PROTOCOL_VERSION 协商拦截旧端验收。
- 【已决 2026-11-20】会话格式演进：严格式——新增条目变体与破坏既有字段语义都升 SESSION_VERSION（等值门保留、LEGACY 迁移路径删除、不引入 Unknown/Opaque 变体）；可选字段增删不升版本（serde 天然穿透未知字段，写进 design 备忘）；不变量一句话：同版本文件必然可完整解析。首批升格条目 contextInjected（注入器 id / point / 内容指纹或预览）；低频观测事实（如 queue_revoked）走 custom 扩展点（todo_checkpoint / workflow 先例）。
- 【已决 2026-11-20】采样编排：采用独立 SamplerActor（每会话槽一个，Submit / Cancel / UpdateConfig / IsActive 协议），按请求取消与运行中配置为预留能力——首个消费者场景为模型思考循环防护（跨 step 循环由 loop 中间件检测裁决，步内流级急停由按请求取消执行）；旁路模型调用（标题生成 / compaction 摘要）统一登记进采样记账；XyModel 端口零改动（&self + Send + Sync 天然并发安全）；热重载维持 r1042 turn 边界原则（同一 turn 全部请求用起始绑定，可 / 不可热换清单随合约重写落 specs）。
- 【已决 2026-11-20】插件信任门：安装 + 首用批准——发现不等于装配，首次工具表纳入时 TUI 弹批准（对齐项目信任 ChoicePrompt 交互），批准状态落盘；config 可随时显式停用且优先级最高；config 来源的 MCP 服务器行为不变。
- skills 展开等 loop 内置项在新循环中的位置。
