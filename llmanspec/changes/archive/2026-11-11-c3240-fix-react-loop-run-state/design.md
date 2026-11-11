# Design — ReAct 内环运行状态一致性（含词汇定稿与协议改名）

## 0. 词汇定稿（三层共享概念 + 宿主实现术语）

共享概念词汇（全仓统一，文档叙事主语）：

| 层 | 定义 | 事件 / 代码载体 |
|---|---|---|
| session | 会话 | 会话存储 |
| turn（问答轮） | 一条用户输入（或 follow-up 续问）到最终答复的完整往返 | 由若干 step 构成；末端发生 Settle |
| step | 一次模型调用周期 + 工具批 | StepStart / StepEnd（原 TurnStart / TurnEnd）、step_index |

宿主实现术语（不进共享概念词汇；指称提交边界用「一次提交」或具体事件名）：run = 一次提交、一个事件流（RunCoordinator 相位 / AgentStart … AgentEnd / run_start / run_end hook）——保留为实现命名，文档不得把它当概念层使用。

反向提示（下游 gate 引用）：文档若仍把 run 当概念层（如「run 边界」「同 run」），是旧稿——对照本节三层版修订。

- Settle 不是「一种 turn」，是 turn 的收尾动作（发生在末个 step：token 结算 → 可能压缩 → 查 steer / follow-up）。
- ContextTokenSettlement 的 reason「turn_settled」在新词汇下恰好正确，不改。

## 1. 目标与边界

- 修复三个已核实缺陷：虚假 AutoRetryEnd、max_turns 计数跨提交累积（随机制整体移除）、bang 取消槽覆盖。
- AbortReason 进事件流（AgentEnd 可选字段），全栈可见。
- 词汇与协议名一步到位对齐，不留别名（仓库改名规则：一次性改全部调用点）。
- 边界：不改工具批执行与 loop 中间件协议；不动 steer / follow_up 队列语义（abort-queue-semantics 合约不变）。

## 2. 决策摘要（全部已决 2026-11-11）

| 决策 | 内容 |
|---|---|
| AutoRetry | 每 step 边界重建 RetryState；End 仅真实重试后发射 |
| max_turns | 机制整体移除，替换为 session.max_steps_per_turn 经 should_stop_after_step 生效 |
| AbortReason | 进 XyEvent：AgentEnd.reason 可选字段 + wire / TUI / print 全栈透出 |
| bang | 并发忙拒（单飞） |
| nested | RAII guard |
| 协议改名 | TurnStart/TurnEnd → StepStart/StepEnd，一次性，全链同步 |
| 深挖降级 | 「Cancelling 且 cancel=None 可达」未找到构造路径，降级未确认；保留的真实问题 = 转移分散 + mark_streaming 覆写 Cancelling 竞态 |

## 3. max_steps_per_turn 设计

- 为什么不是修 settle 计数：settle 计数防不了不 settle 的 runaway——永不停止要工具的循环根本不触发 Settle。step 上限才同时覆盖 runaway 与成本。
- 配置：session.max_steps_per_turn（正整数，未配置不启用；旧键 session.max_turns 移除，遇到即报错并指路新键，不留别名）。
- 机制：单槽 should_stop_after_step 钩子，每次 step 收尾调用（含 ContinueTools）；ctx = { step_index, is_settle, assistant, tool_results, history, new_messages }；kernel 装配接线（替代 bootstrap 的 max_turns_stop_hook）。
- 触发语义：达到上限 → 本 step 正常收尾（不 abort 已完成消息，对齐 r1027 精神）→ AgentEnd(reason=StepLimit)。
- 有意放弃：按提交计的问答轮上限（原 settle 计数的正确版）——若未来需要 follow-up 预算另立 change。

## 4. AbortReason 全栈

- 内部枚举 AbortReason { UserCancel, ModelError, MiddlewareStop, MiddlewareFault, ModalityUnsupported, StepLimit }；turn_aborted 十六处写点改 abort(reason)，首个非 OK 归因胜出。
- 事件：XyEvent::AgentEnd { messages, reason: Option<AgentEndReason> }（None = 自然完成；serde 可选字段，不新增事件变体——与 r1027「不得新增专用停闸变体」条款兼容，spec delta 注明）。
- 链路：plugin-sdk lifecycle → wire 双向映射 → TUI bridge handler（通知走 app-tui-fixed-zone 固定词）→ print 一行 → BDD 各归因场景。

