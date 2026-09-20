# Design: iteration 关闭 vs settle

## 已有分层（不要再发明第四层）

观测（`agent/runtime/obs.rs`）已经是：

```text
agent.turn          ← 一次用户触发的 run（AgentStart…AgentEnd）
  ├─ agent.iteration  ← 一轮 generate + 这一轮的 tool.execute
  │    ├─ llm.request
  │    └─ tool.execute
  └─ agent.compaction / agent.compaction.skipped
```

`XyEvent::TurnStart` 在 **每轮 generate 之前** 就发出（`react/mod.rs` 里 `stream_clock.begin_turn`）。所以 iteration 在产品事件上已经有「开始」。缺的是结束时不要把 settle 绑死在「有没有刚跑完工具」。

本地计数器名叫 `turn`，实际是 **iteration index**。本 change 不强制改字段名，但 specs 与 `turn_end` API 必须停止把这个计数器叫「用户回合」。

## 为什么不能 `if continue_after_tools { continue }`

那会留下 **TurnStart 无 TurnEnd**：工具续跑再进下一轮时会再发一个 TurnStart。TUI / hook / `turn_index` 都按成对边界活。少一次 `finish_turn` 是补丁，不是边界。

也不要只在 FileReporter / Langfuse 侧把相同 `skip_reason` 去重——预检仍然跑了两遍。

## 穷举关闭

`turn_end.rs` 已是收尾缝。把今日的 `finish_turn` 拆成调用方能穷举的结果，而不是在 `react/mod.rs` 中部再塞一个布尔。

| 关闭 | 何时 | TurnEnd | Settlement | compact 预检 | should_stop |
|---|---|---|---|---|---|
| **ContinueTools** | 本轮 `tool_calls` 非空，将再 generate | 要（配成对） | 不要 | 不要 | 不要 |
| **Settle** | 本轮不再要工具 | 要 | 要 | 要 | 要 |
| generate 失败 overflow | 既有错误路径 | 走 Settle 预检（Case1） | 同今日 | 同今日 | 同今日 |

`AgentIterationSpan` 继续靠 drop 关 span，不必为 ContinueTools 另做 obs 名。

## max_turns

`should_stop_after_turn` 只挂在 **Settle** 之后。`session.max_turns=1` 允许「一轮 generate + 任意次工具续跑 + 再 generate」，直到某轮不再要工具才停。这与「用户可见的一次说完」对齐，也与 `agent.turn` 根 span 对齐。

BDD `max_turns=2` + follow_up 仍成立：follow_up 是下一次 Settle 之后的新 iteration 序列，不是工具续跑。

## 测试缝

复用 ReAct 假模型 + CollectingReporter（otel27 已有 skipped 形状测）：

1. user → tool → 再 generate 无 tool：`TurnEnd` 两次（成对），`agent.compaction.skipped` **一次**，`token.estimate` settlement **一次**。
2. `max_turns=1`：第一轮带 tool 时 MUST 续跑第二轮 generate，MUST NOT AgentEnd。
3. 无工具的单轮：行为与今日 Settle 相同（回归）。

不把「少一个 skipped」写成 FileReporter 断言；断言调用次数在编排缝。

## 明确不做

- 不新增 `XyEvent` 变体
- 不把 compaction.skipped 改成 iteration 子 span 来「看起来少一次」
- 不在本票改 AgentStatusBar / assembler
