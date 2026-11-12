# Design — turn 模型复核收尾（全部已决 2026-11-12）

## 0. 基线

spec 真值 = agent-runtime r3322（one-submission-one-turn）；实现以 b63532b 为基线。词汇三层（session/turn/step；run 已退役为标识符）。

## 1. C-1 steer 双路径路由

~~~mermaid
flowchart TB
  S["steer(text) 调用"] --> IS{"turn 在途？"}
  IS -->|是| SQ["steer 队列（既有 A 语义：<br/>Settle drain → decide_continue 注入当前 turn）"]
  IS -->|否（settle 后 / 空闲 / turn 间隙）| FQ["follow_up FIFO<br/>（视为一次提交 → 弹出成新 turn）"]
~~~

- 判据 = **turn 是否在途**（非流是否活跃——turn 间隙到达也必须成新 turn；若只看流活跃，间隙 steer 会滞留 steer 队列到下一 turn 的 Settle，语义错位）。
- 实现建议：AgentRuntime/AgentCapabilities 暴露 turn-in-flight 标志（内层 turn 循环置位、Settle/AgentEnd 清位），steer() 按标志路由。
- 测试改写：steer_queued_after_settle_joins_next_submission（mod.rs:2530）改名并断言**新语义**——settle 后 steer 经 FIFO 成为独立 turn（边界序列含独立 AgentStart/AgentEnd），不与用户消息合并。

## 2. abort 对齐 r3322

- 前：abort → break 'outer → AgentEnd(UserCancel) → 流结束，FIFO 挂起到下次提交。
- 后：abort 取消当前 turn、清 steer；**FIFO 非空 → 立即弹下一条开新 turn**（流不结束）；FIFO 空 → 流结束。
- 依据：r3322「abort 须取消当前 turn、清空 steer 并保留 follow_up FIFO。队列非空时须立即弹出下一条（新 turn）」通读；用户改向意图针对当前 turn，排队提交仍应投递。

## 3. 三处补测

1. steer 在途注入（runtime 级）：turn 进行中（工具续跑间）调 steer → 断言注入**当前 turn** 的后续 step，边界序列无额外 AgentStart。
2. abort 后弹出：入队 2 条 follow-up → 流内 abort → 断言当前 turn 取消（AgentEnd UserCancel）后**立即** AgentStart 开新 turn 处理第 1 条，第 2 条随后。
3. fork 祖先链（c3266 T11 漏项）：注入 context_reminder 后 fork 于用户消息 entry → 断言提醒行仍在 fork 分支祖先链（经 store 加载验证）。

## 4. app-tui-input 场景措辞

「run 不再活跃」→「turn 不再活跃」；「run 收到 second」→「turn 收到 second」；**必须同步** tests/bdd 对应步骤绑定（步骤文本精确匹配）并跑该场景 BDD。

## 5. 测试边界与风险

- 全部改动使实现符合已落地 spec，无 spec 变更、无 breaking。
- 风险：turn-in-flight 标志的置位/清位点要覆盖 panic/unwind 路径（对齐 RunInflightGuard 纪律）。
