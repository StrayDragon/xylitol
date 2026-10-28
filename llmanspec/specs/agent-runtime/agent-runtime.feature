# language: zh-CN
# purpose: "薄编排 ReAct 运行时：AgentRuntime 循环、工具批执行、steer/follow-up 队列、abort 与 XyEvent 流（不含 adk / OutputGuard）。"

功能: agent-runtime

  @req:r1027
  规则: thin-react-loop
    agent/ 层 须以 AgentRuntime 驱动 ReAct：调用 XyModel、执行工具、直至无 tool_calls、abort、或可选 should_stop_after_turn 请求停；不得以产品默认 max_iterations 或未配置的硬步数闸结束 run；可选 session.max_turns（见 ar30）须仅通过 should_stop_after_turn 生效。不得在循环实现内联 bash 执行或厂商 HTTP。

    场景: ReAct结束
      假如 mock 模型先 tool 后无 tool
      当 运行 AgentRuntime
      那么 先执行工具再结束且无 adk 类型
  @req:r1036
  规则: xy-event-stream
    运行时 须以 XyEvent 异步流对外输出（TextDelta / ThinkingDelta / Tool* / Turn* / QueueUpdate / Error 等）；不得再引入 AgentEvent 或 adk 事件类型。

    场景: 流为XyEvent
      假如 消费事件流
      当 轮询
      那么 每项为 XyEvent
  @req:r1047
  规则: 工具批后继续
    同一模型响应中的全部工具调用 须在下一轮模型调用前执行完毕（可并行或串行）；不得因流结束 Done 而提前结束含 tool_calls 的回合。该续跑 须关闭本 iteration（与 TurnStart 成对的 TurnEnd），不得因此做 ContextTokenSettlement、auto-compact 预检或 should_stop_after_turn。该行为 须有可执行 BDD 场景（live `.feature`，`@req`）。

    场景: 继续之后工具
      假如 mock 模型先 tool 后无 tool
      当 运行 AgentRuntime
      那么 turn_end 事件包含 toolResult
  @req:r1038
  规则: tool-intent-before-execution
    ReAct 消费 XyChunk::ToolCallStart/Delta/End 时：流式阶段 须经 MessageUpdate（message 含渐进 ToolCall 部分）暴露工具意图；不得在流未结束（MessageEnd 前）发射 ToolExecutionStart 或执行工具副作用。ToolExecutionStart/Update/End 须仅在 MessageEnd 之后的执行路径发出。该行为 须有可执行 BDD 或等价单测场景。

    场景: 意图之前执行
      假如 mock 模型先 tool 后无 tool
      当 运行 AgentRuntime 并收集事件
      那么 MessageUpdate 含工具意图且早于任意 ToolExecutionStart；ToolExecutionStart 不早于 MessageEnd
  @req:r1039
  规则: system-via-generate-options
    配置的 system prompt 须经 XyGenerateOptions.system_prompt 传给模型适配层；不得将 system 正文作为 AgentMessage::user 写入会话 history（空会话首条 须为真实用户输入）。由 ReAct 单测覆盖，不得为静态存在性单独扩 BDD step。

    场景: 首次回合历史为真实用户
      假如 配置了 mock 模型 "test-model"
      当 运行一轮对话后检查持久化条目
      那么 首条消息条目为真实 user 输入且无 system 角色行
  @req:r1055
  规则: builder-ports
    Agent 装配 须经 AgentBuilder，依赖 protocol 端口与 agent 会话词汇；app 组合根注入具体 infra 实现。装配入口由单元测试 / 组合根覆盖，不得为静态存在性单独扩 BDD step。
    场景: 构建器端口驱动一回合
      假如 mock 模型先 tool 后无 tool
      当 运行 AgentRuntime 并收集事件
      那么 先执行工具再结束且无 adk 类型

  @req:r1056
  规则: sessionstore-eventsink
    protocol 须提供 SessionStore 与 EventSink；SessionManager / EventBus 为实现。端口存在性由类型系统与组合根覆盖，不得为 trait 存在性单独扩 BDD step。
    场景: 端口产出Xy事件
      假如 消费事件流
      当 轮询
      那么 每项为 XyEvent

  @req:r1057
  规则: mutable-slots-next-turn
    set_tools / set_hooks / set_system_prompt 等 须只影响下一轮 run；进行中回合使用启动快照。
    场景: 热已应用槽重建提示词
      假如 Agent 能力聚合体已就绪
      当 热应用新的上下文与系统提示资源
      那么 系统提示按新资源重建

  @req:r1058
  规则: hooks-at-tool-boundary
    工具执行前 须跑 before 链（可拒绝）；之后 须跑 after 链；无 hook 时零开销。

    场景: 之前拒绝
      假如 注册匹配 bash 的 before 拒绝 hook
      当 运行 AgentRuntime 触发 bash
      那么 tool-error 回写且未执行
  @req:r1059
  规则: pending-queues
    须提供独立的 steer 与 follow_up 队列（enqueue/drain/clear）；缺省 QueueMode / SteeringMode 须为 OneAtATime（一次 drain 一条；配置可显式 all）；ReAct 须在模型调用前 drain steer；在将因无 tool_calls 而结束前 drain follow_up（非空则继续）；若 should_stop_after_turn 已请求停，不得 drain steer 或 follow_up。

    场景: 改向之前模型
      假如 装配并运行入队 steer 的 agent
      当 检查队列与历史
      那么 steer 计数归零且已处理

    场景: 追问延长
      假如 装配无工具 agent 并入队 follow_up
      当 运行至将结束
      那么 继续循环而非 AgentEnd

    场景: 应当停止跳过追问
      假如 入队 follow_up 且 should_stop_after_turn 在首次 TurnEnd 后返回 true
      当 运行 AgentRuntime
      那么 本 run 以 AgentEnd 结束且 follow_up 未被注入历史
  @req:r1060
  规则: queue-update-events
    入队/drain/clear 后 须发射 XyEvent::QueueUpdate（steer_count / follow_up_count）。

    场景: 队列更新
      假如 装配 agent 并入队 steer
      当 观察事件流
      那么 出现 QueueUpdate 且计数正确
  @req:r1028
  规则: abort-queue-semantics
    abort 须清空 steer、保留 follow_up；须取消当前 run CancellationToken 与进行中工具/交互 bash；不得仅靠无人订阅的 EventBus 旁路传队列状态。

    场景: 中止清除改向
      假如 装配 agent 并入队 steer 与 follow_up
      当 abort
      那么 steer 空且 follow_up 保留

    场景: 中止取消bang
      假如 启动交互 bang 长命令后 abort
      当 检查 bash 结果
      那么 cancelled 为 true
  @req:r1029
  规则: abort-resets-token
    每次 run 开始 须安装新 CancellationToken；abort 后下一次 run 不得因沿用已取消 token 立刻 aborted。

    场景: 第二次运行之后中止
      假如 装配慢速 agent 并在首轮 abort 后
      当 再次运行
      那么 正常完成而非立即 aborted
  @req:r1030
  规则: abort-cancels-provider-stream
    模型流进行中 abort 须停止 poll 并 drop provider stream，以 aborted 结束；应用面 须经 Driver::abort。该行为 须有可执行 BDD 场景（live `.feature`，`@req`），并复用既有 abort step 词表。

    场景: 中止丢弃SSE
      假如 配置了 mock 模型 test-model 且慢速流式 40 段间隔 20 毫秒
      当 经 Driver 启动会话并在首个 TextDelta 后 abort
      那么 事件流包含 aborted 错误
  @req:r1031
  规则: unknown-event-degrade
    应用面/bridge 对未识别 XyEvent 变体 须降级（日志），不得 panic。由 protocol 单元测试覆盖。
    场景: 未知队列类型降级
      当 对 QueueUpdate 事件做线协议序列化与反序列化往返
      那么 队列计数保真且未知 type 落入 Unknown 而非 panic

  @req:r1032
  规则: defaults
    未显式配置时 须有可测默认：thinking level、compaction 频率等集中定义；不得将 max_iterations 或未配置的硬步数闸列为运行时默认。由单元测试覆盖。
    场景: 集中缺省思考档位
      假如 Settings.default_thinking_level 为 high 且模型支持集为 off 与 high 与 max
      当 会话首次装配
      那么 当前 thinking level 为 high

  @req:r1035
  规则: Driver context 重载缝
    InProcessDriver（经 xylitol-runtime Trust 门禁）须能在 Trust 语义下重载磁盘 context/SYSTEM/APPEND 并应用到 agent；未信任 不得注入项目侧 context；重载 不得清空 transcript。

    场景: 上下文重载信任门控
      当 在项目目录写 AGENTS.md 并分别以信任与未信任重载 context
      那么 信任时项目 context 注入而未信任时被跳过且会话条目不变
  @req:r1037
  规则: Driver skills 重载缝
    InProcessDriver（经 xylitol-runtime Trust 门禁）须能在 Trust 语义下重载磁盘 skills 并应用到 agent；须能查询当前已加载 skill 名（供后续 $ 展开）；未信任 不得注入项目 skills；重载 不得清空 transcript；本要求 不得依赖 /session 或 /status skills UI。

    场景: 技能重载信任门控与可查询
      当 在项目目录写 skill 并分别以信任与未信任重载 skills
      那么 信任时项目 skill 注入且可查询已加载名而未信任时被跳过
  @req:r1040
  规则: persist-done-usage
    ReAct 在模型流正常结束时 须将 XyChunk::Done 携带的 usage 与 stop_reason（若有）写入随后持久化的 AssistantMessage；不得在 Done 已提供非空 usage 时仍硬编码 usage: None。该行为 须有可执行 BDD 或等价单测场景。
    场景: 完成用量锚点已持久化
      假如 会话条目含 stop 回合的 usage 锚点
      当 以同源估计器估计上下文
      那么 估计来源为 Api

  @req:r1041
  规则: should-stop-after-turn
    ReAct 须在 Settle（本轮模型调用不再续跑工具）的 TurnEnd 之后、轮询 steer/follow-up 或开始下一模型调用之前，调用可选的单槽 should_stop_after_turn（对齐 pi shouldStopAfterTurn）；返回 true 时 须发射 AgentEnd 并结束本 run，不得为此新增专用停闸 XyEvent 变体，不得 abort 本 turn 已完成的助手消息或工具。工具续跑的 iteration TurnEnd（ContinueTools）不得调用该钩子。未注册时 须不因步数上限停止。该行为 须有可执行 BDD 场景（live `.feature`，`@req`）。

    场景: 应当停止发出agent结束
      假如 注册 should_stop_after_turn 在首次 TurnEnd 后返回 true
      当 运行 AgentRuntime
      那么 出现 AgentEnd 且其后无新的模型轮 TurnStart

    场景: 无钩子打开结束
      假如 未注册 should_stop_after_turn 的无工具 agent
      当 运行 AgentRuntime
      那么 正常出现 AgentEnd 且恰好一轮 TurnStart
  @req:r1042
  规则: next-turn-refresh-model-thinking
    ReAct 须在每次模型调用前（turn 边界，对齐 pi prepareNextTurn）从 session 重读当前选中模型与 thinking level，并据此重建本 turn 的 XyModel 与 XyGenerateOptions（或等价绑定）；不得在整个 run 内握死首帧 model/thinking snapshot。已开始的 in-flight generate_stream 须继续使用该次调用开始时的绑定，不得中途拆流。run 结束（AgentEnd）或 abort 后无 in-flight 时，active 与 selected 须收敛。该行为 须有可执行 BDD 或等价单测场景。
    场景: 下一回合重读模型与思考
      当 以桥缝注入元数据事件（模型选择与 thinking 档位）
      那么 模型不 panic 且条目与相位不变

  @req:r1043
  规则: abort-persist-skip-llm
    用户 abort 中断模型流时，ReAct 须将已累积的 partial assistant（非空 content）以 stop_reason=aborted 持久化进 session history；project_for_llm（或等价 LLM 投影）须跳过 stop_reason 为 aborted 或 error 的 assistant 行，不得将其作为下一轮 provider 输入。空 content 的 aborted MAY 不落盘。该行为 须有单测或 BDD 覆盖。

    场景: 中止持久化部分带已中止原因
      假如 配置了 mock 模型 test-model 且慢速流式 40 段间隔 20 毫秒
      当 经 Driver 启动会话并在首个 TextDelta 后 abort 并检查持久化条目
      那么 partial assistant 以 aborted 落盘且非空
  @req:r1044
  规则: tool-batch-default-barrier-parallel
    未显式配置工具批模式时，ReAct 须对同一 MessageEnd 后的 tool call 批采用 barrier_parallel（连续 ParallelSafe 扇出；Barrier 先汇聚再串行）调度；不得默认退回整批串行（除非显式 mode=sequential）。该行为 须有可执行 BDD 场景（live `.feature`，`@req`）。

    场景: 批缺省屏障并行
      假如 未配置工具批模式且 mock 模型同 turn 发出两个可并行假工具
      当 运行 AgentRuntime
      那么 两工具执行时间重叠
  @req:r1045
  规则: tool-batch-barrier-parallel
    当工具批模式为 barrier_parallel（含产品缺省）时，ReAct 须按 assistant 源序将连续 ParallelSafe 工具收入并行窗扇出执行，遇 Barrier 工具（写/副作用/未知/一切 mcp_ 前缀工具等）须先汇聚（await 齐）当前窗再串行执行该屏障工具，然后继续；不得把屏障之后的 ParallelSafe 提前并入屏障之前的并行窗；不得在 MessageEnd 前执行工具（ar21）；不得经配置/glob/annotation 将 mcp_ 工具升为 ParallelSafe。该行为 须有可执行 BDD 或等价可控假工具单测/BDD 场景。

    场景: 批屏障并行重叠
      假如 工具批模式为 barrier_parallel 且 mock 同 turn 发出两个 ParallelSafe 慢假工具后接一个 Barrier 假工具
      当 运行 AgentRuntime
      那么 两 ParallelSafe 执行时间重叠且均在 Barrier 开始前结束

    场景: 批屏障保留来源窗口
      假如 工具批模式为 barrier_parallel 且 mock 同 turn 工具序为 ParallelSafe、Barrier、ParallelSafe
      当 运行 AgentRuntime
      那么 第二个 ParallelSafe 不与第一个 ParallelSafe 同窗并行且 在 Barrier 完成之后开始

    场景: 批MCP从不并行
      假如 工具批模式为 barrier_parallel 且 mock 同 turn 工具序为 ParallelSafe、mcp 假工具、ParallelSafe
      当 运行 AgentRuntime
      那么 mcp 假工具与两侧 ParallelSafe 均无执行时间重叠
  @req:r1046
  规则: tool-batch-history-source-order
    无论 sequential 或 barrier_parallel，写入 session history 与发往模型的 toolResult 须严格按 assistant 源序；ToolExecutionEnd（及 Update）MAY 按完成序交错。该行为 须有可执行 BDD 或等价单测场景。

    场景: 批历史来源顺序
      假如 工具批模式为 barrier_parallel 且并行窗内后发先完成
      当 检查 session history 中 toolResult
      那么 toolResult 顺序与 assistant 源序一致
  @req:r1048
  规则: config-max-turns-via-should-stop
    当运行时配置 session.max_turns（正整数 N）存在时，组合根 须在装配后安装 should_stop_after_turn：于 Settle 的 TurnEnd 后若 settle 次数 >= N 则返回 true 结束 run；缺省或未配置时 不得安装该步数钩子（开放结束，见 ar24）。不得把工具续跑的 iteration TurnEnd 计入额度，不得使用 max_iterations 字段名。该行为 须有可执行 BDD 或等价单测场景。

    场景: 最大回合停止运行
      假如 按 session.max_turns=2 安装 should_stop_after_turn 且入队 follow_up 以迫使第二轮
      当 运行 AgentRuntime
      那么 至多出现 2 次 TurnStart 后出现 AgentEnd
  @req:r1843
  规则: iteration-close-vs-settle
    一次用户触发的 run（观测根 agent.turn，AgentStart…AgentEnd）内，每一轮模型 generate 及其工具批是一次 iteration（观测 agent.iteration，XyEvent TurnStart/TurnEnd 成对）。本轮仍有 tool_calls、将再 generate 时 须以 ContinueTools 关闭 iteration：发 TurnEnd 配成对，不得 ContextTokenSettlement、不得 auto-compact 预检、不得 should_stop_after_turn。本轮不再要工具时 须 Settle：TurnEnd + settlement + threshold/overflow 预检 + should_stop。generate 失败走 overflow Case1 的收尾仍为 Settle。不得用「跳过 TurnEnd」或「导出侧去重 skipped」代替这组穷举。由单测覆盖，不得为静态存在性单独扩 BDD step。

    场景: 工具回合两轮次一结算
      假如 mock 模型先 tool 后无 tool
      当 运行 AgentRuntime 并收集事件
      那么 TurnStart 与 TurnEnd 成对出现两次且 ContextTokenSettlement 恰一次
  @req:r1049
  规则: turn-end-threshold-compaction
    ReAct 或 session 编排在 Settle（本轮不再续跑工具、非 abort）后 须调用 threshold auto-compact 检查（domain-compaction c2 地板感知有效触发阈值 + c17/c18）；ContinueTools、CompactionSettings.enabled 为 false、未超有效阈值、abort、或 stale 守卫命中时 不得 compact；不得仅依赖 TUI host 轮询触发。该行为 须有可执行 BDD 或等价单测场景。
    场景: 回合结束阈值自动压缩
      假如 会话叶上存在可信 Api usage 锚点且 footer 同源估计可用
      当 执行 auto-compact reserve 触发判断
      那么 所用 token 数字与同源估计一致且 不另算独立启发式总和
      并且 触发比较式为占用大于有效触发阈值 max(window 减 reserveTokens, 压后地板 加 迟滞带)

  @req:r1050
  规则: turn-end-overflow-compact-retry
    ReAct 在 Settle（或 generate 失败走 overflow Case1 的收尾）后 须先于 threshold（ar31）评估 overflow Case1（domain-compaction c21–c23）。ContinueTools 不得做该预检。sameModel 且 is_context_overflow 时执行一次 compact-and-retry；willRetry 为 true 时 须从 store 重载与 as45 同源的 compaction-aware 工作 history（含摘掉错误 assistant 的效果，因 error/aborted 投影跳过或重载不含未裁切旧链）并继续本 run 的下一模型调用；不得仅 pop 错误行却继续握持 firstKept 之前的膨胀 history；二次 overflow 须失败并结束 recovery；overflow 不得走 AutoRetry 瞬态重试。该行为 须有可执行 BDD 场景（见 domain-compaction.feature 锚点）。
    场景: 溢出情形一压缩重试
      假如 sameModel 的 assistant 被判定为 context overflow 且 stop_reason 非 stop
      假如 compaction enabled 且尚未做过 overflow recovery
      当 执行 turn 后 overflow 检查
      那么 发生 compaction 且 CompactionStart reason 含 overflow

  @req:r1051
  规则: context-policy-responses-assembler
    agent 层 须提供 ContextPolicy（或等价）code-first 默认板：至少含 tools_mode（默认 full）、status_bar_mode（默认 off）；系统提示 不得含日历日/CWD（无 date_placement 旋钮，c2730 删消融）；不得本 change 实现 search/状态栏完整行为。openai-responses 主路径 须经 ResponsesAssembler（bridge）构造请求 body，并传入 WirePolicy；不得在 ReAct/adapter 散落第二套业务布局。由单测覆盖，不得单独扩 BDD step。
    场景: 代码首次上下文策略缺省
      假如 工具片段含 mcp 前缀工具
      当 以默认路径组装系统提示
      那么 系统提示不含 mcp 工具名且含 MCP 引导句

  @req:r1052
  规则: mcp-tool-table-freeze-gate
    轨 A：Agent/session 必须在首次 generate（会话尚未工具定稿）前执行会话准备门闸（对齐 infra-mcp mcp8）：等待准备事件 settle 或超时后定稿 provider 可见工具表；定稿后 须忽略会扩表的 settle 热并；idle `/reload` 与 resume/切会话 必须清冻并开火准备事件（unary 不得 wait），下一次 generate 才 wait 并按 name upsert 重定稿（指纹持久化后的一致续冻另波，对齐 mcp8）。tools_mode=search（c1960）不在本要求交付范围。由单测覆盖，不得单独扩 BDD step。
    场景: 首次生成冻结工具表
      当 以两组不同工具集的 fixture 配置先后装配
      那么 工具集随配置变化

  @req:r1053
  规则: stream-error-carries-kind
    ReAct / session 热路径在将 XyError 投影为 XyEvent::Error 时 须经 XyEventError::from_xy（或等价）保留稳定 kind（对齐 XyError::kind）；不得仅把 Display 字符串塞进无 kind 的 Error。Abort 路径 须使用 Aborted kind（或 is_aborted 可识别）。由单测覆盖，不得单独扩 BDD step。
    场景: 错误种类存活投影
      当 序列化携带 kind 的 error 事件
      那么 kind 往返保真

  @req:r1054
  规则: stream-node-timestamps
    ReAct 持久化 assistant 消息时 须附加 streamTiming（camelCase unix-ms 节点，LLM 投影忽略）：本 run 的 agentStarted、本 turn 的 turnStarted、思考通道起止、正文首末 TextDelta、首个工具意图、messageEnded；仅写入实际发生的键。思考通道结束 须在先到的正文/工具意图/ThinkingEnd 打一次，不得用 Done 或整段正文结束冒充。textEnded MAY 随最后 TextDelta 覆盖。Thought 展示用思考通道起止派生 thinkingElapsedSecs（不到 1s 省略）。由单测覆盖，不得单独扩 BDD step。
    场景: 已持久化节点计时结算思考
      当 以场景构建器回放思考加工具并结算 7 秒封轮挂载交互面
      当 左键单击折叠命中表中的簇头三角列
      那么 结算行外显 Thought 7s 且不再出现流式 Thinking 头

  @req:r2905
  规则: runtime-phase-transition-table
    RunCoordinator 的 RuntimePhase（Idle/Starting/Streaming/Cancelling）转移 须受守卫约束：仅 Idle 可 try_begin_immediate 入 Starting；mark_streaming 与 finish_run 仅对活动 run_id（或按 stale 语义相不变/降级）生效；cancel_active 自 Starting/Streaming 入 Cancelling；finish_run 匹配 run_id 后归 Idle。守卫失败相 须保持原相不变形。转移闭集 须以场景大纲固定并经状态流转校验（E1-E4）。

    场景大纲: 阶段转换矩阵
      假如 运行协调器处于 "<from>" 阶段
      当 协调器事件 "<event>" 且守卫 "<guard>"
      那么 协调器阶段为 "<to>"

      例子:
        | from      | event  | guard        | to        |
        | idle      | begin  | phase_idle   | starting  |
        | starting  | stream | run_id_match | streaming |
        | starting  | cancel | always       | cancelling |
        | streaming | cancel | always       | cancelling |
        | streaming | begin  | phase_busy   | streaming |
        | starting  | finish | run_id_match | idle      |
        | streaming | finish | run_id_match | idle      |
        | cancelling | finish | run_id_match | idle     |
        | streaming  | finish | run_id_stale | starting  |
        | starting   | finish | run_id_stale | starting  |

  @req:r2920
  规则: loop-middleware-spi
    插件 须经 rutis 生命周期注册 loop 中间件参与 ReAct 决策：阶段闭集为 before_turn、before_model、after_model、before_tool、after_tool、decide_continue、after_turn；注册表 须按阶段分组并按 order 升序排列，同 order 保持注册序；注册 须返回卸载句柄，插件停止时 须卸干净。每次 run 须在启动时快照中间件链，进行中 run 不得受后挂中间件影响；链为空时 须零开销直走基线路径，不得创建观测 span 或额外事件。core 须永远持有 turn 边界、取消、权限最终裁决与消息落盘格式，不得允许插件整段替换 ReAct。

    场景: 中间件有序且卸载后消失
      假如 注册 before_model 阶段 order 10 与 order 5 的两个中间件
      当 读取中间件链快照并注销 order 5 的中间件
      那么 首次快照按 order 升序且注销后不再含该 id

    场景: 空链走基线
      假如 未注册任何 loop 中间件
      当 运行 AgentRuntime 并收集事件
      那么 出现 AgentEnd 且无中间件错误事件

  @req:r2921
  规则: loop-middleware-control
    中间件返回值 须为 { ctx, control }；control 闭集为 continue、stop、compact、handoff。decide_continue 须仅在 Settle 的 TurnEnd 之后、settle 与压缩预检之后调用，ContinueTools 不得调用；stop 须以 AgentEnd 结束本 run 且不得再 drain steer 或 follow_up；compact 须由 core 执行一次强制压缩后继续；handoff 本 change 未支持，core 须以稳定 kind LoopMiddlewareHandoffUnsupported 的错误结束本 run，不得静默忽略。中间件不得自行再调模型，下一次 generate 只能由 core 调度。after_turn 的 control 须仅为 continue，其他取值须记入观测并忽略，不得静默改变行为。

    场景: 自定义停止结束运行
      假如 注册 decide_continue 中间件在首次结算返回 stop
      当 运行 AgentRuntime 并收集事件
      那么 出现 AgentEnd 且其后无新的 TurnStart

    场景: 未支持交接报错
      假如 注册 decide_continue 中间件返回 handoff
      当 运行 AgentRuntime 并收集事件
      那么 事件流含 kind 为 LoopMiddlewareHandoffUnsupported 的错误

    场景: after_turn 非法控制被忽略
      假如 注册 after_turn 中间件返回 stop 且入队 follow_up
      当 运行 AgentRuntime 并收集事件
      那么 after_turn 中间件已被调用且 run 继续到下一轮

  @req:r2922
  规则: loop-middleware-tool-gate
    before_tool 中间件 须在脚本 hooks 与 AgentHooks 之后、core permission 之前运行，且在工具批执行规划之前完成预演：中间件可改工具参数与工具名；core 须按最终工具名与最终参数重算权限目标并再裁一次，并按最终工具的执行分类重排并发窗，不得沿用原工具名或原参数的任何判定。中间件拒绝 须使工具以 tool-error 回写且不执行；中间件放行 不得撤销上游拒绝，也不得绕过 core permission。路由目标 须限于本次运行已装配的工具集，不得新增工具调用，也不得借路由绕过 MCP 前缀的串行纪律；找不到目标即 tool-error。审批只能更严；更松须显式 capability 与用户信任，不在本要求范围。预演 须先于本批任何工具执行完成；批内前序调用返回 stop 时，后续调用的中间件 已运行 但其工具 不得执行，中间件 不得假设「放行即执行」。

    场景: 中间件拒绝更严
      假如 注册 before_tool 中间件拒绝 bash
      当 运行 AgentRuntime 触发 bash
      那么 tool-error 回写且原因含 loop middleware

    场景: 改参不绕过权限
      假如 注册 before_tool 中间件把 bash 参数改为访问被拒域名且权限引擎拒绝该域名
      当 运行 AgentRuntime 触发 bash
      那么 tool-error 回写且原因含 permission denied

    场景: 改路由按新工具重裁
      假如 注册 before_tool 中间件把 read 路由到 write 且权限引擎拒绝该写入路径
      当 运行 AgentRuntime 触发 read
      那么 tool-error 回写且原因含 permission denied

    场景: 改路由按新分类重排
      假如 注册 before_tool 中间件把并行安全的 read 路由到串行工具
      当 运行 AgentRuntime 并收集工具批事件
      那么 该调用单独成窗且不与后续并行安全调用重叠

    场景: 改路由目标须已装配
      假如 注册 before_tool 中间件把 read 路由到未装配的工具名
      当 运行 AgentRuntime 触发 read
      那么 tool-error 回写且原因含 Unknown tool

    场景: 前序停止后批内中间件已运行
      假如 注册两个 before_tool 中间件且首个对首个调用返回 stop
      当 运行 AgentRuntime 触发两个工具调用
      那么 第二个中间件已被调用且其工具未执行

  @req:r2923
  规则: loop-middleware-bounded
    core 须对 decide_continue 的 continue 与 compact 设硬上限：连续裁决且本轮无新工具结果、无新用户消息的次数超过上限时 须停止本 run 并以稳定 kind LoopMiddlewareLimit 的错误可观测，不得挂死循环或静默停。单次中间件调用 须与取消令牌和超时竞速；取消须以 aborted 结束本 run，超时须以稳定 kind LoopMiddlewareTimeout 的错误结束本 run，不得让中间件无限期占用 turn。

    场景: 无进展继续触发上限
      假如 注册 decide_continue 中间件总是返回 continue 且无新工具与新用户消息
      当 运行 AgentRuntime 并收集事件
      那么 事件流含 kind 为 LoopMiddlewareLimit 的错误且随后出现 AgentEnd

  @req:r2924
  规则: loop-middleware-trace
    观测激活时，每次中间件调用 须留 loop.middleware span，属性至少含中间件 id、阶段、order、返回 control、改动摘要与结果；链为空时 不得建 span。由单测或观测场景覆盖。

    场景: 观测可指认改动来源
      假如 注册 before_model 中间件且在观测闸开启下运行一回合
      当 读取 loop.middleware span 属性
      那么 属性含该中间件 id、阶段名与改动摘要

  @req:r2925
  规则: loop-middleware-example-plugin
    仓库 须提供一个官方示例插件，证明第三方可在 rutis 生命周期内注册与卸载 loop 中间件：至少含一个只收紧的 before_tool 与一个自定义 decide_continue。该插件 须由配置显式启用，缺省不装配；未配置时 不得注册任何中间件、不得产生额外事件或观测 span。卸载后 注册表 须不再含该插件的中间件。

    场景: 未配置零装配
      假如 loop 示例插件配置缺省
      当 装配运行时并查询中间件注册表
      那么 注册表为空且运行无额外事件

    场景: 启用后卸载零残留
      假如 以启用 loop 示例插件装配运行时
      当 卸载该插件
      那么 注册表不再含该插件的中间件

    场景: todo 进度启用后注册 todo 进度中间件
      假如 以启用 todo 进度的 todo 插件装配运行时
      当 查询中间件注册表
      那么 注册表含 todo-progress

    场景: todo 提醒启用后注册 todo 提醒中间件
      假如 以启用 todo 提醒的 todo 插件装配运行时
      当 查询中间件注册表
      那么 注册表含 todo-reminder

    场景: 工具收窄启用后注册工具中间件
      假如 以启用工具收窄的 loop 示例插件装配运行时
      当 查询中间件注册表
      那么 注册表含 loop-example 与 tool-exposure

    场景: todo checkpoint 启用后注册 todo checkpoint 中间件
      假如 以启用 todo checkpoint 的 todo 插件装配运行时
      当 查询中间件注册表
      那么 注册表含 todo-checkpoint

    场景: 自愈启用后注册自愈中间件
      假如 以启用自愈的 loop 示例插件装配运行时
      当 查询中间件注册表
      那么 注册表含 loop-example 与 self-heal

    场景: 生成评审启用后注册评审中间件
      假如 以启用生成评审的 loop 示例插件装配运行时
      当 查询中间件注册表
      那么 注册表含 loop-example 与 generator-critic

    场景: workflow 启用后注册 workflow 中间件
      假如 以启用 workflow 的 workflow 插件装配运行时
      当 查询中间件注册表
      那么 注册表含 workflow

  @req:r2955
  规则: plan-mode-middleware
    计划模式中间件 须在 before_model 读取最新 `plan_mode`；模式为 plan 时 须注入一次计划契约（先研究/阅读、用 `plan_write` 产出计划、用 `plan_submit` 请求批准）；模式为 execute 时 不得注入。会话 id 缺失或存储不可用时 须静默降级，不得中断 run。中间件 不得改写工具表、不得调用模型或工具；工具只读限制 不在本要求范围。

  @req:r2929
  规则: loop-middleware-structured-result
    after_tool 中间件 须在脚本 hooks 与 AgentHooks 之后、core quiet 摘要之前接收结构化结果：载荷含原始内容分片、可解析的结构化详情与错误标志，不得只提供拍平文本。中间件改写后的内容与详情 须成为本调用的历史回写与 ToolExecutionEnd 结果；core 的 write/edit quiet 摘要 仅在结果仍为单段文本时生效，且不得丢弃中间件给出的结构化详情。工具产出的非文本分片 须在历史回写中保真，不得被拍平为占位文本。

    场景: 结构化分片保真
      假如 注册 after_tool 中间件且工具返回图片分片
      当 运行 AgentRuntime 触发该工具
      那么 历史回写仍含图片分片

    场景: 改写详情生效
      假如 注册 after_tool 中间件把结构化详情替换为带标记的对象
      当 运行 AgentRuntime 触发该工具
      那么 该调用的历史回写详情含该标记

  @req:r2930
  规则: loop-middleware-compact-request
    decide_continue 返回 compact 时，中间件 须能随决策提供压缩指令与来源标签；core 须把来源标签带入压缩开始与结束事件的 reason，缺省时 须以中间件标识构造稳定标签，不得一律记为手动压缩。压缩成功后 须向本 run 事件流发出压缩后结算，供插件与端读取压缩后占用；压缩失败 须以带中间件标识的错误结束本 run。压缩仍只由 core 调度，中间件不得自行调用压缩或改写压缩策略。

    场景: 压缩来源可辨识
      假如 注册 decide_continue 中间件返回带来源标签的 compact 决策
      当 运行 AgentRuntime 并收集压缩事件
      那么 CompactionStart 与 CompactionEnd 的 reason 含该标签

    场景: 压缩后结算可见
      假如 注册 decide_continue 中间件返回 compact 决策且压缩成功
      当 运行 AgentRuntime 并收集事件
      那么 事件流含压缩后结算

  @req:r2926
  规则: loop-middleware-visible-consistency
    中间件改动 须与对外可见形状一致：before_turn 注入的系统碎片与 before_model 改写的 system / tool schema 须回流到本 run 的压缩固定上下文（token 估计与压缩阈值按最终形状计算，不得沿用启动快照）；before_tool 改写的参数须在 ToolExecutionStart 中呈现，不得先发原始参数再执行改写后参数；after_model 改写的 assistant（含新增 tool_call）须在 MessageEnd 之前生效，不得让已发出的 MessageEnd 与随后落盘/执行的形状不一致。中间件超时或取消结束本 run 时，回合观测须标记为非正常结束，不得记为成功。

    场景: 改写参数可见
      假如 注册 before_tool 中间件把 bash 命令改为 echo safe
      当 运行 AgentRuntime 触发 bash 并收集事件
      那么 ToolExecutionStart 的参数为 echo safe

    场景: 改写助手先于消息结束
      假如 注册 after_model 中间件把 assistant 文本改写为 replaced
      当 运行 AgentRuntime 并收集事件
      那么 MessageEnd 携带的 assistant 文本为 replaced 且无改写前文本

    场景: 系统碎片进入压缩上下文
      假如 注册 before_turn 中间件注入长系统碎片
      当 运行 AgentRuntime 并比较有无碎片的回合结算估计
      那么 含碎片的估计不低于无碎片基线

  @req:r2927
  规则: run-event-boundary
    一次用户触发的 run 的事件流 须以 AgentStart 开始、以 AgentEnd 结束：AgentStart 须在本 run 的会话与历史装配完成、回合观测建立之后、首个 TurnStart 之前发出，且为流内第一个事件；AgentEnd 须为最后一个事件。AgentStart 的 session_id 须为本 run 绑定的会话，model 须取本 run 启动时的当前模型标识，流内不得重读以免与回合观测属性不一致；未选模型时 model 为空串。装配失败（会话或历史加载失败）时 不得发 AgentStart。

    场景: 首末事件边界
      假如 mock 模型先 tool 后无 tool
      当 运行 AgentRuntime 并收集事件
      那么 首个事件为 AgentStart 且末个事件为 AgentEnd

    场景: 启动字段口径
      假如 mock 模型先 tool 后无 tool
      当 运行 AgentRuntime 并收集事件
      那么 AgentStart 的 session_id 为本会话且 model 为当前模型标识

  @req:r2931
  规则: loop-middleware-session-scope
    中间件链 须支持会话级作用域：Host 每个会话槽 须持有自 root 派生的独立链，槽内注册与注销 不得影响 root 或其他会话；派生 须冻结 root 当时的链快照，root 后续注册的中间件 不得流入已存在的会话槽。未配置第三方可注册中间件的 root 链 须保持为空（loop 内置项不计入链，也不产生 span），且空链 不得产生额外事件或观测 span。会话作用域 须为纯数据，槽丢弃即释放，不得需要显式卸载调用，也不得泄漏回 root。嵌入方 须能在不接触 rutis 类型的前提下为自建驱动挂注册表。

    场景: 会话链互不影响
      假如 root 链含一个共享中间件且两个会话槽各自再注册一个独有中间件
      当 比较两个会话槽与 root 的中间件标识
      那么 两会话互不可见对方独有项且 root 不含任何独有项

    场景: 派生冻结父快照
      假如 会话槽已从 root 派生
      当 root 随后注册新中间件
      那么 该会话槽的链仍为派生时刻快照

    场景: 未配置保持空链
      假如 root 未注册任何中间件
      当 读取派生会话槽的链
      那么 链为空

  @req:r2933
  规则: loop-builtin-skills-expand
    Skills 展开 须作为 loop 内置项执行，不得进入可注册中间件链：内置展开 须在 before_model 阶段之前完成，使任何第三方 before_model 中间件 都能观察到展开后的 messages；内置项 不得占用 order 位、不得产生 loop.middleware span、不得计入「空链」判定。第三方中间件 不得替换或跳过内置展开，只允许在其后叠加。未装配任何 skills 时 展开 须为无操作，messages 须逐字节不变。未注册第三方中间件时 装配结果 须与内置化之前逐字节一致。

    场景: 中间件看到展开结果
      假如 注册 before_model 中间件观察 messages 且 skills 已装配
      当 运行 AgentRuntime 提交引用技能的 prompt
      那么 before_model 观察到已展开的技能正文

  @req:r2937
  规则: message-persist-before-end
    ReAct 的每一条消息（用户 pending、助手正常完成、助手中断、助手中间件 stop）须先写入会话存储再下行 MessageEnd，使客户端收到完成事件时 存储已可读到该条；不得先发 MessageEnd 再落库。落库失败 不得阻断 MessageEnd 下行（沿用既有 warn 语义），但 不得因此把顺序颠倒回去。
  @req:r2939
  规则: todo-reminder
    todo 提醒中间件 须在 before_model 读取当前 todo（经 BeforeModelCtx.todo）；当 todo 非空且仍有未完成项，并满足以下任一条件时，向本回合模型上下文追加一条短提醒：连续 N 次工具调用未更新 todo、最后一条 assistant 宣称完成、上一次工具失败。同一 turn 至多提醒一次。提醒 须只读：不得自动改写 todo、不得产生新事件、不得调用模型或工具；未满足条件或 todo 为空时 不得追加任何内容。阈值与开关 须可配置。提醒行为 由中间件单测覆盖。
  @req:r2940
  规则: tool-exposure
    工具收窄中间件 须在 before_model 改写 tool_schemas：按配置的 allow / deny 与只读开关收窄模型可见工具；`todo_rewrite` 与 `todo_update` 须始终可见，避免计划无法更新。收窄 只影响模型可见性：不得删除工具、不得改变 permission 终裁、不得影响 before_tool 的路由目标校验。未启用或规则为空时 不得改变 schema 形状。过滤前后数量与理由 须可观测。step/risk 的细粒度映射 不在本要求范围，后续 change 再评估。
  @req:r2942
  规则: todo-checkpoint-middleware
    todo checkpoint 中间件 须在 after_tool 记录 todo 状态变化与工具失败，并在 before_model（或 after_turn）把变化落为 todo_checkpoint 条目；恢复后的首个 before_model 须注入一次短摘要（已完成/总数、当前步骤、上次失败）。注入 须只读：不得自动重跑已完成步骤、不得改写 todo、不得调用模型或工具；无 checkpoint 或 todo 已完成时 不得注入。同一会话至多注入一次。会话 id 缺失或存储不可用时 须降级为仅内存，不得中断 run。写入 须只追加，不覆盖或删除历史 checkpoint。
  @req:r2943
  规则: self-heal
    自愈中间件 须只在 after_tool 识别到确定性失败签名时介入：命中签名 须记录 pending（工具名、签名、尝试次数）并在结果后追加短指引，要求诊断、修复后重跑同一验证；同一工具成功 须清除 pending。非确定性失败（超时、网络、权限等）不得进入自愈。decide_continue 须在 pending 且尝试次数小于配置上限时 Continue，达到上限时 Stop；无 pending 时 不得改变基线。上限 须远小于 core 的 32 次硬上限。不得在插件内调用模型或工具，不得自动回滚。自愈与 todo-progress 可并存，Stop 仍由最先返回 Stop 的中间件决定。
  @req:r2944
  规则: generator-critic
    生成评审中间件 须在 after_model 只对**无 tool_calls 的 assistant 最终答案**启动评审，且评审调用仍由 core 调度（插件不得自行调用模型）。状态机须为 Idle → Critiquing →（Approved | Revising）→ … → Approved | Exhausted；Critiquing 时 before_model 注入 rubric 与 verdict 格式，Revising 时注入 critic 意见。critic 须以单独一行 `CRITIC_VERDICT: APPROVE` 或 `CRITIC_VERDICT: REVISE` 结尾；无结构化 verdict 时最多再要求一次，超过一次须停止。decide_continue 在 Critiquing / Revising 时 Continue，在 Approved / Exhausted 时 Stop，其它状态不得改变基线。评审轮数 须受 `max_rounds` 约束且远小于 core 的 32；达到上限须停止并记录原因。不得允许 critic 触发 compact，不得自动改代码或提交。

  @req:r2962
  规则: multiple-before-tool-gates
    多个 before_tool 中间件 须按 order 依次执行；任一返回 stopped 须终止后续闸与工具执行；首个 stopped 的原因 须成为最终结果。由单测覆盖。
