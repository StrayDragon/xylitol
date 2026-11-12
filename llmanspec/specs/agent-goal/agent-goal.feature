# language: zh-CN
# purpose: 会话级目标：goal 事实、每轮 evaluator、逐 turn 续跑与 loop 交互。

功能: agent-goal

  @req:r3299
  规则: goal-facts
    System 须将 GoalDefined / GoalStateChanged / GoalEvaluated 以 `custom_type = "automation"` 的 CustomEntry 追加到会话分支；goal 状态 须由事实投影重建，不得覆盖历史；事实 须按 branch_id 过滤，fork 子分支 不得继承父 goal；同一 branch 同时最多一个 active goal；未知 goal fact 须忽略并告警，不得阻塞旧 session 加载。

  @req:r3300
  规则: goal-evaluator
    GoalEvaluator 须在每轮 settle 后使用当前 session model 独立判定，不带工具；输入 须含 acceptance、transcript 与 evidence；acceptance 上限 4000 字符；verdict 闭集为 met / not_met / impossible / invalid；met 无 evidence 须按 not_met；连续 3 次调用失败或非法 须转为 blocked；max_iterations 缺省 256，iterations_used 只计 GoalEvaluated。

  @req:r3301
  规则: goal-decide-continue
    settle 阶段 须读取当前 branch 的 active goal；无 goal 时保持原行为；met + evidence 须追加 GoalEvaluated(met) 与 GoalStateChanged(done) 并 Stop；not_met 须追加 GoalEvaluated(not_met) 并 Continue，达到 max_iterations 须 blocked + Stop；not_met 的 reason 须作为下一条提交的用户消息（下一轮指导）；该指导 须以 `<system-reminder>` 开闭标签整包包裹（与 context_reminder 同约定），投影给模型 须保留标签；端侧 须按系统消息呈现，不得伪装为真人输入样式；impossible 须 failed + Stop；pause / cancel 后 不得再调用 evaluator，pending 结果须丢弃；goal 续跑 须逐 turn 续跑（evaluator 在 turn 收尾投递下一条提交，新 turn 独立 AgentStart/AgentEnd），不得在同一 turn 内续跑或新增第二 admission。

  @req:r3302
  规则: goal-loop-interaction
    active goal 存在时，同 branch 的 loop tick 须记 RunSkipped(goal_active) 且不得启动 turn；goal done 后 c3207 engine 须追加 ScheduleStateChanged(paused, reason=goal_done)，不得删除 schedule 事实；goal 续跑与 loop tick 不得并行驱动同一 session。

  @req:r3318
  规则: goal-state-transitions
    goal 状态写入 必须 经合法转移表：Open→{Running, Paused, Cancelled}；Running→{Done, Failed, Blocked, Paused, Cancelled}；Blocked→{Running, Done, Failed, Paused, Cancelled}；Paused→{Running, Cancelled}；Done / Failed / Cancelled 为终态 必须 锁定。非法转移 必须 拒绝并回 LLM 可见错误；存量事实流 必须 按同表容忍投影（保留最后合法态）。由单测覆盖。
