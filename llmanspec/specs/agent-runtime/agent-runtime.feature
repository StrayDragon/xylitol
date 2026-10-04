# language: zh-CN
# capability: agent-runtime
# purpose: "薄编排 ReAct 运行时：AgentRuntime 循环、工具批执行、steer/follow-up 队列、abort 与 XyEvent 流（不含 adk / OutputGuard）。"
# scope: src/agent/, src/protocol/, tests/

功能: agent-runtime

  @req:r1027
  规则: thin-react-loop
    agent/ 层 MUST 以 AgentRuntime 驱动 ReAct：调用 XyModel、执行工具、直至无 tool_calls、abort、或可选 should_stop_after_turn 请求停；MUST NOT 以产品默认 max_iterations 或未配置的硬步数闸结束 run；可选 session.max_turns（见 ar30）MUST 仅通过 should_stop_after_turn 生效。MUST NOT 在循环实现内联 bash 执行或厂商 HTTP。

    场景: react-terminates
      假如 mock 模型先 tool 后无 tool
      当 运行 AgentRuntime
      那么 先执行工具再结束且无 adk 类型
  @req:r1036
  规则: xy-event-stream
    运行时 MUST 以 XyEvent 异步流对外输出（TextDelta / ThinkingDelta / Tool* / Turn* / QueueUpdate / Error 等）；MUST NOT 再引入 AgentEvent 或 adk 事件类型。

    场景: stream-is-xyevent
      假如 消费事件流
      当 轮询
      那么 每项为 XyEvent
  @req:r1047
  规则: 工具批后继续
    同一模型响应中的全部工具调用 MUST 在下一轮模型调用前执行完毕（可并行或串行）；MUST NOT 因流结束 Done 而提前结束含 tool_calls 的回合。该续跑 MUST 关闭本 iteration（与 TurnStart 成对的 TurnEnd），MUST NOT 因此做 ContextTokenSettlement、auto-compact 预检或 should_stop_after_turn。该行为 MUST 有可执行 BDD 场景（live `.feature`，`@req`）。

    场景: continues-after-tools
      假如 mock 模型先 tool 后无 tool
      当 运行 AgentRuntime
      那么 turn_end 事件包含 toolResult
  @req:r1038
  规则: tool-intent-before-execution
    ReAct 消费 XyChunk::ToolCallStart/Delta/End 时：流式阶段 MUST 经 MessageUpdate（message 含渐进 ToolCall 部分）暴露工具意图；MUST NOT 在流未结束（MessageEnd 前）发射 ToolExecutionStart 或执行工具副作用。ToolExecutionStart/Update/End MUST 仅在 MessageEnd 之后的执行路径发出。该行为 MUST 有可执行 BDD 或等价单测场景。

    场景: intent-before-execution
      假如 mock 模型先 tool 后无 tool
      当 运行 AgentRuntime 并收集事件
      那么 MessageUpdate 含工具意图且早于任意 ToolExecutionStart；ToolExecutionStart 不早于 MessageEnd
  @req:r1039
  规则: system-via-generate-options
    配置的 system prompt MUST 经 XyGenerateOptions.system_prompt 传给模型适配层；MUST NOT 将 system 正文作为 AgentMessage::user 写入会话 history（空会话首条 MUST 为真实用户输入）。由 ReAct 单测覆盖，MUST NOT 为静态存在性单独扩 BDD step。

    场景: first-turn-history-is-real-user
      假如 配置了 mock 模型 "test-model"
      当 运行一轮对话后检查持久化条目
      那么 首条消息条目为真实 user 输入且无 system 角色行
  @req:r1055
  规则: builder-ports
    Agent 装配 MUST 经 AgentBuilder，依赖 protocol 端口与 agent 会话词汇；app 组合根注入具体 infra 实现。装配入口由单元测试 / 组合根覆盖，MUST NOT 为静态存在性单独扩 BDD step。
    # verified-by: tests/support/mod.rs
    场景: builder-ports-drive-one-round
      假如 mock 模型先 tool 后无 tool
      当 运行 AgentRuntime 并收集事件
      那么 先执行工具再结束且无 adk 类型

  @req:r1056
  规则: sessionstore-eventsink
    protocol MUST 提供 SessionStore 与 EventSink；SessionManager / EventBus 为实现。端口存在性由类型系统与组合根覆盖，MUST NOT 为 trait 存在性单独扩 BDD step。
    # verified-by: src/AGENTS.md
    场景: ports-yield-xy-events
      假如 消费事件流
      当 轮询
      那么 每项为 XyEvent

  @req:r1057
  规则: mutable-slots-next-turn
    set_tools / set_hooks / set_system_prompt 等 MUST 只影响下一轮 run；进行中回合使用启动快照。
    # verified-by: src/agent/runtime/react/tests.rs
    场景: hot-applied-slots-rebuild-prompt
      假如 Agent 能力聚合体已就绪
      当 热应用新的上下文与系统提示资源
      那么 系统提示按新资源重建

  @req:r1058
  规则: hooks-at-tool-boundary
    工具执行前 MUST 跑 before 链（可拒绝）；之后 MUST 跑 after 链；无 hook 时零开销。

    场景: before-denies
      假如 注册匹配 bash 的 before 拒绝 hook
      当 运行 AgentRuntime 触发 bash
      那么 tool-error 回写且未执行
  @req:r1059
  规则: pending-queues
    MUST 提供独立的 steer 与 follow_up 队列（enqueue/drain/clear）；缺省 QueueMode / SteeringMode MUST 为 OneAtATime（一次 drain 一条；配置可显式 all）；ReAct MUST 在模型调用前 drain steer；在将因无 tool_calls 而结束前 drain follow_up（非空则继续）；若 should_stop_after_turn 已请求停，MUST NOT drain steer 或 follow_up。

    场景: steer-before-model
      假如 装配并运行入队 steer 的 agent
      当 检查队列与历史
      那么 steer 计数归零且已处理

    场景: followup-extends
      假如 装配无工具 agent 并入队 follow_up
      当 运行至将结束
      那么 继续循环而非 AgentEnd

    场景: should-stop-skips-followup
      假如 入队 follow_up 且 should_stop_after_turn 在首次 TurnEnd 后返回 true
      当 运行 AgentRuntime
      那么 本 run 以 AgentEnd 结束且 follow_up 未被注入历史
  @req:r1060
  规则: queue-update-events
    入队/drain/clear 后 MUST 发射 XyEvent::QueueUpdate（steer_count / follow_up_count）。

    场景: queue-update
      假如 装配 agent 并入队 steer
      当 观察事件流
      那么 出现 QueueUpdate 且计数正确
  @req:r1028
  规则: abort-queue-semantics
    abort MUST 清空 steer、保留 follow_up；MUST 取消当前 run CancellationToken 与进行中工具/交互 bash；MUST NOT 仅靠无人订阅的 EventBus 旁路传队列状态。

    场景: abort-clears-steer
      假如 装配 agent 并入队 steer 与 follow_up
      当 abort
      那么 steer 空且 follow_up 保留

    场景: abort-cancels-bang
      假如 启动交互 bang 长命令后 abort
      当 检查 bash 结果
      那么 cancelled 为 true
  @req:r1029
  规则: abort-resets-token
    每次 run 开始 MUST 安装新 CancellationToken；abort 后下一次 run MUST NOT 因沿用已取消 token 立刻 aborted。

    场景: second-run-after-abort
      假如 装配慢速 agent 并在首轮 abort 后
      当 再次运行
      那么 正常完成而非立即 aborted
  @req:r1030
  规则: abort-cancels-provider-stream
    模型流进行中 abort MUST 停止 poll 并 drop provider stream，以 aborted 结束；应用面 MUST 经 Driver::abort。该行为 MUST 有可执行 BDD 场景（live `.feature`，`@req`），并复用既有 abort step 词表。

    场景: abort-drops-sse
      假如 配置了 mock 模型 test-model 且慢速流式 40 段间隔 20 毫秒
      当 经 Driver 启动会话并在首个 TextDelta 后 abort
      那么 事件流包含 aborted 错误
  @req:r1031
  规则: unknown-event-degrade
    应用面/bridge 对未识别 XyEvent 变体 MUST 降级（日志），MUST NOT panic。由 protocol 单元测试覆盖。
    # verified-by: src/protocol/lifecycle.rs
    场景: unknown-queue-type-degrades
      当 对 QueueUpdate 事件做线协议序列化与反序列化往返
      那么 队列计数保真且未知 type 解析为错误而非 panic

  @req:r1032
  规则: defaults
    未显式配置时 MUST 有可测默认：thinking level、compaction 频率等集中定义；MUST NOT 将 max_iterations 或未配置的硬步数闸列为运行时默认。由单元测试覆盖。
    # verified-by: src/agent/compaction/settings.rs
    场景: centralised-default-thinking-level
      假如 Settings.default_thinking_level 为 high 且模型支持集为 off 与 high 与 max
      当 会话首次装配
      那么 当前 thinking level 为 high

  @req:r1035
  规则: Driver context 重载缝
    InProcessDriver（经 app/core 助手）MUST 能在 Trust 语义下重载磁盘 context/SYSTEM/APPEND 并应用到 agent；未信任 MUST NOT 注入项目侧 context；重载 MUST NOT 清空 transcript。

    场景: context-reload-trust-gated
      当 在项目目录写 AGENTS.md 并分别以信任与未信任重载 context
      那么 信任时项目 context 注入而未信任时被跳过且会话条目不变
  @req:r1037
  规则: Driver skills 重载缝
    InProcessDriver（经 app/core 助手）MUST 能在 Trust 语义下重载磁盘 skills 并应用到 agent；MUST 能查询当前已加载 skill 名（供后续 $ 展开）；未信任 MUST NOT 注入项目 skills；重载 MUST NOT 清空 transcript；本要求 MUST NOT 依赖 /session 或 /status skills UI。

    场景: skills-reload-trust-gated-and-queryable
      当 在项目目录写 skill 并分别以信任与未信任重载 skills
      那么 信任时项目 skill 注入且可查询已加载名而未信任时被跳过
  @req:r1040
  规则: persist-done-usage
    ReAct 在模型流正常结束时 MUST 将 XyChunk::Done 携带的 usage 与 stop_reason（若有）写入随后持久化的 AssistantMessage；MUST NOT 在 Done 已提供非空 usage 时仍硬编码 usage: None。该行为 MUST 有可执行 BDD 或等价单测场景。
    # verified-by: src/agent/runtime/react/tests.rs
    场景: done-usage-anchor-persisted
      假如 会话条目含 stop 回合的 usage 锚点
      当 以同源估计器估计上下文
      那么 估计来源为 Api

  @req:r1041
  规则: should-stop-after-turn
    ReAct MUST 在 Settle（本轮模型调用不再续跑工具）的 TurnEnd 之后、轮询 steer/follow-up 或开始下一模型调用之前，调用可选的单槽 should_stop_after_turn（对齐 pi shouldStopAfterTurn）；返回 true 时 MUST 发射 AgentEnd 并结束本 run，MUST NOT 为此新增专用停闸 XyEvent 变体，MUST NOT abort 本 turn 已完成的助手消息或工具。工具续跑的 iteration TurnEnd（ContinueTools）MUST NOT 调用该钩子。未注册时 MUST 不因步数上限停止。该行为 MUST 有可执行 BDD 场景（live `.feature`，`@req`）。

    场景: should-stop-emits-agent-end
      假如 注册 should_stop_after_turn 在首次 TurnEnd 后返回 true
      当 运行 AgentRuntime
      那么 出现 AgentEnd 且其后无新的模型轮 TurnStart

    场景: no-hook-open-end
      假如 未注册 should_stop_after_turn 的无工具 agent
      当 运行 AgentRuntime
      那么 正常出现 AgentEnd 且恰好一轮 TurnStart
  @req:r1042
  规则: next-turn-refresh-model-thinking
    ReAct MUST 在每次模型调用前（turn 边界，对齐 pi prepareNextTurn）从 session 重读当前选中模型与 thinking level，并据此重建本 turn 的 XyModel 与 XyGenerateOptions（或等价绑定）；MUST NOT 在整个 run 内握死首帧 model/thinking snapshot。已开始的 in-flight generate_stream MUST 继续使用该次调用开始时的绑定，MUST NOT 中途拆流。run 结束（AgentEnd）或 abort 后无 in-flight 时，active 与 selected MUST 收敛。该行为 MUST 有可执行 BDD 或等价单测场景。
    # verified-by: src/agent/runtime/react/tests.rs
    场景: next-turn-rereads-model-and-thinking
      当 以桥缝注入元数据事件（模型选择与 thinking 档位）
      那么 模型不 panic 且条目与相位不变

  @req:r1043
  规则: abort-persist-skip-llm
    用户 abort 中断模型流时，ReAct MUST 将已累积的 partial assistant（非空 content）以 stop_reason=aborted 持久化进 session history；project_for_llm（或等价 LLM 投影）MUST 跳过 stop_reason 为 aborted 或 error 的 assistant 行，MUST NOT 将其作为下一轮 provider 输入。空 content 的 aborted MAY 不落盘。该行为 MUST 有单测或 BDD 覆盖。

    场景: abort-persists-partial-with-aborted-reason
      假如 配置了 mock 模型 test-model 且慢速流式 40 段间隔 20 毫秒
      当 经 Driver 启动会话并在首个 TextDelta 后 abort 并检查持久化条目
      那么 partial assistant 以 aborted 落盘且非空
  @req:r1044
  规则: tool-batch-default-barrier-parallel
    未显式配置工具批模式时，ReAct MUST 对同一 MessageEnd 后的 tool call 批采用 barrier_parallel（连续 ParallelSafe 扇出；Barrier 先汇聚再串行）调度；MUST NOT 默认退回整批串行（除非显式 mode=sequential）。该行为 MUST 有可执行 BDD 场景（live `.feature`，`@req`）。

    场景: batch-default-barrier-parallel
      假如 未配置工具批模式且 mock 模型同 turn 发出两个可并行假工具
      当 运行 AgentRuntime
      那么 两工具执行时间重叠
  @req:r1045
  规则: tool-batch-barrier-parallel
    当工具批模式为 barrier_parallel（含产品缺省）时，ReAct MUST 按 assistant 源序将连续 ParallelSafe 工具收入并行窗扇出执行，遇 Barrier 工具（写/副作用/未知/一切 mcp_ 前缀工具等）MUST 先汇聚（await 齐）当前窗再串行执行该屏障工具，然后继续；MUST NOT 把屏障之后的 ParallelSafe 提前并入屏障之前的并行窗；MUST NOT 在 MessageEnd 前执行工具（ar21）；MUST NOT 经配置/glob/annotation 将 mcp_ 工具升为 ParallelSafe。该行为 MUST 有可执行 BDD 或等价可控假工具单测/BDD 场景。

    场景: batch-barrier-parallel-overlap
      假如 工具批模式为 barrier_parallel 且 mock 同 turn 发出两个 ParallelSafe 慢假工具后接一个 Barrier 假工具
      当 运行 AgentRuntime
      那么 两 ParallelSafe 执行时间重叠且均在 Barrier 开始前结束

    场景: batch-barrier-preserves-source-windows
      假如 工具批模式为 barrier_parallel 且 mock 同 turn 工具序为 ParallelSafe、Barrier、ParallelSafe
      当 运行 AgentRuntime
      那么 第二个 ParallelSafe MUST NOT 与第一个 ParallelSafe 同窗并行且 MUST 在 Barrier 完成之后开始

    场景: batch-mcp-never-parallel
      假如 工具批模式为 barrier_parallel 且 mock 同 turn 工具序为 ParallelSafe、mcp 假工具、ParallelSafe
      当 运行 AgentRuntime
      那么 mcp 假工具与两侧 ParallelSafe 均无执行时间重叠
  @req:r1046
  规则: tool-batch-history-source-order
    无论 sequential 或 barrier_parallel，写入 session history 与发往模型的 toolResult MUST 严格按 assistant 源序；ToolExecutionEnd（及 Update）MAY 按完成序交错。该行为 MUST 有可执行 BDD 或等价单测场景。

    场景: batch-history-source-order
      假如 工具批模式为 barrier_parallel 且并行窗内后发先完成
      当 检查 session history 中 toolResult
      那么 toolResult 顺序与 assistant 源序一致
  @req:r1048
  规则: config-max-turns-via-should-stop
    当运行时配置 session.max_turns（正整数 N）存在时，组合根 MUST 在装配后安装 should_stop_after_turn：于 Settle 的 TurnEnd 后若 settle 次数 >= N 则返回 true 结束 run；缺省或未配置时 MUST NOT 安装该步数钩子（开放结束，见 ar24）。MUST NOT 把工具续跑的 iteration TurnEnd 计入额度，MUST NOT 使用 max_iterations 字段名。该行为 MUST 有可执行 BDD 或等价单测场景。

    场景: max-turns-stops-run
      假如 按 session.max_turns=2 安装 should_stop_after_turn 且入队 follow_up 以迫使第二轮
      当 运行 AgentRuntime
      那么 至多出现 2 次 TurnStart 后出现 AgentEnd
  @req:r1843
  规则: iteration-close-vs-settle
    一次用户触发的 run（观测根 agent.turn，AgentStart…AgentEnd）内，每一轮模型 generate 及其工具批是一次 iteration（观测 agent.iteration，XyEvent TurnStart/TurnEnd 成对）。本轮仍有 tool_calls、将再 generate 时 MUST 以 ContinueTools 关闭 iteration：发 TurnEnd 配成对，MUST NOT ContextTokenSettlement、MUST NOT auto-compact 预检、MUST NOT should_stop_after_turn。本轮不再要工具时 MUST Settle：TurnEnd + settlement + threshold/overflow 预检 + should_stop。generate 失败走 overflow Case1 的收尾仍为 Settle。MUST NOT 用「跳过 TurnEnd」或「导出侧去重 skipped」代替这组穷举。由单测覆盖，MUST NOT 为静态存在性单独扩 BDD step。

    场景: tool-turn-two-iterations-one-settlement
      假如 mock 模型先 tool 后无 tool
      当 运行 AgentRuntime 并收集事件
      那么 TurnStart 与 TurnEnd 成对出现两次且 ContextTokenSettlement 恰一次
  @req:r1049
  规则: turn-end-threshold-compaction
    ReAct 或 session 编排在 Settle（本轮不再续跑工具、非 abort）后 MUST 调用 threshold auto-compact 检查（domain-compaction c2 地板感知有效触发阈值 + c17/c18）；ContinueTools、CompactionSettings.enabled 为 false、未超有效阈值、abort、或 stale 守卫命中时 MUST NOT compact；MUST NOT 仅依赖 TUI host 轮询触发。该行为 MUST 有可执行 BDD 或等价单测场景。
    # verified-by: llmanspec/specs/domain-compaction/domain-compaction.feature
    场景: turn-end-threshold-auto-compact
      假如 会话叶上存在可信 Api usage 锚点且 footer 同源估计可用
      当 执行 auto-compact reserve 触发判断
      那么 所用 token 数字与同源估计一致且 MUST NOT 另算独立 len/4 总和
      并且 触发比较式为占用大于有效触发阈值 max(window 减 reserveTokens, 压后地板 加 迟滞带)

  @req:r1050
  规则: turn-end-overflow-compact-retry
    ReAct 在 Settle（或 generate 失败走 overflow Case1 的收尾）后 MUST 先于 threshold（ar31）评估 overflow Case1（domain-compaction c21–c23）。ContinueTools MUST NOT 做该预检。sameModel 且 is_context_overflow 时执行一次 compact-and-retry；willRetry 为 true 时 MUST 从 store 重载与 as45 同源的 compaction-aware 工作 history（含摘掉错误 assistant 的效果，因 error/aborted 投影跳过或重载不含未裁切旧链）并继续本 run 的下一模型调用；MUST NOT 仅 pop 错误行却继续握持 firstKept 之前的膨胀 history；二次 overflow MUST 失败并结束 recovery；overflow MUST NOT 走 AutoRetry 瞬态重试。该行为 MUST 有可执行 BDD 场景（见 domain-compaction.feature 锚点）。
    # verified-by: llmanspec/specs/domain-compaction/domain-compaction.feature
    场景: overflow-case1-compact-retry
      假如 sameModel 的 assistant 被判定为 context overflow 且 stop_reason 非 stop
      假如 compaction enabled 且尚未做过 overflow recovery
      当 执行 turn 后 overflow 检查
      那么 发生 compaction 且 CompactionStart reason 含 overflow

  @req:r1051
  规则: context-policy-responses-assembler
    agent 层 MUST 提供 ContextPolicy（或等价）code-first 默认板：至少含 tools_mode（默认 full）、status_bar_mode（默认 off）；系统提示 MUST NOT 含日历日/CWD（无 date_placement 旋钮，c2730 删消融）；MUST NOT 本 change 实现 search/状态栏完整行为。openai-responses 主路径 MUST 经 ResponsesAssembler（bridge）构造请求 body，并传入 WirePolicy；MUST NOT 在 ReAct/adapter 散落第二套业务布局。由单测覆盖，MUST NOT 单独扩 BDD step。
    # verified-by: src/agent/prompt/mod.rs
    场景: code-first-context-policy-defaults
      假如 工具片段含 mcp 前缀工具
      当 以默认路径组装系统提示
      那么 系统提示不含 mcp 工具名且含 MCP 引导句

  @req:r1052
  规则: mcp-tool-table-freeze-gate
    轨 A：Agent/session MUST 在首次 generate（会话尚未工具定稿）前执行 MCP 门闸（对齐 infra-mcp mcp8）：等待 settle 或超时后定稿 provider 可见工具表；定稿后 MUST 忽略会扩表的 settle 热并；idle `/reload` MUST 按 name upsert 重定稿；本波 resume/切会话 MUST 清冻再门闸（指纹持久化后的一致续冻另波，对齐 mcp8）。tools_mode=search（c1960）不在本要求交付范围。由单测覆盖，MUST NOT 单独扩 BDD step。
    # verified-by: src/agent/tools/freeze.rs
    场景: first-generate-freezes-tool-table
      当 以两组不同工具集的 fixture 配置先后装配
      那么 工具集随配置变化

  @req:r1053
  规则: stream-error-carries-kind
    ReAct / session 热路径在将 XyError 投影为 XyEvent::Error 时 MUST 经 XyEventError::from_xy（或等价）保留稳定 kind（对齐 XyError::kind）；MUST NOT 仅把 Display 字符串塞进无 kind 的 Error。Abort 路径 MUST 使用 Aborted kind（或 is_aborted 可识别）。由单测覆盖，MUST NOT 单独扩 BDD step。
    # verified-by: llmanspec/specs/protocol-app/protocol-app.feature
    场景: error-kind-survives-projection
      当 序列化携带 kind 的 error 事件
      那么 kind 往返保真

  @req:r1054
  规则: stream-node-timestamps
    ReAct 持久化 assistant 消息时 MUST 附加 streamTiming（camelCase unix-ms 节点，LLM 投影忽略）：本 run 的 agentStarted、本 turn 的 turnStarted、思考通道起止、正文首末 TextDelta、首个工具意图、messageEnded；仅写入实际发生的键。思考通道结束 MUST 在先到的正文/工具意图/ThinkingEnd 打一次，MUST NOT 用 Done 或整段正文结束冒充。textEnded MAY 随最后 TextDelta 覆盖。Thought 展示用思考通道起止派生 thinkingElapsedSecs（不到 1s 省略）。由单测覆盖，MUST NOT 单独扩 BDD step。
    场景: persisted-node-timings-settle-thought
      当 以场景构建器回放思考加工具并结算 7 秒封轮挂载交互面
      当 左键单击折叠命中表中的簇头三角列
      那么 结算行外显 Thought 7s 且不再出现流式 Thinking 头


# re-review(c2827): 复审结论——本 capability 管辖行为不变；分支内改动为 BDD 场景落地、BDD 测试基建（steps/bindings/驱动旋钮与探针）与可见性再导出（2026-09-28）
    # verified-by: src/agent/runtime/react/tests.rs
# re-review(c2835): 复审结论——本 capability 管辖行为不变；改动为批量 barrier 的测试判据与协议载体终态形状，ReAct 状态机与批语义未动。（2026-09-29）
