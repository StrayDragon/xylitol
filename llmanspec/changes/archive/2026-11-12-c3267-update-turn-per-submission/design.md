# Design — 提交即 turn（全部已决 2026-11-12）

## 0. 词汇终态

~~~text
session ⊃ turn（= 一次提交 = 一对 AgentStart/AgentEnd）⊃ step（模型调用周期）
run：已退役边界的历史实现命名——仅存于标识符（RunCoordinator / RunId / run_inflight），
    语义 = 单个 turn 的执行与串行化机器；概念层与文档叙事零使用（反向提示同步更新）。
~~~

## 1. 模型与新循环流

~~~mermaid
flowchart TB
  Q["待提交 FIFO（follow-up 队列重塑）"] -->|settle 后弹一条| T2
  subgraph turn["turn = 一次提交"]
    A["AgentStart"] --> S1["step 1 … step n<br/>（steer 在途：step 边界注入当前 turn）"] --> E["AgentEnd"]
  end
  T2 --> A
  E -->|"队列空 → idle"| IDLE["空闲"]
  E -->|"队列非空 → 立即弹下一条"| Q
~~~

- follow-up：drain_queue(all) → **pop_front 一条**；每条独立 turn。
- steer（A 语义）：turn 在途 → 经既有 decide_continue 通路在下一 step 边界注入当前 turn；settle 后到达 → 无在途工作可改 → 作为一次投递进 FIFO（新 turn）。
- abort：取消当前 turn；清空 steer；FIFO 保留（下一条照常弹出）——语义重述，机制沿用。

## 2. 走查样例（前后对照）

~~~text
用户在 turn 1 进行中连发 3 条 follow-up（f1 f2 f3）+ 1 条 steer：

现状（batch）：turn1 settle → drain [f1 f2 f3] 合并进同一个 turn2 的同一次模型调用
             （一个 AgentStart..AgentEnd 内含两个 turn，边界模糊）
新模型：     turn1 settle → AgentEnd → 弹 f1 → AgentStart → turn2 …
             → AgentEnd → 弹 f2 → AgentStart → turn3 … → AgentEnd → f3 → turn4
             （每条独立回合、独立事件对；steer 若在 turn2 在途时到达 → 注入 turn2）
~~~

## 3. 更名与跟名清单

- hook 事件字符串：run_start / run_end / run_settled → turn_start / turn_end / turn_settled（runtime script_hook_ctx、hooks 插件、全部消费端与 BDD；不留别名）。
- XyEvent::AgentStart / AgentEnd 变体名**不动**（若改名 AgentStart→TurnStart 会与刚从 TurnStart 改名的 StepStart 重建混淆线，_wire/protocol 稳定优先）。
- c3248 注入点：run_start / run_end → turn_start / turn_end（枚举值、配置校验、编译表、spec r3312/3313 措辞）；turn_pre 判定简化（每 turn 以用户输入开始，has_new_user_messages 每 turn 恰一次真）。
- 实现标识符不动：RunCoordinator / RunId / run_inflight / RunInflightGuard / run_command_capture（AGENTS 词汇段改写其描述）。

## 4. TUI 连续性

AgentEnd 粒度变细的风险：follow-up 接续时 AgentEnd→AgentStart 间隙可能让 busy 闪断。对策：QueueUpdate 已带队列深度——端侧在 AgentEnd 时若 steer/follow-up 深度非零则保持 busy（bridge/host 联动一处判定 + 回归测试）。

## 5. automation / goal 跟进

- 调度 tick（c3207）：admit 后投递 = 一次提交 = 一个 turn；coalesce 语义不变（重复到期仍并一条）。
- goal 续跑（c3208）：evaluator 在 turn 收尾决定续跑 = 投递下一条提交（新 turn）；「同 run 续跑」措辞全仓改「逐 turn 续跑」。

## 6. spec delta（landing 时）

- agent-runtime：新条款「提交即 turn」（每 turn 一对 AgentStart/AgentEnd；FIFO 逐条；abort 清 steer 留 FIFO）；abort-queue-semantics 重写；既有条款概念层 run 残留五处+ 改写（「结束 run」→「结束 turn」等）。
- agent-hooks r1000：事件名更名。
- agent-prompt（c3248 落地条款）：注入点跟名。
- agent-goal / agent-automation：续跑措辞。
- app-tui-bridge / app-tui-host：busy 连续性行为条款。
- 根 / src AGENTS.md：词汇段改写（run 退役描述 + 反向提示更新）。

## 7. 测试边界（seam）

- 单测：FIFO 逐条（3 条 follow-up → 3 对 AgentStart/End，顺序断言）；steer 在途注入当前 turn / settle 后入 FIFO 两路径；abort 后 FIFO 弹出下一条；队列非空 busy 不闪断（端侧）。
- BDD：agent-runtime 提交即 turn 场景；agent-hooks turn_start/turn_end 分发；c3248 注入点跟名回归。
- 终检：rg 概念层 run 残留归零（标识符白名单外）；hook 事件名旧串归零。

## 8. 风险与回滚

- 风险：事件流消费者对 AgentEnd 频率升高敏感（TUI / print 已列连续性任务）；steer 双路径（在途注入 / settle 入队）需测试覆盖两分支。
- 回滚：循环改造 / 更名 / TUI 连续性三片独立（更名宜原子合入）。
