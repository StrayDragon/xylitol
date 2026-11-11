# Design — 中间件生命周期复位与自动化转移（全部已决 2026-11-11）

## 0. 上游术语 gate

假定 c3240 已合入（三层词汇 session/turn/step——run 为宿主实现术语；LoopStage::BeforeStep、BeforeStepCtx.has_new_user_messages——缺失则 rg 'has_new_user_messages' 并查 c3240 T2）；假定 c3250 已合入（admit 单一入口——缺失则查 c3250 T6）。勿自行造词或重复实现。反向提示：本文档若仍把 run 当概念层，对照 c3240 §0 三层版修订。

## 1. critic 复位（turn 边界）

~~~mermaid
stateDiagram-v2
  [*] --> Idle
  Idle --> Critiquing: 回复无 tool_call
  Critiquing --> Approved: APPROVE
  Critiquing --> Revising: REVISE / 解析失败(1次)
  Critiquing --> Exhausted: 连续 2 次解析失败
  Revising --> Critiquing: rounds < max
  Revising --> Exhausted: rounds >= max
  Approved --> Idle: turn 边界复位（修复核心）
  Exhausted --> Idle: turn 边界复位（修复核心）
~~~

- 复位谓词 = BeforeStep && ctx.has_new_user_messages（新问答轮，含 follow-up）；rounds / unparsed / instruction_injected 单一 reset 入口同步清零。

## 2. goal / schedule 合法转移表

| from \ to | Open | Running | Done | Failed | Blocked | Paused | Cancelled |
|---|---|---|---|---|---|---|---|
| Open | — | ✓ | ✗ | ✗ | ✗ | ✓ | ✓ |
| Running | ✗ | — | ✓ | ✓ | ✓ | ✓ | ✓ |
| Done / Failed | ✗ | ✗ | — | ✗ | ✗ | ✗ | ✗（终态锁定） |
| Blocked | ✗ | ✓(解除) | ✓(判定) | ✓ | — | ✓ | ✓ |
| Paused | ✗ | ✓ | ✗ | ✗ | ✗ | — | ✓ |
| Cancelled | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | —（终态锁定） |

- 校验位置：goal_set_state / goal_pause / goal_resume / goal_clear / set_state 工具入口 + runtime decide_active_goal 内部转移。
- 存量投影：历史事实流按同表投影，非法序列保留最后合法态 + warn（不改写历史）。
- Exhausted 事实化：ticks 耗尽时由 tick 引擎写 ScheduleStateChanged(Exhausted)，派生与持久合一。

## 3. 其余修复

- todo 闸门：先判 session_id 再 swap；restore 失败回滚 restored=false 供下轮重试。
- todo reminder：stale_tool_calls / last_error / last_reminder_turn 三原子合并进单 Mutex 快照（一致读取）；进度解析改用 SDK TodoStatus（消灭字符串词汇分叉）。

## 4. spec delta（landing 时）

agent-goal / agent-automation 增转移合法性条款（终态锁定、Exhausted 事实化）；agent-runtime 增中间件 turn 边界复位条款；agent-todo 增闸门可重试条款。

## 5. 测试边界（seam）

单测：转移表全矩阵；critic 两问场景（第二问不被 stop，含 follow-up 续问变体）；todo 无 session 首调后恢复能力。BDD：agent-goal / agent-automation 既有场景回归 + 非法转移拒绝。

## 6. 风险与回滚

- 风险：终态锁定与存量事实冲突——容忍投影兜底。
- 回滚：critic 复位 / 闸门 / 转移表 / reminder 四片独立。

## 7. 场景走查附录（新实现者必读）

场景 A（critic 秒停；主语 = turn）：turn 1 APPROVE → Approved → stop（正常）；turn 2（独立提交或 follow-up 续问两种形态）→ phase 仍 Approved → decide_continue 立即 stop，模型从未被调用。修复边 = BeforeStep && has_new_user_messages——修复只认「新 turn」，对两种形态一视同仁，这正是选 turn 边界而非提交边界的原因。

场景 B（todo 闸门）：swap(true) 先于 session_id 判定——首调无 session 则 restored 永久 true、恢复永不发生。修复 = 先判后置 + 失败回滚重试。

场景 C（goal 僵尸复活）：LLM 对 Cancelled goal 调 set_state(Running)——现状仅查存在性，直接追加事实。修复 = 转移矩阵拒绝（终态锁定），存量按表容忍投影。

上游交付物对照：BeforeStepCtx.has_new_user_messages ← c3240 T2；admit 单一入口 ← c3250 T6（本 change T4 之前必须完成）。

样例对照（测试夹具骨架）：

~~~text
critic 秒停（前 → 后；主语 = turn）：
  turn 1 "实现 X"：APPROVE → Approved → stop（正常收束）
  turn 2 "再实现 Y"（形态 A：独立提交；形态 B：忙时排队的 follow-up 续问）
    前: decide_continue 见 Approved → 立即 stop，0 个 TextDelta（模型未被调用）
    后: BeforeStep && has_new_user_messages → reset(Approved→Idle, rounds=0)
        → 正常答复 + 新评审（两种形态一视同仁——修复只认「新 turn」，不问提交形态）

todo 闸门（代码形状）：
  前: !swap(true) && session_id.is_some()   ← swap 先烧，None 时标志已毁
  后: 先判 !restored && session_id → 置位 → restore 失败回滚 false 可重试

goal 僵尸复活：
  事实流 …cancelled → 模型 set_state(running)
    前: 直接追加（复活）；投影无感知
    后: 工具报「非法转移 cancelled→running（终态锁定）」；存量投影保留最后合法态 + warn

schedule Exhausted：
  前: 投影层推导（持久仍 active，两套真值）
  后: tick 耗尽写 schedule_state_changed(exhausted)

reminder / 进度：
  前: 三原子两次读之间翻转拼中间态；{"status":"in-progres"} 静默归 pending
  后: 单 Mutex 快照一次读全；SDK TodoStatus 枚举，未知值显式报错
~~~