## 5. RetryState 与其余修复

- RetryState 随每次模型调用（attempt 循环所在作用域）重建；AutoRetryEnd 的两个发射点（成功 :1242 / 失败 :1279）加「本 step 真实重试过」守卫；spec 新增 AutoRetry 语义条款。
- RunCoordinator：transition(event) 单入口（Begin / StreamStarted / Cancel / Finish）；mark_streaming 增加「当前相位为 Cancelling 则不覆写」守卫。
- bang：cancel 槽占用时 execute 返回 busy 错误；nested 计数五处手工 fetch_sub 改 RAII guard。

## 6. 改名清单（实测 rg 规模，约 30 文件）

- 词表层：plugin-sdk lifecycle（XyEvent 变体）、loop_middleware（10 处，含 **LoopStage::BeforeTurn / AfterTurn → BeforeStep / AfterStep** 与 BeforeTurnCtx / AfterTurnCtx 类型更名——阶段在 step 边界开火，命名随词汇定稿对齐）、protocol event（6 处）、wire event（10 处双向映射 + type 字符串）。
- runtime：react/mod.rs（20）、obs.rs（19，TurnEndReason → StepEndReason）、turn_end.rs（模块改名 step_end.rs）、hooks / middleware / script_hook_ctx / tool_exec / dispatch。
- 插件：hooks lib（10，含脚本 hook 事件名 turn_start / turn_end → step_start / step_end，及 agent_start / agent_end / agent_settled → run_start / run_end / run_settled——宿主实现术语收口（run 不进共享概念词汇），r1000 同步改写）、workflow / loop-policies / todo / mcp truncate / codemode middleware。
- BDD：steps_agent_runtime（20）、steps_server（10）、steps_loop_middleware（8）、steps_protocol、steps_app_tui_bridge 等。
- specs：agent-runtime（r1027 改写、ar185 重写为 step 钩子、r1036 Turn* → Step*）、runtime-config（max_turns 键条款）、workspace-assembly、cli-print、protocol-app、app-tui-bridge。
- 术语落点：根 / src AGENTS.md 增三层词汇定义（session / turn 问答轮 / step）与 run 宿主实现术语注记；仓库无独立 docs/ 目录。

## 7. 破坏性变更（三项，release note 声明）

1. 协议事件名 TurnStart / TurnEnd → StepStart / StepEnd（旧端不识别新名）。
2. 脚本 hook 事件名 turn_start / turn_end → step_start / step_end；agent_start / agent_end / agent_settled → run_start / run_end / run_settled（用户 hook 配置需改）。
3. 配置键 session.max_turns → session.max_steps_per_turn（旧键报错指路）。

### Release note 草稿（三项 breaking）

```text
BREAKING:
- 协议事件 TurnStart / TurnEnd 更名为 StepStart / StepEnd；旧端不识别新名。
- 脚本 hook 事件名 turn_start / turn_end → step_start / step_end；agent_start / agent_end / agent_settled → run_start / run_end / run_settled；用户 hook 配置需同步改名。
- 配置键 session.max_turns → session.max_steps_per_turn（旧键加载失败并指路新键）；语义由「Settle 次数」改为「每 turn 的 step 上限」，未配置不启用。
```

## 8. 测试边界（seam）

- BDD：agent-runtime（AutoRetry 语义、StepLimit 停止、各 AbortReason 端可见）、agent-hooks（step_start / step_end 分发）、cli-print。
- 单测：transition 矩阵；两 step 重试场景（第 1 step 重试、第 2 step 顺利，断言无 AutoRetryEnd）；bang 并发忙拒；nested guard Drop。
- 改名回归：rg 'TurnStart|TurnEnd|turn_index|max_turns|should_stop_after_turn' 全仓归零（排除 changes/archive 与本 change）。

## 9. 风险与回滚

- 风险：改名清扫面大（约 30 文件）——机械替换为主，BDD 兜底；三项 breaking 均在未推广期。
- 回滚：改名清扫 / max_steps 机制 / AbortReason / 缺陷修复四片独立可 revert（改名片宜原子合入不拆）。
