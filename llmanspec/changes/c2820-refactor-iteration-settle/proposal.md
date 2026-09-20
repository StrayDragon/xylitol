---
depends_on: []
---

# 把 ReAct iteration 结束与 user-run 收尾拆开

> **一句话**：`finish_turn` 不再同时承担「这一轮模型+工具结束」和「可以压缩 / 可以按 max_turns 停」。工具续跑只关闭 iteration；settlement、auto-compact 预检、`should_stop_after_turn` 只在不再续跑工具时发生。

## Why

Langfuse `agent.turn` `16e32cfcfd214923c11d3b99daad193d` 一次用户发言、两次 `llm.request`，两次 `agent.compaction.skipped`，`skip_reason` 相同。不是导出重复，是 **同一种收尾函数被调用了两次**。

今日 `finish_turn` 打包了四件本应分层的事：

1. 与 `TurnStart` 配对的 `TurnEnd`（iteration 界）
2. `ContextTokenSettlement`（footer / 预检尺子）
3. `try_turn_end_compaction`（threshold / overflow 预检 → 早退就 `agent.compaction.skipped`）
4. `should_stop_after_turn`（`session.max_turns`）

工具窗口结束后无条件走这整包，再 `continue_after_tools` 进入下一轮 generate。观测上像压缩被点了两次；产品上 `max_turns=1` 会在 **第一批工具之后** 结束 run，续跑 generate 不会发生。

观测栈已经有名字：`agent.turn` = 一次用户触发的 run；`agent.iteration` = 一轮 generate（及其工具）。缺的是代码与 `XyEvent` 收尾跟这棵树对齐，而不是在 `react/mod.rs` 里加一个 `if continue { continue }`。

## What Changes

- 在 `turn_end` 缝定义穷举的 iteration 关闭结果：`ContinueTools` vs `Settle`（名字以代码为准）。禁止第三种「悄悄少导出一个 skipped」。
- **ContinueTools**（本轮有 tool_calls、将再 generate）：关闭本 iteration（`TurnEnd` 与 `TurnStart` 配对）；MUST NOT settlement、MUST NOT auto-compact 预检、MUST NOT `should_stop_after_turn`。
- **Settle**（本轮不再要工具）：今日 `finish_turn` 的完整收尾（settlement + 预检 + should_stop + 队列）。
- overflow 在 generate **失败** 后的 Case1 仍走 Settle 预检，不并进 ContinueTools。
- live specs 写清三层词：user-triggered run / iteration / settle；改 `agent-runtime` 的 max_turns 与 `domain-compaction` 的「turn 收尾预检」主语，避免再把它们写成「每次 TurnEnd」。
- `infra-otel` 的 skipped span 形状不改；次数随真预检次数下降。

## Capabilities

- `agent-runtime`
- `domain-compaction`
- `infra-otel`（次数，非形状）

## Impact

- Langfuse：一条用户发言里工具续跑不再刷同因 skipped。
- `max_turns`：只在 Settle 的 TurnEnd 之后计数（工具续跑不再吃额度、不再停 run）。
- TUI：工具续跑仍有成对 TurnStart/TurnEnd（中间 TurnEnd 仍不得当整轮 idle，既有 bridge 护栏）。

## Out of scope

- 压缩算法 / keep window / skipped 文案
- AgentStatusBar 注入形状（c2805 已归档）
- Assembler 布局观测（c1935）
